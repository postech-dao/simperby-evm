/// A temporary module that will be replaced by the real one
/// exported by the Simperby main repository.
pub mod common;

use common::*;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
/// Information of a contract.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ContractInfo {
    /// The address of the contract.
    pub address: String,
    /// The increasing sequence that is for preventing the replay attack.
    ///
    /// A valid message from PBC MUST carry the same number with this,
    /// in order to succesfully convince the contract. (i.e., this number is something that the consensus should have finalized on-chain).
    ///
    /// - Note1: this is totally irrelevant to the account sequence.
    /// - Note2: the light client update operation is not related to this
    /// because the 'block height' provides the same safety guard.
    pub sequence: u64,
}

/// An error that can occur when interacting with the contract.
#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum Error {
    /// When there is a problem to access to the full node.
    #[error("connection error: {0}")]
    ConnectionError(String),
    /// When the transaction is rejected by the full node, before it gets to the contract.
    #[error("transaction rejected: {0}")]
    TransactionRejected(String),
    /// When the contract fails to decode the input data.
    #[error("failed to parse the payload of the transaction")]
    FailedToParseTransactionPayload,
    /// When the given proof is invalid.
    #[error("invalid proof given: got merkle root of {0} but expected {1}")]
    InvalidProof(String, String),
    /// When the given message is well decoded and verified, but the message argument is invalid.
    #[error("invalid message argument given: {0}")]
    InvalidMessageArgument(String),
    /// When the relayer account has not enough balance to execute the transaction.
    #[error("not enough balance: got {0}")]
    NotEnoughBalance(u64),
    /// When the account sequence given in the transaction is invalid.
    #[error("invalid account sequence; expected {0} but got {1}")]
    InvalidAccountSequence(u64, u64),
    /// When the contract sequence given in the transaction is invalid.
    #[error("invalid contract sequence; expected {0} but got {1}")]
    InvalidContractSequence(u64, u64),
    /// When the contract fails to execute the transaction with its own error.
    #[error("internal contract error: {0}")]
    InternalContractError(String),
    /// When the contract is missing.
    #[error("no such contract: {0}")]
    NoSuchContract(String),
    /// An unknown error.
    #[error("unknown error: {0}")]
    Unknown(String),
}

/// An abstract information about a block.
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    /// The height of the block.
    pub height: u64,
    /// The UNIX timestamp of the block in seconds.
    pub timestamp: u64,
}

/// An abstraction of the residential chain with its treasury deployed on it.
///
/// One trivial implementation of this trait would carry the API endpoint of the full node and
/// the relayer account used to submit message delivering transactions.
#[async_trait::async_trait]
pub trait ResidentialChain: Send + Sync {
    /// Returns the name of the chain.
    async fn get_chain_name(&self) -> String;

    /// Checks whether the chain is healthy and the full node is running.
    async fn check_connection(&self) -> Result<(), Error>;

    /// Getes the latest finalized block on the chain.
    async fn get_last_block(&self) -> Result<Block, Error>;

    /// Returns the address and the current balance (which is used to pay the gas fee) of the relayer account in this chain.
    ///
    /// Note that there is no authority of the relayer account; it is just a convenient account to pay the gas fee
    /// (i.e., there is no need to check the transaction submitter by the contract).
    async fn get_relayer_account_info(&self) -> Result<(String, Decimal), Error>;

    /// Returns the latest header that the light client has verified.
    async fn get_light_client_header(&self) -> Result<Header, Error>;

    /// Returns the current balance of a particular fungible token in the treasury contract.
    async fn get_treasury_fungible_token_balance(&self, address: String) -> Result<Decimal, Error>;

    /// Returns the current balance of all fungible tokens in the treasury contract.
    async fn get_treasury_all_fungible_token_balance(
        &self,
    ) -> Result<HashMap<String, Decimal>, Error>;

    /// Returns the current balance of a particular non-fungible token collection in the treasury contract,
    /// identified as their token indices.
    async fn get_treasury_non_fungible_token_balance(
        &self,
        address: Vec<String>,
    ) -> Result<String, Error>;

    /// Returns the current balance of all non-fungible tokens in the treasury contract,
    /// identified as `(collection address, token index)`.
    async fn get_treasury_all_non_fungible_token_balance(
        &self,
    ) -> Result<Vec<(String, String)>, Error>;

    /// Updates the light client state in the treasury by providing the next, valid block header and its proof.
    ///
    /// This is one of the message delivery methods; a transaction that carries the given data will be submitted to the chain.
    async fn update_treasury_light_client(
        &self,
        header: Header,
        proof: BlockFinalizationProof,
    ) -> Result<(), Error>;

    /// Transfers a given amount of fungible tokens from the treasury contract to the destination address.
    ///
    /// This is one of the message delivery methods; a transaction that carries the given data and the proof will be submitted to the chain.
    async fn transfer_treasury_fungible_token(
        &self,
        message: FungibleTokenTransfer,
        block_height: u64,
        proof: MerkleProof,
    ) -> Result<(), Error>;

    /// Transfers an NFT from the treasury contract to the destination address.
    ///
    /// This is one of the message methods; a transaction that carries the given data and the proof will be submitted to the chain.
    async fn transfer_treasury_non_fungible_token(
        &self,
        message: NonFungibleTokenTransfer,
        block_height: u64,
        proof: MerkleProof,
    ) -> Result<(), Error>;
}
