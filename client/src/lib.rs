use async_trait::async_trait;
use eyre::Error;
use merkle_tree::MerkleProof;
use rust_decimal::Decimal;
use simperby_core::*;
use simperby_settlement::*;

pub struct Ethereum {}

#[async_trait]
impl SettlementChain for Ethereum {
    async fn get_chain_name(&self) -> String {
        todo!()
    }

    async fn check_connection(&self) -> Result<(), Error> {
        todo!()
    }

    async fn get_last_block(&self) -> Result<SettlementChainBlock, Error> {
        todo!()
    }

    async fn get_contract_sequence(&self) -> Result<u128, Error> {
        todo!()
    }

    async fn get_relayer_account_info(&self) -> Result<(HexSerializedVec, Decimal), Error> {
        todo!()
    }

    async fn get_light_client_header(&self) -> Result<BlockHeader, Error> {
        todo!()
    }

    async fn get_treasury_fungible_token_balance(
        &self,
        _address: HexSerializedVec,
    ) -> Result<Decimal, Error> {
        todo!()
    }

    async fn get_treasury_non_fungible_token_balance(
        &self,
        _address: HexSerializedVec,
    ) -> Result<Vec<HexSerializedVec>, Error> {
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
        _transaction: Transaction,
        _block_height: u64,
        _proof: MerkleProof,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn eoa_get_sequence(&self, address: HexSerializedVec) -> Result<u128, Error> {
        todo!()
    }

    async fn eoa_get_fungible_token_balance(
        &self,
        address: HexSerializedVec,
        token_address: HexSerializedVec,
    ) -> Result<Decimal, Error> {
        todo!()
    }

    async fn eoa_transfer_fungible_token(
        &self,
        address: HexSerializedVec,
        sender_private_key: HexSerializedVec,
        token_address: HexSerializedVec,
        receiver_address: HexSerializedVec,
        amount: Decimal,
    ) -> Result<(), Error> {
        todo!()
    }
}
