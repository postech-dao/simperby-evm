// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IEVMTreasury {
    event TransferFungibleToken(
        address indexed tokenAddress,
        uint256 amount,
        address indexed receiverAddress,
        uint256 contractSequence
    );

    event TransferNonFungibleToken(
        address indexed tokenAddress,
        uint256 tokenIndex,
        address indexed receiverAddress,
        uint256 contractSequence
    );

    event UpdateLightClient(uint256 indexed height, bytes indexed lastHeader);

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
        bytes32[] commitRoots;
    }

    function execute(
        bytes memory transaction,
        bytes memory executionHash,
        uint64 blockHeight,
        bytes memory merkleProof
    ) external;

    function updateLightClient(bytes memory header, bytes memory proof) external;

    function lightClient() external view returns (uint64 heightOffset, bytes memory lastHeader);

    function viewCommitRoots() external view returns (bytes32[] memory commitRoots);
}
