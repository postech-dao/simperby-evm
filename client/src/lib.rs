use async_trait::async_trait;
use ethers::{
    abi::Abi,
    contract::Contract,
    providers::{Http, Provider},
    signers::{LocalWallet, Wallet},
    types::{Address, H256, U256},
};
use ethers_providers::Middleware;
use execution::*;
use eyre::Error;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_common::*;
use simperby_settlement::*;
use std::{convert::TryFrom, str::FromStr};

pub struct Ethereum {
    pub chain_name: String,
    pub full_node_endpoint_uri: String,
    pub relayer_pub_key_and_secret_key: (String, String),
    pub treasury_address_and_abi: (String, String),
    pub light_client_address_and_abi: (String, String),
}

#[async_trait]
impl SettlementChain for Ethereum {
    async fn get_chain_name(&self) -> String {
        self.chain_name.clone()
    }

    async fn check_connection(&self) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        provider.get_block_number().await.unwrap();
        Ok(())
    }

    async fn get_last_block(&self) -> Result<SettlementChainBlock, Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let last_block_number = provider.get_block_number().await.unwrap();
        let last_block = provider.get_block(last_block_number).await?;
        let timestamp = last_block.unwrap().timestamp.as_u64();
        Ok(SettlementChainBlock {
            height: last_block_number.as_u64(),
            timestamp,
        })
    }

    async fn get_relayer_account_info(&self) -> Result<(String, Decimal), Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (pub_key, _) = &self.relayer_pub_key_and_secret_key;
        let address = pub_key.parse::<Address>().unwrap();
        let balance = provider.get_balance(address, None).await?;
        let balance_in_decimal = Decimal::from_str_exact(&balance.to_string()).unwrap();
        Ok((address.to_string(), balance_in_decimal))
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (light_client_address, light_client_abi) = &self.light_client_address_and_abi;
        let address = light_client_address.parse::<Address>().unwrap();
        let abi: Abi = serde_json::from_str(light_client_abi).unwrap();
        let contract = Contract::new(address, abi, provider);
        todo!()
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Decimal, Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (treasury_address, treasury_abi) = &self.treasury_address_and_abi;
        let contract_address = treasury_address.parse::<Address>().unwrap();
        let abi: Abi = serde_json::from_str(treasury_abi).unwrap();
        let contract = Contract::new(contract_address, abi, provider);
        let _address = _address.parse::<Address>().unwrap();
        let balance = contract
            .method::<_, U256>("balanceOf", (_address,))?
            .call()
            .await?
            .to_string();
        Ok(Decimal::from_str_exact(&balance).unwrap())
    }

    async fn get_treasury_non_fungible_token_balance(
        &self,
        _address: String,
    ) -> Result<Vec<String>, Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (treasury_address, treasury_abi) = &self.treasury_address_and_abi;
        let contract_address = treasury_address.parse::<Address>().unwrap();
        let abi: Abi = serde_json::from_str(treasury_abi).unwrap();
        let contract = Contract::new(contract_address, abi, provider);
        let _address = _address.parse::<Address>().unwrap();
        let balance = contract
            .method::<_, Vec<String>>("balanceOf", (_address,))?
            .call()
            .await?;
        Ok(balance)
    }

    async fn update_treasury_light_client(
        &self,
        _header: BlockHeader,
        _proof: FinalizationProof,
    ) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (pub_key, secret_key) = &self.relayer_pub_key_and_secret_key;
        let (treasury_address, treasury_abi) = &self.treasury_address_and_abi;
        let contract_address = treasury_address.parse::<Address>().unwrap();
        let abi: Abi = serde_json::from_str(treasury_abi).unwrap();
        let wallet = LocalWallet::from_str(secret_key).unwrap();
        let contract = Contract::new(contract_address, abi, provider);
        todo!()
    }

    async fn execute(
        &self,
        _execution: Execution,
        _block_height: u64,
        _proof: MerkleProof,
    ) -> Result<(), Error> {
        let provider = Provider::<Http>::try_from(&self.full_node_endpoint_uri)?;
        let (pub_key, secret_key) = &self.relayer_pub_key_and_secret_key;
        let (treasury_address, treasury_abi) = &self.treasury_address_and_abi;
        let contract_address = treasury_address.parse::<Address>().unwrap();
        let abi: Abi = serde_json::from_str(treasury_abi).unwrap();
        let wallet = LocalWallet::from_str(secret_key).unwrap();
        let contract = Contract::new(contract_address, abi, provider);
        todo!()
    }
}
