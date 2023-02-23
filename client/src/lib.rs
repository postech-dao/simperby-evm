use std::str::FromStr;

use async_trait::async_trait;
use ethers::types::Address;
use ethers_core::types::{BlockId, BlockNumber};
use ethers_providers::{Http, Middleware, Provider};
use execution::*;
use eyre::Error;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_common::*;
use simperby_settlement::*;

pub struct EvmCompatibleChain {
    pub chain: ChainType,
    pub relayer_address_hex_str: String,
    pub treasury: Option<Treasury>,
}

pub struct Treasury {
    pub address: String,
    pub fungible_token: String,
    pub non_fungible_token: String,
    pub abi: String,
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
        let provider = Provider::<Http>::try_from(self.chain.get_rpc_url())?;
        let balance = provider.get_balance(from, None).await?.as_u128();
        Ok((address, Decimal::from(balance)))
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        todo!()
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Decimal, Error> {
        todo!()
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
        todo!()
    }

    async fn execute(
        &self,
        _execution: Execution,
        _block_height: u64,
        _proof: MerkleProof,
    ) -> Result<(), Error> {
        todo!()
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
        let eth_last_block = eth.get_last_block().await;
        // println!("{:?}", eth_last_block.is_ok());
        assert!(eth_last_block.is_ok());
        let eth_last_block = eth_last_block.unwrap();
        print!("{:?}", eth_last_block);
        assert!(eth_last_block.height > 0);
        assert!(eth_last_block.timestamp > 0);
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
    async fn get_relayer_acoount_info_from_local_node() {
        const LOCAL_NODE_RPC_URL: &str = "http://localhost:8545";
        // change this address to your own
        const ADDR: &str = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC";
        let chain_type = ChainType::Other(ChainInfo {
            rpc_url: LOCAL_NODE_RPC_URL.to_owned(),
            chain_name: Some("Local Node".to_owned()),
        });
        let chain = EvmCompatibleChain {
            chain: chain_type,
            relayer_address_hex_str: ADDR[2..].to_owned(),
            treasury: None,
        };
        let (address, balance) = chain.get_relayer_account_info().await.unwrap();
        println!("Checking connection to {}", LOCAL_NODE_RPC_URL);
        println!("Relayer address: {}, balance: {}", address, balance);
        assert_eq!(address, ADDR);
        assert!(balance >= Decimal::from(0));
    }
}
