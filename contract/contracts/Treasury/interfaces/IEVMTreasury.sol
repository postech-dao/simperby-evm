// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IEVMTreasury {
    struct FungibleTokenTransfer {
        uint128 contractSequence;
        uint128 amount;
        bytes chain;
        address tokenAddress;
        address receiverAddress;
    }

    struct NonFungibleTokenTransfer {
        uint128 contractSequence;
        uint128 tokenId;
        bytes chain;
        address collectionAddress;
        address receiverAddress;
    }

    struct LightClient {
        uint64 heightOffset;
        bytes lastHeader;
        bytes32[] repositoryRoots;
        bytes32[] commitRoots;
    }

    function execute(
        bytes memory transaction,
        bytes memory executionHash,
        uint64 blockHeight,
        bytes memory merkleProof
    ) external;

    function updateLightClient(bytes memory header, bytes memory proof) external;
}
