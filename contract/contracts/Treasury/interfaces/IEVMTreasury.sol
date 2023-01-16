// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IEVMTreasury {
    enum DeliverableMessage {
        FungibleTokenTransfer,
        NonFungibleTokenTransfer,
        Custom
    }

    struct FungibleTokenTransfer {
        address tokenAddress;
        uint256 amount;
        address receiverAddress;
        uint256 contractSequence;
    }

    struct NonFungibleTokenTransfer {
        address collectionAddress;
        uint256 tokenIndex;
        address receiverAddress;
        uint256 contractSequence;
    }

    struct Custom {
        string message;
        uint256 contractSequence;
    }

    struct Client {
        uint256 height;
        bytes lastHeader;
        string chainName;
    }

    function transferToken(
        DeliverableMessage _message,
        bytes memory _data,
        uint256 height,
        string memory merkleProof
    ) external;

    function updateLightClient(bytes memory header, bytes[] memory proof) external;

    function verifyTransactionCommitment(
        DeliverableMessage message,
        uint256 height,
        string memory MerkleProof
    ) external returns (bool);
}
