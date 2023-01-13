// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IEVMTreasury {
    enum DeliverableMessage {
        FungibleTokenTransfer,
        NonFungibleTokenTransfer,
        Custom
    }

    struct FungibleTokenTransfer {
        address token_address;
        uint256 amount;
        address receiver_address;
        uint256 contract_sequence;
    }

    struct NonFungibleTokenTransfer {
        address collection_address;
        uint256 token_index;
        address receiver_address;
        uint256 contract_sequence;
    }

    struct Custom {
        string message;
        uint256 contract_sequence;
    }

    struct Client {
        uint256 height;
        bytes last_header;
        string chain_name;
    }

    function transfer_token(
        DeliverableMessage _message,
        bytes memory _data,
        uint256 height,
        string memory merkleProof
    ) external;

    function update_light_client(bytes memory header, bytes[] memory proof) external;

    function verify_transaction_commitment(
        DeliverableMessage message,
        uint256 height,
        string memory MerkleProof
    ) external returns (bool);
}
