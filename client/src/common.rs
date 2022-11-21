use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    // TODO
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct BlockFinalizationProof {
    // TODO
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MerkleProof {
    // TODO
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct FungibleTokenTransfer {
    pub token_id: String,
    pub amount: u128,
    pub receiver_address: String,
    pub contract_sequence: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct NonFungibleTokenTransfer {
    pub collection_address: String,
    pub token_index: String,
    pub receiver_address: String,
    pub contract_sequence: u64,
}
