use async_trait::async_trait;
use ethers::{contract::abigen, types::Address};
use ethers_core::types::{BlockId, BlockNumber, Bytes, H256, U256};
use ethers_providers::{Http, Middleware, Provider};
use eyre::Error;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_common::*;
use simperby_settlement::execution::convert_transaction_to_execution;
use simperby_settlement::*;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};

abigen!(
    ITreasury,
    r#"[
        function updateLightClient(bytes memory header, bytes memory proof) public
        function name() external view returns (string memory)
        function chainName() external view returns (bytes memory)
        function contractSequence() external view returns (uint128)
        function lightClient() external view returns (uint64 heightOffset, bytes memory lastHeader)
        function viewCommitRoots() external view returns (bytes32[] memory commitRoots)
        function execute(bytes memory transaction,bytes memory executionHash, uint64 blockHeight, bytes memory merkleProof) public
    ]"#,
);

abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function totalSupply() public view returns (uint256)
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
    pub relayer_address_hex_str: String,
    pub treasury: Option<Treasury>,
}

pub struct Treasury {
    pub address: String,
    pub ft_contract_address_list: Option<Vec<(String, String)>>, // (token_name, token_address)
    pub nft_contract_address_list: Option<Vec<(String, String)>>, // (token_name, token_address)
}

pub struct ChainInfo {
    rpc_url: String,
    chain_name: Option<String>,
}

