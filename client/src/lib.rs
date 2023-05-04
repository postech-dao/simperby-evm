use async_trait::async_trait;
use dotenvy_macro::{self, dotenv};
use ethers::signers::Signer;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder};
use ethers::types::{H160, U256};
use ethers::{contract::abigen, middleware::SignerMiddleware, types::Address};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::{BlockId, BlockNumber, Bytes};
use ethers_providers::{Http, Middleware, Provider};
use eyre::Error;
use hex;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_core::*;
use simperby_settlement::execution::convert_transaction_to_execution;
use simperby_settlement::*;
use std::str::FromStr;
use std::sync::Arc;

const EVM_COMPATIBLE_ADDRESS_BYTES: usize = 20;

abigen!(
    ITreasury,
    r#"[
        function name() external view returns (string memory)
        function chainName() external view returns (bytes memory)
        function contractSequence() external view returns (uint128)
        function lightClient() external view returns (uint64 heightOffset, bytes memory lastHeader)
        function viewCommitRoots() external view returns (bytes32[] memory commitRoots)
        function updateLightClient(bytes memory header, bytes memory proof) public
        function execute(bytes memory transaction,bytes memory executionHash, uint64 blockHeight, bytes memory merkleProof) public
    ]"#,
);

abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function totalSupply() public view returns (uint256)
        function transfer(address _to, uint256 _value) public returns (bool success)
        function transferFrom(address _from, address _to, uint256 _value) public returns (bool success)
    ]"#,
);

abigen!(
    IERC721,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function tokenOfOwnerByIndex(address owner, uint256 index) external view returns (uint256)
    ]"#,
);

pub struct ChainConfigs {
    /// The RPC URL of the chain
    rpc_url: String,
    /// The name of the chain
    chain_name: Option<String>,
}

pub enum ChainType {
    Ethereum(ChainConfigs),
    Goerli(ChainConfigs),
    Other(ChainConfigs),
}

impl ChainType {
    fn get_rpc_url(&self) -> &str {
        match self {
            ChainType::Ethereum(chain) => chain.rpc_url.as_str(),
            ChainType::Goerli(chain) => chain.rpc_url.as_str(),
            ChainType::Other(chain) => chain.rpc_url.as_str(),
        }
    }

