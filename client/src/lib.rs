use async_trait::async_trait;
use dotenvy_macro::{self, dotenv};
use ethers::signers::Signer;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet};
use ethers::{contract::abigen, middleware::SignerMiddleware, types::Address};
use ethers_core::types::{BlockId, BlockNumber, Bytes};
use ethers_providers::{Http, Middleware, Provider};
use eyre::Error;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_common::*;
use simperby_settlement::execution::convert_transaction_to_execution;
use simperby_settlement::*;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};

const ADDRESS_HEX_LEN_WITH_PREFIX: usize = 42;

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

pub struct EvmCompatibleChain {
    pub chain: ChainType,
    pub treasury: Option<Treasury>,
}

pub struct Treasury {
    pub address: String,
    pub ft_contract_address_list: Option<Vec<(String, String)>>, // (token_name, token_address)
    pub nft_contract_address_list: Option<Vec<(String, String)>>, // (token_name, token_address)
}

pub struct ChainConfigs {
    rpc_url: String,
    chain_name: Option<String>,
}

pub enum ChainType {
    Ethereum(ChainConfigs),
    Polygon(ChainConfigs),
    BinanceSmartChain(ChainConfigs),
    Arbitrum(ChainConfigs),
    Optimism(ChainConfigs),
    Klaytn(ChainConfigs),
    Fantom(ChainConfigs),
    Avalanche(ChainConfigs),
    Moonbeam(ChainConfigs),
    Moonriver(ChainConfigs),
    Harmony(ChainConfigs),
    Celo(ChainConfigs),
    Other(ChainConfigs),
}

impl ChainType {
    fn get_rpc_url(&self) -> &str {
        match self {
            ChainType::Ethereum(chain) => chain.rpc_url.as_str(),
            ChainType::Polygon(chain) => chain.rpc_url.as_str(),
            ChainType::BinanceSmartChain(chain) => chain.rpc_url.as_str(),
            ChainType::Arbitrum(chain) => chain.rpc_url.as_str(),
            ChainType::Optimism(chain) => chain.rpc_url.as_str(),
            ChainType::Klaytn(chain) => chain.rpc_url.as_str(),
            ChainType::Fantom(chain) => chain.rpc_url.as_str(),
            ChainType::Avalanche(chain) => chain.rpc_url.as_str(),
            ChainType::Moonbeam(chain) => chain.rpc_url.as_str(),
            ChainType::Moonriver(chain) => chain.rpc_url.as_str(),
            ChainType::Harmony(chain) => chain.rpc_url.as_str(),
            ChainType::Celo(chain) => chain.rpc_url.as_str(),
            ChainType::Other(chain) => chain.rpc_url.as_str(),
        }
    }