pub enum ChainType {
    Ethereum(ChainInfo),
    Polygon(ChainInfo),
    BinanceSmartChain(ChainInfo),
    Arbitrum(ChainInfo),
    Optimism(ChainInfo),
    Klaytn(ChainInfo),
    Fantom(ChainInfo),
    Avalanche(ChainInfo),
    Moonbeam(ChainInfo),
    Moonriver(ChainInfo),
    Harmony(ChainInfo),
    Celo(ChainInfo),
    Other(ChainInfo),
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
            ChainType::Other(chain) => {
                if chain.chain_name.is_some() {
                    chain.chain_name.as_ref().unwrap().as_str()
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
        if block.is_some() {
            let block = block.unwrap();
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
        address.reserve(42);
        let hex_str = self.relayer_address_hex_str.as_str();
        if (hex_str.len() != 40 && hex_str.len() != 42)
            || (hex_str.len() == 42 && !hex_str.starts_with("0x"))
            || (hex_str.len() == 40 && hex_str.starts_with("0x"))
        {
            return Err(Error::msg(format!("Invalid relayer address {}", hex_str)));
        }
        if hex_str.len() == 40 {
            address.push_str("0x");
        }
        address.push_str(hex_str);
        let from = Address::from_str(address.as_str())?;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?.interval(Duration::from_secs(1));

        let balance = provider.get_balance(from, None).await?.as_u128();
        Ok((address, Decimal::from(balance)))
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        let treasury = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?;
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let provider = Arc::new(provider);
        let address = treasury
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid treasury address"))?;
        let contract = ITreasury::new(address, Arc::clone(&provider));
        let (_, last_header) = contract.light_client().call().await.unwrap();
        let light_client_header: BlockHeader = bincode::deserialize(&last_header).unwrap();
        Ok(light_client_header)
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Decimal, Error> {
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url()).unwrap();
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
        let sender = self.relayer_address_hex_str.parse::<Address>().unwrap();
        let provider = Arc::new(provider.with_sender(sender));
        let address = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let contract = ITreasury::new(address, Arc::clone(&provider));
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
        let sender = self.relayer_address_hex_str.parse::<Address>().unwrap();
        let provider = Arc::new(provider.with_sender(sender));
        let address = self
            .treasury
            .as_ref()
            .ok_or_else(|| Error::msg("Treasury is not set"))?
            .address
            .parse::<Address>()
            .map_err(|_| Error::msg("Invalid address"))?;
        let contract = ITreasury::new(address, Arc::clone(&provider));
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
        let proof = Bytes::from(
            serde_spb::to_vec(&_proof)
                .map_err(|_| Error::msg("Failed to serialize merkle proof"))?,
        );
        let block_height = _block_height;
        contract
            .execute(transaction, execution, block_height, proof)
            .send()
            .await
            .map_err(|_| Error::msg("Failed to execute"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chain_type() {
        // Ethereum
        const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
        let eth_type = ChainType::Ethereum(ChainInfo {
            chain_name: Some("Ethereum".to_owned()),
            rpc_url: ETH_RPC_URL.to_owned(),
        });
        assert_eq!(eth_type.get_chain_name(), "Ethereum");
        assert_eq!(eth_type.get_rpc_url(), ETH_RPC_URL);

        // Ethereum without chain name
        let eth_type = ChainType::Ethereum(ChainInfo {
            chain_name: None,
            rpc_url: ETH_RPC_URL.to_owned(),
        });
        assert_eq!(eth_type.get_chain_name(), "Ethereum");
        assert_eq!(eth_type.get_rpc_url(), ETH_RPC_URL);

        // Other
        const XDAI_RPC_URL: &str = "https://rpc.gnosischain.com";
        let xdai_type = ChainType::Other(ChainInfo {
            chain_name: Some("XDAI".to_owned()),
            rpc_url: XDAI_RPC_URL.to_owned(),
        });
        assert_eq!(xdai_type.get_chain_name(), "XDAI");
        assert_eq!(xdai_type.get_rpc_url(), XDAI_RPC_URL);

        // Other without chain name
        let unknown = ChainType::Other(ChainInfo {
            chain_name: None,
            rpc_url: XDAI_RPC_URL.to_owned(),
        });
        assert_eq!(unknown.get_chain_name(), "Unknown");
        assert_eq!(unknown.get_rpc_url(), XDAI_RPC_URL);
    }

    #[tokio::test]
    async fn check_connection() {
        // Ethereum
        const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
        let eth_type = ChainType::Ethereum(ChainInfo {
            rpc_url: ETH_RPC_URL.to_owned(),
            chain_name: None,
        });
        let eth = EvmCompatibleChain {
            chain: eth_type,
            relayer_address_hex_str: "0x0000000".to_owned(),
            treasury: None,
        };
        let eth_connection = eth.check_connection().await;
        assert!(eth_connection.is_ok());
    }

    #[tokio::test]
    async fn get_last_block() {
        const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
        let provider = Provider::<Http>::try_from(ETH_RPC_URL).unwrap();
        let eth_last_block = provider
            .get_block(BlockId::Number(BlockNumber::Latest))
            .await
            .unwrap();
        let eth_last_block = eth_last_block.unwrap();
        println!("{}", eth_last_block.number.unwrap().as_u64());
        println!("{}", eth_last_block.timestamp.as_u64());
        assert!(eth_last_block.number.unwrap().as_u64() > 0);
        assert!(eth_last_block.timestamp.as_u64() > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn check_connection_to_the_mainnet_of_the_chains() {
        // RPC URLs
        const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
        const POLYGON_RPC_URL: &str = "https://rpc-mainnet.maticvigil.com";
        const BSC_RPC_URL: &str = "https://bsc-dataseed.binance.org";
        const ARBITRUM_RPC_URL: &str = "https://arb1.arbitrum.io/rpc";
        const OPTIMISM_RPC_URL: &str = "https://mainnet.optimism.io";
        const KLAYTN_RPC_URL: &str = "https://public-node-api.klaytnapi.com/v1/cypress";
        const FANTOM_RPC_URL: &str = "https://rpcapi.fantom.network";
        const AVALANCHE_RPC_URL: &str = "https://api.avax.network/ext/bc/C/rpc";
        const MOONBEAM_RPC_URL: &str = "https://moonbeam.api.onfinality.io/public";
        const MOONRIVER_RPC_URL: &str = "https://moonriver.api.onfinality.io/public";
        const CELO_RPC_URL: &str = "https://forno.celo.org";
        const HARMONY_RPC_URL: &str = "https://api.s0.t.hmny.io";
        const XDAI_RPC_URL: &str = "https://rpc.gnosischain.com";
        let urls = vec![
            ETH_RPC_URL,
            POLYGON_RPC_URL,
            BSC_RPC_URL,
            ARBITRUM_RPC_URL,
            OPTIMISM_RPC_URL,
            KLAYTN_RPC_URL,
            FANTOM_RPC_URL,
            AVALANCHE_RPC_URL,
            MOONBEAM_RPC_URL,
            MOONRIVER_RPC_URL,
            CELO_RPC_URL,
            HARMONY_RPC_URL,
            XDAI_RPC_URL,
        ];
        for url in urls {
            let chain_type = ChainType::Other(ChainInfo {
                rpc_url: url.to_owned(),
                chain_name: None,
            });
            let chain = EvmCompatibleChain {
                chain: chain_type,
                relayer_address_hex_str: "0x0000000".to_owned(),
                treasury: None,
            };
            println!("Checking connection to {}", url);
            let connection = chain.check_connection().await;
            assert!(connection.is_ok());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn check_connection_to_local_node() {
        const LOCAL_NODE_RPC_URL: &str = "http://localhost:8545";
        let chain_type = ChainType::Other(ChainInfo {
            rpc_url: LOCAL_NODE_RPC_URL.to_owned(),
            chain_name: Some("Local Node".to_owned()),
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            relayer_address_hex_str: "0x0000000".to_owned(),
            treasury: None,
        };
        println!("Checking connection to {}", LOCAL_NODE_RPC_URL);
        let connection = chain.check_connection().await;
        assert!(connection.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn get_relayer_acoount_info_on_local() {
        const LOCAL_NODE_RPC_URL: &str = "http://localhost:8545";
        // change this address to your own
        const ADDRESS: &str = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC";
        let chain_type = ChainType::Other(ChainInfo {
            rpc_url: LOCAL_NODE_RPC_URL.to_owned(),
            chain_name: Some("Local Node".to_owned()),
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            relayer_address_hex_str: ADDRESS[2..].to_owned(),
            treasury: None,
        };
        let (address, balance) = chain.get_relayer_account_info().await.unwrap();
        println!("Checking connection to {}", LOCAL_NODE_RPC_URL);
        println!("Relayer address: {}, balance: {}", address, balance);
        assert_eq!(address, ADDRESS);
        assert!(balance >= Decimal::from(0));
    }

    #[tokio::test]
    #[ignore]
    async fn get_light_client_on_local() {
        let address = "0xEC7F43Fe03E9AFDCE2F87AdF50D34F1C49492841";
        let treasury = Treasury {
            address: address.to_owned(),
            ft_contract_address_list: None,
            nft_contract_address_list: None,
        };
        let client = EvmCompatibleChain {
            chain: ChainType::Other(ChainInfo {
                rpc_url: "http://localhost:8545".to_owned(),
                chain_name: Some("Local Node".to_owned()),
            }),
            relayer_address_hex_str: "0x619b2fe763f885f59ce96e1bec1375d2c94c9f4a".to_owned(),
            treasury: Some(treasury),
        };
        let header = client.get_light_client_header().await.unwrap();
        print!("Header: {:?}", header);
    }

    #[tokio::test]
    #[ignore]
    async fn update_light_client_on_local() {
        let _header = "04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a075640400000000000000bdc284f3140c1d17fefa7b7db866767027345a547a6a13b7ed4e2389e9125b24477fe6396cf54ce2e6a5ff7f4df9ffeca6d15f645c8c46f0f62ca554d232813a1c04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a0756484a1b6122d41f0fea7884ae8949de8facfa6d124af26dbbf909881bf625212cb28c44b78580f28d8d2decfc8e97cb8923af71fbd8fa8dc7eb02485d29901c2801b04a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b408363c113b520cfd6c51fcf1978637562a1e26a455e66a713f48829b070cede740db28b8dba86d44a195158f51bb1494cac1d5d83752375d4e03c47c8459c591b04c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974cc2ba5fe4e40257408b5df5c44137ab439fa361a647769b2c0b2a79deee161bcc63e30417a822d731bb0bd15fafe544a2640dc85098f7ad95d1da18f29148b8c41c0420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc51455f978240d18bae917f6cbca88e19cd0ca603fed6f98dc5a43b002c56db1593a8801000000000000000000000000000000b1681c696f19ec0ef665900e49a1fd05f1d23534a01a0a8ff7233ce37384fb2f0000000000000000000000000000000000000000000000000000000000000000040000000000000004b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a07564010000000000000004a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b010000000000000004c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974c01000000000000000420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc5145501000000000000000500000000000000302e312e30";
        let _proof = "0400000000000000a7f48e414877566a80a99ba028901e9bed3c2aaee28f2b8d4d2db6ef4113ed7919011fd14ca73a276d3816c00688bba8d66a0d5a31641a10013e49ad546f8ab91b04b31b74ad078b082cad69775717016d7fbfae7b9f7dde8d1d988e0ff2e2b30e9413090e436c7c2a2c06e7ddf69484aeaaadc7ecbf1dd92459769ba96043a075649b54b52df49b4486202fa9e91a5fbadbbb3d6e8014861145cf59b1bebbef9bda1865ed720da370212e9b2d6e4abb8984da1a497980c185924200ada0829bcb1f1b04a688f0a4f9c863b6aa927e0df198307e058999c3ea8a012e47e1c598a70b67b383c8a3f7b2a392904e71689595147334e821985b1175b10fbc47d1d9ffd4ec6b3fb9d560513e05fc48c2453f542b96db60dab8f4b51f8372ac82c1975032efc2487d4e0b34215a9a17b59ad3ce1fd59ed9f00563f1c6702c7a00436356e290551b04c1b5a31db87d102ac45efe81288a1ea380abca214a37b3b9bc9ad1da984f08c4d40e948e6548df924ee7f2513324136f40fe20ebe77a1ee019e526ea6e3b974cb47e9823ea61fb045724693c9a59980b5f04e00e8060a989a7e56593a45eb0a2525cd353e3d9d810b11c791df49b0be65f2ce00cb647c08ece0170ebae20568a1c0420e4b9d289f068377a1ec0c37fd89661a60351914cacaca2f116c95d0ec0e8a7f48f22a495f6922c8b48790975d4a639f320135e89c98c30cf0da2201fc51455";
        let header =
            serde_spb::from_slice::<BlockHeader>(hex::decode(_header).unwrap().as_slice()).unwrap();
        let proof =
            serde_spb::from_slice::<FinalizationProof>(hex::decode(_proof).unwrap().as_slice())
                .unwrap();
        let encoded_header = hex::encode(serde_spb::to_vec(&header).unwrap());
        let encoded_proof = hex::encode(serde_spb::to_vec(&proof).unwrap());
        assert_eq!(encoded_header, _header);
        assert_eq!(encoded_proof, _proof);
        let client = EvmCompatibleChain {
            chain: ChainType::Other(ChainInfo {
                rpc_url: "http://localhost:8545".to_owned(),
                chain_name: Some("Local Node".to_owned()),
            }),
            relayer_address_hex_str: "0x1bc43e9283D35DCC205bE7225069B4B6f1f2287C".to_owned(),
            treasury: Some(Treasury {
                address: "0x66De55F4457948e40e68c355304a4082844a5349".to_owned(),
                ft_contract_address_list: None,
                nft_contract_address_list: None,
            }),
        };
        let last_header = client.get_light_client_header().await.unwrap();
        println!("last_header before update: {:?}", last_header);
        client
            .update_treasury_light_client(header.clone(), proof)
            .await
            .unwrap();
        let last_header = client.get_light_client_header().await.unwrap();
        assert_eq!(&last_header, &header);
    }

    #[tokio::test]
    #[ignore]
    async fn tx_to_execution() {
        let tx: Transaction = Transaction {
            author: "".to_owned(),
            timestamp: 1,
            head: "".to_owned(),
            body: "".to_owned(),
            diff: Diff::None,
        };
        let execution = convert_transaction_to_execution(&tx).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn get_ft_balance() {
        let treasury_address = "0x619b2fe763f885f59ce96e1bec1375d2c94c9f4a";
        let contract_address = "0x9683858e4B429315A8007723819d7deffBB211Cd";
        let treasury = Treasury {
            address: treasury_address.to_owned(),
            ft_contract_address_list: None,
            nft_contract_address_list: None,
        };
        let client = EvmCompatibleChain {
            chain: ChainType::Other(ChainInfo {
                rpc_url: "http://localhost:8545".to_owned(),
                chain_name: Some("Local Node".to_owned()),
            }),
            relayer_address_hex_str: "0x619b2fe763f885f59ce96e1bec1375d2c94c9f4a".to_owned(),
            treasury: Some(treasury),
        };
        let balance = client
            .get_treasury_fungible_token_balance(contract_address.to_owned())
            .await
            .unwrap();
        println!("balance: {}", balance);
    }

    #[tokio::test]
    #[ignore]
    async fn ft_total_supply() {
        let contract_address = "0x9683858e4B429315A8007723819d7deffBB211Cd"
            .to_owned()
            .parse::<Address>()
            .unwrap();
        let provider = Provider::try_from("http://localhost:8545").unwrap();
        let contract = IERC20::new(contract_address, Arc::new(provider));
        let total_supply = contract.total_supply().call().await.unwrap();

        let owner_address = "0x619B2fE763f885f59Ce96e1Bec1375d2C94c9F4A"
            .to_owned()
            .parse::<Address>()
            .unwrap();
        let balance = contract.balance_of(owner_address).call().await.unwrap();

        println!("total supply: {}", total_supply);
        println!("balance: {}", balance);
    }
}
