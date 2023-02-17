// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "../Library/Verify.sol";
import "../Library/BytesLib.sol";
import "../Library/Utils.sol";
import "./interfaces/IEVMTreasury.sol";

contract EVMTreasury is Pausable, ReentrancyGuard, IEVMTreasury {
    using BytesLib for bytes;

    /// @notice The name of this contract
    string public constant name = "EVM SETTLEMENT CHAIN TREASURY V1";
    bytes public constant chainName = hex"6d797468657265756d"; // mythereum, for testing
    uint128 public constant contractSequence = 0;

    LightClient public lightClient;

    mapping(uint256 => LightClient) public lightClients;
    /* ========== EVENTS ========== */

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

    event UpdateLightClient(bytes lastHeader);

    /* ========== CONSTRUCTOR ========== */
    constructor(bytes memory initialHeader) {
        Verify.BlockHeader memory _blockHeader = Verify.parseHeader(initialHeader);

        bytes32[] memory repositoryRoots = new bytes32[](1);
        bytes32[] memory commitRoots = new bytes32[](1);
        repositoryRoots[0] = _blockHeader.repositoryMerkleRoot;
        commitRoots[0] = _blockHeader.commitMerkleRoot;

        lightClient = LightClient(
            _blockHeader.blockHeight,
            initialHeader,
            repositoryRoots,
            commitRoots
        );
        lightClients[0] = lightClient;
    }

    /* ========== TREASURY FUNCTIONS ========== */
    /**
     * @dev Functions to execute transactions.
     * @param transaction The transaction to be executed.
     * @param blockHeight The block height of the transaction.
     * @param merkleProof The merkle proof of the transaction.
     */
    function execute(
        bytes memory transaction,
        bytes memory executionHash,
        uint64 blockHeight,
        bytes memory merkleProof
    ) public whenNotPaused nonReentrant {
        bytes memory hashOfExecution = Utils.fromHex(
            Utils.bytesToString(transaction.slice(transaction.length - 68, 64))
        );
        require(
            bytes32(hashOfExecution) == keccak256(executionHash),
            "EVMTreasury::execute: Invalid execution hash"
        );

        uint64 lengthOfHeader = Utils.reverse64(transaction.slice(73, 8).toUint64(0));
        if (lengthOfHeader == 25) {
            FungibleTokenTransfer memory fungibleTokenTransfer = Verify.parseFTExecution(
                executionHash
            );
            require(
                fungibleTokenTransfer.contractSequence == contractSequence,
                "EVMTreasury::execute: Invalid contract sequence"
            );
            require(
                keccak256(fungibleTokenTransfer.chain) == keccak256(chainName),
                "EVMTreasury::execute: Invalid chain"
            );

            Verify.verifyTransactionCommitment(
                transaction,
                lightClient.commitRoots,
                merkleProof,
                blockHeight,
                lightClient.heightOffset
            );

            if (fungibleTokenTransfer.tokenAddress == address(0)) {
                withdrawETH(fungibleTokenTransfer.receiverAddress, fungibleTokenTransfer.amount);
            } else {
                withdrawERC20(
                    fungibleTokenTransfer.tokenAddress,
                    fungibleTokenTransfer.receiverAddress,
                    fungibleTokenTransfer.amount
                );
            }
        } else if (lengthOfHeader == 26) {
            NonFungibleTokenTransfer memory nonFungibleTokenTransfer = Verify.parseNFTExecution(
                executionHash
            );
            require(
                nonFungibleTokenTransfer.contractSequence == contractSequence,
                "EVMTreasury::execute: Invalid contract sequence"
            );
            require(
                keccak256(nonFungibleTokenTransfer.chain) == keccak256(chainName),
                "EVMTreasury::execute: Invalid chain"
            );

            Verify.verifyTransactionCommitment(
                transaction,
                lightClient.commitRoots,
                merkleProof,
                blockHeight,
                lightClient.heightOffset
            );

            withdrawERC721(
                nonFungibleTokenTransfer.collectionAddress,
                nonFungibleTokenTransfer.receiverAddress,
                nonFungibleTokenTransfer.tokenId
            );
        } else {
            revert("Invalid transaction header");
        }
    }

    function withdrawETH(address to, uint256 amount) internal {
        require(address(this).balance >= amount, "EVMTreasury::withdrawETH: Insufficient balance");
        emit TransferFungibleToken(address(0), amount, to, 0);

        payable(to).transfer(amount);
    }

    function withdrawERC20(address token, address to, uint256 amount) internal {
        require(
            IERC20(token).balanceOf(address(this)) >= amount,
            "EVMTreasury::withdrawERC20: Insufficient balance"
        );
        IERC20(token).transfer(to, amount);

        emit TransferFungibleToken(token, amount, to, 0);
    }

    function withdrawERC721(address token, address to, uint256 tokenId) internal {
        require(
            IERC721(token).ownerOf(tokenId) == address(this),
            "EVMTreasury::withdrawERC721: Insufficient balance"
        );
        IERC721(token).transferFrom(address(this), to, tokenId);

        emit TransferNonFungibleToken(token, tokenId, to, 0);
    }

    /* ========== LIGHTCLIENT FUNCTIONS ========== */
    /**
     * @dev Functions to update light client.
     * @param header The header to be updated.
     * @param proof The finalization proof of the header.
     */
    function updateLightClient(bytes memory header, bytes calldata proof) public whenNotPaused {
        Verify.BlockHeader memory _blockHeader = Verify.parseHeader(header);
        Verify.TypedSignature[] memory _proof = Verify.parseProof(proof);

        Verify.verifyHeaderToHeader(lightClient.lastHeader, header);
        Verify.verifyFinalizationProof(_blockHeader, keccak256(header), _proof);

        lightClient.lastHeader = header;
        lightClient.repositoryRoots.push(_blockHeader.repositoryMerkleRoot);
        lightClient.commitRoots.push(_blockHeader.commitMerkleRoot);

        lightClients[_blockHeader.blockHeight] = LightClient(
            lightClient.heightOffset,
            header,
            lightClient.repositoryRoots,
            lightClient.commitRoots
        );

        emit UpdateLightClient(lightClient.lastHeader);
    }
}