    fn get_chain_name(&self) -> &str {
        match self {
            ChainType::Ethereum(_) => "Ethereum",
            ChainType::Polygon(_) => "Polygon",
            ChainType::BinanceSmartChain(_) => "BinanceSmartChain",
            ChainType::Arbitrum(_) => "Arbitrum",
            ChainType::Optimism(_) => "Optimism",
            ChainType::Klaytn(_) => "Klaytn",
            ChainType::Fantom(_) => "Fantom",
            ChainType::Avalanche(_) => "Avalanche",
            ChainType::Moonbeam(_) => "Moonbeam",
            ChainType::Moonriver(_) => "Moonriver",
            ChainType::Harmony(_) => "Harmony",
            ChainType::Celo(_) => "Celo",
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

#[async_trait]
impl SettlementChain for EvmCompatibleChain {
    async fn get_chain_name(&self) -> String {
        self.chain.get_chain_name().to_owned()
    }

    async fn check_connection(&self) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let block_number = provider.get_block_number().await;
        if block_number.is_err() {
            return Err(Error::msg(format!(
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
            return Err(Error::msg(format!(
                "Failed to get last block from chain {}",
                self.chain.get_chain_name()
            )));
        }
    }

    async fn get_relayer_account_info(&self) -> Result<(String, Decimal), Error> {
        let mut address = String::new();
        address.reserve(ADDRESS_HEX_LEN_WITH_PREFIX);
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url()).unwrap();
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let encoded = hex::encode(wallet.address().to_fixed_bytes());
        address.push_str("0x");
        address.push_str(encoded.as_str());
        let relayer_address = Address::from_str(address.as_str())?;
        let provider =
            Provider::<Http>::try_from(self.chain.get_rpc_url())?.interval(Duration::from_secs(1));
        let balance = provider.get_balance(relayer_address, None).await?.as_u128();
        Ok((address, Decimal::from(balance)))
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        let treasury = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let address = treasury
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid treasury address"))?;
        let contract = ITreasury::new(address, Arc::new(&provider));
        let (_, last_header) = contract.light_client().call().await.unwrap();
        let light_client_header: BlockHeader = bincode::deserialize(&last_header).unwrap();
        Ok(light_client_header)
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Decimal, Error> {
        let treasury_address = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let contract_address = _address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url()).unwrap();
        let contract = IERC20::new(contract_address, Arc::new(provider));
        let balance = contract.balance_of(treasury_address).call().await.unwrap();
        Ok(Decimal::from(balance.as_u128()))
    }

    async fn get_treasury_non_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Vec<String>, Error> {
        todo!()
    }

    async fn update_treasury_light_client(
        &self,
        _header: BlockHeader,
        _proof: FinalizationProof,
    ) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url()).unwrap();
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let client = SignerMiddleware::new(&provider, wallet);
        let contract_address = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let contract = ITreasury::new(contract_address, Arc::new(client));
        let header = Bytes::from(
            serde_spb::to_vec(&_header)
                .map_err(|_| Error::msg("Failed to serialize block header"))?,
        );
        let proof = Bytes::from(
            serde_spb::to_vec(&_proof)
                .map_err(|_| Error::msg("Failed to serialize finalization proof"))?,
        );
        contract
            .update_light_client(header, proof)
            .send()
            .await
            .map_err(|_| Error::msg("Failed to update light client"))?;
        Ok(())
    }

    async fn execute(
        &self,
        _transaction: Transaction,
        _block_height: u64,
        _proof: MerkleProof,
    ) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url()).unwrap();
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
        let client = SignerMiddleware::new(&provider, wallet);
        let contract_address = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let contract = ITreasury::new(contract_address, Arc::new(client));
        let transaction = Bytes::from(
            serde_spb::to_vec(&_transaction)
                .map_err(|_| Error::msg("Failed to serialize transaction"))?,
        );
        let execution = convert_transaction_to_execution(&_transaction).map_err(|_| {
            Error::msg(format!(
                "Failed to convert transaction to execution: {:?}",
                _transaction
            ))
        })?;
        let execution = Bytes::from(
            serde_spb::to_vec(&execution)
                .map_err(|_| Error::msg("Failed to serialize execution"))?,
        );
        println!("execution: {:?}", execution);
        let proof = Bytes::from(
            serde_spb::to_vec(&_proof)
                .map_err(|_| Error::msg("Failed to serialize merkle proof"))?,
        );
        contract
            .execute(transaction, execution, _block_height, proof)
            .send()
            .await
            .map_err(|err| Error::msg(format!("Failed to execute: {:?}", err)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ethers::signers::Signer;
    use ethers::solc::artifacts::Block;
    use ethers_core::types::{Signature, TransactionRequest};
    use ethers_core::types::{H160, U256};
    use rust_decimal::prelude::ToPrimitive;
    use simperby_common::merkle_tree::OneshotMerkleTree;
    use simperby_settlement::execution::Execution;

    use super::*;

    //이전에 쓰던거
    const HEADER: &str = "04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a075640400000000000000bdc284f3140c1d17fefa7b7db866767027345a547a6a13b7ed4e2389e9125b24477fe6396cf54ce2e6a5ff7f4df9ffeca6d15f645c8c46f0f62ca554d232813a1c04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a0756484a1b6122d41f0fea7884ae8949de8facfa6d124af26dbbf909881bf625212cb28c44b78580f28d8d2decfc8e97cb8923af71fbd8fa8dc7eb02485d29901c2801b04a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b408363c113b520cfd6c51fcf1978637562a1e26a455e66a713f48829b070cede740db28b8dba86d44a195158f51bb1494cac1d5d83752375d4e03c47c8459c591b04c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974cc2ba5fe4e40257408b5df5c44137ab439fa361a647769b2c0b2a79deee161bcc63e30417a822d731bb0bd15fafe544a2640dc85098f7ad95d1da18f29148b8c41c0420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc51455f978240d18bae917f6cbca88e19cd0ca603fed6f98dc5a43b002c56db1593a8801000000000000000000000000000000b1681c696f19ec0ef665900e49a1fd05f1d23534a01a0a8ff7233ce37384fb2f0000000000000000000000000000000000000000000000000000000000000000040000000000000004b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a07564010000000000000004a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b010000000000000004c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974c01000000000000000420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc5145501000000000000000500000000000000302e312e30";
    const PROOF: &str = "0400000000000000a7f48e414877566a80a99ba028901e9bed3c2aaee28f2b8d4d2db6ef4113ed7919011fd14ca73a276d3816c00688bba8d66a0d5a31641a10013e49ad546f8ab91b04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a075649b54b52df49b4486202fa9e91a5fbadbbb3d6e8014861145cf59b1bebbef9bda1865ed720da370212e9b2d6e4abb8984da1a497980c185924200ada0829bcb1f1b04a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b3fb9d560513e05fc48c2453f542b96db60dab8f4b51f8372ac82c1975032efc2487d4e0b34215a9a17b59ad3ce1fd59ed9f00563f1c6702c7a00436356e290551b04c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974cb47e9823ea61fb045724693c9a59980b5f04e00e8060a989a7e56593a45eb0a2525cd353e3d9d810b11c791df49b0be65f2ce00cb647c08ece0170ebae20568a1c0420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc51455";

    const TRANSACTION: &str = "0e00000000000000646f65736e2774206d61747465720000000000000000190000000000000065782d7472616e736665722d66743a206d797468657265756d56010000000000007b0a2020227461726765745f636861696e223a20226d797468657265756d222c0a202022636f6e74726163745f73657175656e6365223a20302c0a2020226d657373616765223a207b0a20202020225472616e7366657246756e6769626c65546f6b656e223a207b0a20202020202022746f6b656e5f61646472657373223a202232653233346461653735633739336636376133353038396339643939323435653163353834373062222c0a20202020202022616d6f756e74223a203130302c0a2020202020202272656365697665725f61646472657373223a202266333966643665353161616438386636663463653661623838323732373963666666623932323636220a202020207d0a20207d0a7d0a2d2d2d0a3464643363303333373565353938653838353032383265306662303764323436666561626265333133656565663261636239353365616131633838336666313600000000";
    const EXECUTION: &str = "09000000000000006d797468657265756d00000000000000000000000000000000010000002e234dae75c793f67a35089c9d99245e1c58470b64000000000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    const MERKLE_PROOF: &str = "020000000000000001000000a2a30a5b30235bd110c8fefa4b05346957b2250e762b75ab57cfd0791629df9701000000d828a7918714df1a888a05d8a8dfaf319fd05969e2cec9d9fc135ffaf762703d";

    // 내가 직접 만들어 쓰는 것
    const GENESIS_HEADER: &str = "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000004b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a07564010000000000000004a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b010000000000000004c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974c01000000000000000420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc5145501000000000000000500000000000000302e312e30";
    const GENESIS_PROOF: &str = "0400000000000000bdc284f3140c1d17fefa7b7db866767027345a547a6a13b7ed4e2389e9125b24477fe6396cf54ce2e6a5ff7f4df9ffeca6d15f645c8c46f0f62ca554d232813a1c04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a0756484a1b6122d41f0fea7884ae8949de8facfa6d124af26dbbf909881bf625212cb28c44b78580f28d8d2decfc8e97cb8923af71fbd8fa8dc7eb02485d29901c2801b04a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b408363c113b520cfd6c51fcf1978637562a1e26a455e66a713f48829b070cede740db28b8dba86d44a195158f51bb1494cac1d5d83752375d4e03c47c8459c591b04c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974cc2ba5fe4e40257408b5df5c44137ab439fa361a647769b2c0b2a79deee161bcc63e30417a822d731bb0bd15fafe544a2640dc85098f7ad95d1da18f29148b8c41c0420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc51455";
    const CREATED_TX:&str = "06000000000000004a494e574f4f0000000000000000160000000000000065782d7472616e736665722d66743a20474f45524c4951010000000000007b0a2020227461726765745f636861696e223a2022474f45524c49222c0a202022636f6e74726163745f73657175656e6365223a20322c0a2020226d657373616765223a207b0a20202020225472616e7366657246756e6769626c65546f6b656e223a207b0a20202020202022746f6b656e5f61646472657373223a202236306664626233323664366331373564633134363363353937333061366636353165343437373462222c0a20202020202022616d6f756e74223a203130302c0a2020202020202272656365697665725f61646472657373223a20223630363862343635643938386361656333613161343130636166396434663763313063623732220a202020207d0a20207d0a7d0a2d2d2d0a6632633237323937323966623831663839373238333361343633383065633331343763306562613331333266383830333131636666653835643538643139613800000000";
    const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
    const GOERLI_RPC_URL: &str = "https://goerli.infura.io/v3/9aa3d95b3bc440fa88ea12eaa4456161";
    const XDAI_RPC_URL: &str = "https://rpc.gnosischain.com";

    const TREASURY_ADDRESS: &str = "0xF4CC3d69DA2EBaDE740D12755766667eeC9aF19f";
    const ERC20_ADDRESS: &str = "0x60fdbb326d6c175dc1463C59730a6F651e44774b";
    const RECEIVER_ADDRESS: &str = "0x026068b465D988caec3a1A410caf9d4f7C10cB72";

    #[tokio::test]
    async fn check_chain_type() {
        let eth_type = ChainType::Ethereum(ChainConfigs {
            chain_name: Some("Ethereum".to_owned()),
            rpc_url: ETH_RPC_URL.to_owned(),
        });
        assert_eq!(eth_type.get_chain_name(), "Ethereum");
        assert_eq!(eth_type.get_rpc_url(), ETH_RPC_URL);
        let eth_type = ChainType::Ethereum(ChainConfigs {
            chain_name: None,
            rpc_url: ETH_RPC_URL.to_owned(),
        });
        assert_eq!(eth_type.get_chain_name(), "Ethereum");
        assert_eq!(eth_type.get_rpc_url(), ETH_RPC_URL);
        let unknown = ChainType::Other(ChainConfigs {
            chain_name: None,
            rpc_url: XDAI_RPC_URL.to_owned(),
        });
        assert_eq!(unknown.get_chain_name(), "Unknown");
        assert_eq!(unknown.get_rpc_url(), XDAI_RPC_URL);
    }

    #[tokio::test]
    async fn check_connection() {
        // Ethereum
        let eth_type = ChainType::Ethereum(ChainConfigs {
            rpc_url: ETH_RPC_URL.to_owned(),
            chain_name: None,
        });
        let eth = EvmCompatibleChain {
            chain: eth_type,
            treasury: None,
        };
        let eth_connection = eth.check_connection().await;
        assert!(eth_connection.is_ok());
    }

    #[tokio::test]
    async fn get_last_block() {
        let provider = Provider::<Http>::try_from(ETH_RPC_URL).unwrap();
        let eth_last_block = provider
            .get_block(BlockId::Number(BlockNumber::Latest))
            .await
            .unwrap();
        let eth_last_block = eth_last_block.unwrap();
        println!(
            "Ethereum last block: {}",
            eth_last_block.number.unwrap().as_u64()
        );
        println!("Timestamp: {}", eth_last_block.timestamp.as_u64());
        assert!(eth_last_block.number.unwrap().as_u64() > 0);
        assert!(eth_last_block.timestamp.as_u64() > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn get_relayer_account() {
        let chain_type = ChainType::Other(ChainConfigs {
            rpc_url: GOERLI_RPC_URL.to_owned(),
            chain_name: Some("Goerli".to_owned()),
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            treasury: None,
        };
        let (address, balance) = chain.get_relayer_account_info().await.unwrap();
        assert_eq!(address.len(), 42);
        assert_eq!(&address[0..2], "0x");
        assert!(balance >= Decimal::from(0));
    }

    #[tokio::test]
    #[ignore]
    async fn update_light_client() {
        let chain_type = ChainType::Other(ChainConfigs {
            rpc_url: GOERLI_RPC_URL.to_owned(),
            chain_name: Some("Goerli".to_owned()),
        });
        let treasury = Some(Treasury {
            address: TREASURY_ADDRESS.to_owned(),
            ft_contract_address_list: None,
            nft_contract_address_list: None,
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            treasury,
        };
        let header: BlockHeader = serde_spb::from_slice(&hex::decode(HEADER).unwrap()).unwrap();
        let proof: FinalizationProof = serde_spb::from_slice(&hex::decode(PROOF).unwrap()).unwrap();
        let is_updated = chain
            .update_treasury_light_client(header.clone(), proof)
            .await
            .is_ok();
        assert!(is_updated);
    }

    #[tokio::test]
    #[ignore]
    async fn check_last_header_after_update() {
        let chain_type = ChainType::Other(ChainConfigs {
            rpc_url: GOERLI_RPC_URL.to_owned(),
            chain_name: Some("Goerli".to_owned()),
        });
        let treasury = Some(Treasury {
            address: TREASURY_ADDRESS.to_owned(),
            ft_contract_address_list: None,
            nft_contract_address_list: None,
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            treasury,
        };
        let header: BlockHeader = serde_spb::from_slice(&hex::decode(HEADER).unwrap()).unwrap();
        let last_header = chain.get_light_client_header().await.unwrap();
        assert_eq!(last_header, header);
        println!(
            "last_header: {:?}",
            hex::encode(serde_spb::to_vec(&last_header).unwrap())
        );
    }

    use execution::*;

    #[tokio::test]
    #[ignore]
    async fn execute() {
        let chain_type = ChainType::Other(ChainConfigs {
            rpc_url: GOERLI_RPC_URL.to_owned(),
            chain_name: Some("Goerli".to_owned()),
        });
        let provider = Provider::try_from(GOERLI_RPC_URL).unwrap();
        let chain_id = provider.get_chainid().await.unwrap().as_u64();
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(dotenv!("RELAYER_MNEMONIC"))
            .build()
            .unwrap()
            .with_chain_id(chain_id);
    }

    #[tokio::test]
    #[ignore]
    async fn create_tx() {
        let erc20_address = hex::decode(&ERC20_ADDRESS[2..]).unwrap();
        let amount = 100;
        let receiver_address = hex::decode(&RECEIVER_ADDRESS[2..]).unwrap();
        let execution = Execution {
            target_chain: "GOERLI".to_owned(),
            contract_sequence: 2,
            message: ExecutionMessage::TransferFungibleToken(TransferFungibleToken {
                token_address: HexSerializedVec {
                    data: erc20_address,
                },
                amount,
                receiver_address: HexSerializedVec {
                    data: receiver_address,
                },
            }),
        };
        let author = "JINWOO".to_owned();
        let timestamp = 0;
        let ex_tx = execution::create_execution_transaction(&execution, author, timestamp).unwrap();
        println!("ex_tx: {:?}", ex_tx);
        let encoded_tx = hex::encode(serde_spb::to_vec(&ex_tx).unwrap());
        println!("encoded_tx: {}", encoded_tx);
    }


    use simperby_common::{verify::CommitSequenceVerifier, *};

    pub struct Chain {
        pub chain_name: String,
        pub last_finalized_header: BlockHeader,
        pub last_finalization_proof: FinalizationProof,
        pub reserved_state: ReservedState,
        /// The private keys of the validators of the next block.
        ///
        /// Both governance and consensus sets must be the same.
        pub validators: Vec<PrivateKey>,
    }

    impl Chain {
        /// Creates Chain info from the standard genesis test suite.
        ///
        /// This is useful when you want to test the treasury for the first time.
        pub fn standard_genesis(chain_name: String) -> Self {
            let (reserved_state, validators) = test_utils::generate_standard_genesis(4);
            Self {
                chain_name,
                last_finalized_header: reserved_state.genesis_info.header.clone(),
                last_finalization_proof: reserved_state.genesis_info.genesis_proof.clone(),
                reserved_state,
                validators: validators
                    .into_iter()
                    .map(|(_, private_key)| private_key)
                    .collect(),
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn create_genesis_header() {
        let chain = Chain::standard_genesis("TEST".to_owned());
        let header = chain.last_finalized_header.clone();
        let proof = chain.last_finalization_proof.clone();
        println!(
            "header: {:?}",
            hex::encode(serde_spb::to_vec(&header).unwrap())
        );
        println!(
            "proof: {:?}",
            hex::encode(serde_spb::to_vec(&proof).unwrap())
        );
    }

    async fn eoa_get_ft_balance(eoa: String, ft_address: String, rpc_url: String) -> u64 {
        let provider = Provider::try_from(rpc_url).unwrap();
        let eoa = Address::from_slice(&hex::decode(&eoa[2..]).unwrap());
        let ca = Address::from_slice(&hex::decode(&ft_address[2..]).unwrap());
        let contract = IERC20::new(ca, Arc::new(provider));
        contract.balance_of(eoa).call().await.unwrap().as_u64()
    }

    #[tokio::test]
    #[ignore]
    async fn scenario() {
        // Setup the on-chain state
        let local_node_url = "http://localhost:8545";
        const TREASURY_ADDRESS: &str = "0x9683858e4B429315A8007723819d7deffBB211Cd";
        const ERC20_ADDRESS: &str = "0x50DAa9BC6862EA35163eDac8Ee21637eDe18f7e8";
        const RECEIVER_ADDRESS: &str = "0x1bc43e9283D35DCC205bE7225069B4B6f1f2287C";

        let sc = EvmCompatibleChain {
            chain: ChainType::Other(ChainConfigs {
                rpc_url: local_node_url.to_owned(),
                chain_name: Some("Local".to_owned()),
            }),
            treasury: Some(Treasury {
                address: TREASURY_ADDRESS.to_owned(),
                ft_contract_address_list: None,
                nft_contract_address_list: None,
            }),
        };
        let chain = Chain::standard_genesis("mythereum".to_owned());
        let mut csv = CommitSequenceVerifier::new(
            chain.last_finalized_header.clone(),
            chain.reserved_state.clone(),
        )
        .unwrap();

        // Query the initial status
        let intial_balance = sc.get_treasury_fungible_token_balance(ERC20_ADDRESS.to_owned()).await.unwrap().to_u128().unwrap();
        let initial_treasury_header: BlockHeader =
            serde_spb::from_slice(&hex::decode(GENESIS_HEADER).unwrap()).unwrap();
        let initial_contract_sequence = 0;
        let initial_temporary_receiver_balance = eoa_get_ft_balance(
            RECEIVER_ADDRESS.to_owned(),
            ERC20_ADDRESS.to_owned(),
            local_node_url.to_owned(),
        )
        .await;
        assert_eq!(initial_treasury_header, chain.last_finalized_header);

        // Apply transactions
        let mut transactions = Vec::new();
        let erc20_address = HexSerializedVec {
            data: hex::decode(&ERC20_ADDRESS[2..]).unwrap(),
        };
        let temporary_receiver_address = HexSerializedVec {
            data: hex::decode(&RECEIVER_ADDRESS[2..]).unwrap(),
        };
        let execute_tx = execution::create_execution_transaction(
            &Execution {
                target_chain: chain.chain_name,
                contract_sequence: initial_contract_sequence,
                message: ExecutionMessage::TransferFungibleToken(TransferFungibleToken {
                    token_address: erc20_address.clone(),
                    amount: intial_balance,
                    receiver_address: temporary_receiver_address.clone(),
                }),
            },
            "jinwoo".to_owned(),
            0,
        )
        .unwrap();
        println!(
            "execute_tx: {:?}",
            &execute_tx
        );
        println!(
            "execute_tx: {:?}",
            hex::encode(serde_spb::to_vec(&execute_tx).unwrap())
        );
        csv.apply_commit(&Commit::Transaction(execute_tx.clone()))
            .unwrap();
        transactions.push(execute_tx.clone());

        // Complete the block
        let agenda = Agenda {
            height: 1,
            author: chain.reserved_state.consensus_leader_order[0].clone(),
            timestamp: 1,
            transactions_hash: Agenda::calculate_transactions_hash(&transactions),
        };
        csv.apply_commit(&Commit::Agenda(agenda.clone())).unwrap();
        csv.apply_commit(&Commit::AgendaProof(AgendaProof {
            height: 1,
            agenda_hash: agenda.to_hash256(),
            proof: chain
                .validators
                .iter()
                .map(|private_key| TypedSignature::sign(&agenda, private_key).unwrap())
                .collect::<Vec<_>>(),
            timestamp: 0,
        }))
        .unwrap();
        let block_header = BlockHeader {
            author: chain.validators[0].public_key(),
            prev_block_finalization_proof: chain.last_finalization_proof,
            previous_hash: chain.last_finalized_header.to_hash256(),
            height: 1,
            timestamp: 0,
            commit_merkle_root: BlockHeader::calculate_commit_merkle_root(
                &csv.get_total_commits()[1..],
            ),
            repository_merkle_root: Hash256::zero(),
            validator_set: chain.last_finalized_header.validator_set.clone(),
            version: chain.last_finalized_header.version,
        };
        csv.apply_commit(&Commit::Block(block_header.clone()))
            .unwrap();
        println!(
            "block_header: {:?}",
            hex::encode(serde_spb::to_vec(&block_header).unwrap())
        );
        let fp = chain
            .validators
            .iter()
            .map(|private_key| TypedSignature::sign(&block_header, private_key).unwrap())
            .collect::<Vec<_>>();
        csv.verify_last_header_finalization(&fp).unwrap();
        println!("fp: {:?}", hex::encode(serde_spb::to_vec(&fp).unwrap()));

        // Update light client
        sc.update_treasury_light_client(block_header.clone(), fp)
            .await
            .unwrap();
        assert_eq!(sc.get_light_client_header().await.unwrap(), block_header);

        // Execute transfer
        let commits = csv.get_total_commits();
        let merkle_tree = OneshotMerkleTree::create(
            commits[1..=(commits.len() - 2)]
                .iter()
                .map(|c| c.to_hash256())
                .collect(),
        );
        let merkle_proof = merkle_tree
            .create_merkle_proof(execute_tx.to_hash256())
            .unwrap();
        println!(
            "merkle_proof: {:?}",
            hex::encode(serde_spb::to_vec(&merkle_proof).unwrap())
        );
        sc.execute(execute_tx, 1, merkle_proof).await.unwrap();

        // Check the result
        let balance_after_tx = eoa_get_ft_balance(
            RECEIVER_ADDRESS.to_owned(),
            ERC20_ADDRESS.to_owned(),
            local_node_url.to_owned(),
        )
        .await;
        assert_eq!(
            balance_after_tx as u128,
            initial_temporary_receiver_balance as u128 + intial_balance
        );
    }
}