    fn get_chain_name(&self) -> &str {
        match self {
            ChainType::Ethereum(_) => "Ethereum",
            ChainType::Goerli(_) => "Goerli",
            ChainType::Other(configs) => {
                if configs.chain_name.is_some() {
                    configs.chain_name.as_ref().unwrap().as_str()
                } else {
                    "Unknown"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvmCompatibleAddress {
    pub address: Address,
}

impl EvmCompatibleAddress {
    pub fn to_hex_str(&self) -> String {
        format!("0x{}", hex::encode(&self.address.as_fixed_bytes()))
    }

    pub fn to_hex_str_without_prefix(&self) -> String {
        hex::encode(&self.address.as_fixed_bytes())
    }

    pub fn to_hex_serialized_vec(&self) -> HexSerializedVec {
        HexSerializedVec {
            data: self.address.as_bytes().to_vec(),
        }
    }

    pub fn from_hex_str(address: &str) -> Result<EvmCompatibleAddress, Error> {
        let address = if address.len() == 2 * EVM_COMPATIBLE_ADDRESS_BYTES + 2 {
            if !address.starts_with("0x") {
                return Err(eyre::eyre!(
                    "Invalid address format: missing 0x prefix or invalid length({})",
                    address.len()
                ));
            }
            address[2..].to_string()
        } else if address.len() == 2 * EVM_COMPATIBLE_ADDRESS_BYTES {
            address.to_string()
        } else {
            return Err(eyre::eyre!(
                "Invalid address format: invalid length({})",
                address.len()
            ));
        };
        address
            .parse::<Address>()
            .map_err(|e| eyre::eyre!("Invalid address format: {}", e))
            .map(|address| EvmCompatibleAddress { address })
    }

    pub fn from_hex_serialized_vec(
        address: &HexSerializedVec,
    ) -> Result<EvmCompatibleAddress, Error> {
        let address = if address.data.len() == EVM_COMPATIBLE_ADDRESS_BYTES {
            hex::encode(&address.data)
        } else {
            return Err(eyre::eyre!("Invalid address format: invalid length"));
        };
        address
            .parse::<Address>()
            .map_err(|e| eyre::eyre!("Invalid address format : {}", e))
            .map(|address| EvmCompatibleAddress { address })
    }
}

pub struct EvmCompatibleChain {
    pub chain: ChainType,
    pub treasury_address: Option<EvmCompatibleAddress>,
}

#[async_trait]
impl SettlementChain for EvmCompatibleChain {
    async fn get_chain_name(&self) -> String {
        self.chain.get_chain_name().to_string()
    }

    async fn check_connection(&self) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let block_number = provider.get_block_number().await;
        if block_number.is_err() {
            return Err(eyre::eyre!(format!(
                "Failed to connect to chain {}",
                self.chain.get_chain_name()
            )));
        }
        Ok(())
    }

    async fn get_last_block(&self) -> Result<SettlementChainBlock, Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let block = provider
            .get_block_with_txs(BlockId::Number(BlockNumber::Latest))
            .await?;
        if let Some(block) = block {
            let height = block.number.unwrap().as_u64();
            let timestamp = block.timestamp.as_u64();
            return Ok(SettlementChainBlock { height, timestamp });
        } else {
            return Err(eyre::eyre!(format!(
                "Failed to get last block from chain {}",
                self.chain.get_chain_name()
            )));
        }
    }

    async fn get_contract_sequence(&self) -> Result<u128, Error> {
        let treasury = if let Some(address) = &self.treasury_address {
            address
        } else {
            return Err(eyre::eyre!("Treasury address is not set"));
        };
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let contract = ITreasury::new(treasury.address, Arc::new(provider));
        let contract_sequence = contract.contract_sequence().call().await?;
        Ok(contract_sequence)
    }

    async fn get_relayer_account_info(&self) -> Result<(HexSerializedVec, Decimal), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let relayer_address: H160 = wallet.address();
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let balance = provider
            .get_balance(relayer_address, None)
            .await?
            .to_string();
        let address = HexSerializedVec::from(relayer_address.as_bytes().to_vec());
        Ok((
            address,
            Decimal::from_str(balance.as_str()).map_err(|_| {
                eyre::eyre!(format!("Failed to parse balance {} to decimal", balance))
            })?,
        ))
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        let treasury = if let Some(address) = &self.treasury_address {
            address
        } else {
            return Err(eyre::eyre!("Treasury address is not set"));
        };
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let contract = ITreasury::new(treasury.address, Arc::new(&provider));
        let (_, last_header) = contract.light_client().call().await.unwrap();
        let light_client_header: BlockHeader = serde_spb::from_slice(&last_header).unwrap();
        Ok(light_client_header)
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        address: HexSerializedVec,
    ) -> Result<Decimal, Error> {
        let treasury = if let Some(address) = &self.treasury_address {
            address
        } else {
            return Err(eyre::eyre!("Treasury address is not set"));
        };
        let contract_address = EvmCompatibleAddress::from_hex_serialized_vec(&address)?.address;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let contract = IERC20::new(contract_address, Arc::new(provider));
        let balance = contract.balance_of(treasury.address).call().await.unwrap();
        Ok(Decimal::from(balance.as_u128()))
    }

    async fn get_treasury_non_fungible_token_balance(
        &self,
        address: HexSerializedVec,
    ) -> Result<Vec<HexSerializedVec>, Error> {
        todo!()
    }

    async fn update_treasury_light_client(
        &self,
        header: BlockHeader,
        proof: FinalizationProof,
    ) -> Result<(), Error> {
        let treasury = if let Some(address) = &self.treasury_address {
            address
        } else {
            return Err(eyre::eyre!("Treasury address is not set"));
        };
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let client = SignerMiddleware::new(&provider, wallet);
        let contract = ITreasury::new(treasury.address, Arc::new(client));
        let header = Bytes::from(
            serde_spb::to_vec(&header)
                .map_err(|_| eyre::eyre!("Failed to serialize block header"))?,
        );
        let proof = Bytes::from(
            serde_spb::to_vec(&proof)
                .map_err(|_| eyre::eyre!("Failed to serialize finalization proof"))?,
        );
        contract
            .update_light_client(header, proof)
            .gas_price(U256::from(10000000000u64))
            .send()
            .await
            .map_err(|err| eyre::eyre!("Failed to update light client: {}", err))?;
        Ok(())
    }

    async fn execute(
        &self,
        transaction: Transaction,
        block_height: u64,
        proof: MerkleProof,
    ) -> Result<(), Error> {
        let treasury = if let Some(address) = &self.treasury_address {
            address
        } else {
            return Err(eyre::eyre!("Treasury address is not set"));
        };
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let client = SignerMiddleware::new(&provider, wallet);
        let contract = ITreasury::new(treasury.address, Arc::new(client));
        let execution = convert_transaction_to_execution(&transaction).map_err(|_| {
            eyre::eyre!(format!(
                "Failed to convert transaction to execution: {:?}",
                transaction
            ))
        })?;
        let transaction = Bytes::from(
            serde_spb::to_vec(&transaction)
                .map_err(|_| eyre::eyre!("Failed to serialize transaction"))?,
        );

        let execution = Bytes::from(
            serde_spb::to_vec(&execution)
                .map_err(|_| eyre::eyre!("Failed to serialize execution"))?,
        );
        let proof = Bytes::from(
            serde_spb::to_vec(&proof)
                .map_err(|_| eyre::eyre!("Failed to serialize merkle proof"))?,
        );
        contract
            .execute(transaction, execution, block_height, proof)
            .send()
            .await
            .map_err(|err| eyre::eyre!(format!("Failed to execute: {:?}", err)))?;
        Ok(())
    }

    async fn eoa_get_sequence(&self, address: HexSerializedVec) -> Result<u128, Error> {
        let eoa = EvmCompatibleAddress::from_hex_serialized_vec(&address)?.address;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let sequence = provider
            .get_transaction_count(eoa, None)
            .await
            .map_err(|_| eyre::eyre!(format!("Failed to get sequence for address: {:?}", eoa)))?
            .as_u128();
        Ok(sequence)
    }

    async fn eoa_get_fungible_token_balance(
        &self,
        address: HexSerializedVec,
        token_address: HexSerializedVec,
    ) -> Result<Decimal, Error> {
        let eoa = EvmCompatibleAddress::from_hex_serialized_vec(&address)?.address;
        let contract_address =
            EvmCompatibleAddress::from_hex_serialized_vec(&token_address)?.address;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let contract = IERC20::new(contract_address, Arc::new(provider));
        let balance = contract.balance_of(eoa).call().await.unwrap();
        Ok(Decimal::from(balance.as_u128()))
    }

    async fn eoa_transfer_fungible_token(
        &self,
        address: HexSerializedVec,
        sender_private_key: HexSerializedVec,
        token_address: HexSerializedVec,
        receiver_address: HexSerializedVec,
        amount: Decimal,
    ) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let eoa = EvmCompatibleAddress::from_hex_serialized_vec(&address)?.address;
        let signer = SigningKey::from_slice(&sender_private_key.data.as_slice())?;
        let wallet = LocalWallet::new_with_signer(signer, eoa, chain_id);
        let client = SignerMiddleware::new(&provider, wallet);
        let contract_address =
            EvmCompatibleAddress::from_hex_serialized_vec(&token_address)?.address;
        let contract = IERC20::new(contract_address, Arc::new(client));
        let receiver_address =
            EvmCompatibleAddress::from_hex_serialized_vec(&receiver_address)?.address;
        let amount = U256::from_dec_str(amount.to_string().as_str()).unwrap();
        contract
            .transfer(receiver_address, amount)
            .send()
            .await
            .map_err(|_| eyre::eyre!("Failed to transfer fungible token"))?;
        Ok(())
    }
}
