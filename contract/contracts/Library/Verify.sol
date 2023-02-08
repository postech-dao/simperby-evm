// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./BytesLib.sol";
import "./Utils.sol";
import "../Treasury/interfaces/IEVMTreasury.sol";

/**
 * @dev Functions to verify signature & decode bytes
 * @notice For public key recovery, we need to change to uncompressed format. (65 bytes)
 */
library Verify {
    using BytesLib for bytes;

    struct TypedSignature {
        bytes signature;
        bytes signer;
    }

    struct validatorSet {
        bytes validator;
        uint64 votingPower;
    }

    struct BlockHeader {
        bytes author;
        TypedSignature[] prevBlockFinalizationProof;
        bytes32 previousHash;
        uint64 blockHeight;
        int64 timestamp;
        bytes32 commitMerkleRoot;
        bytes32 repositoryMerkleRoot;
        validatorSet[] validators;
        bytes version;
    }

    /* ========== VERIFY FUNCTIONS ========== */
    function verifyHeaderToHeader(bytes memory prevHeader, bytes memory header) internal pure {
        BlockHeader memory _prevBlockHeader = parseHeader(prevHeader);
        BlockHeader memory _blockHeader = parseHeader(header);
        require(
            _prevBlockHeader.blockHeight + 1 == _blockHeader.blockHeight,
            "Verify::verifyHeaderToHeader: Invalid block height"
        );
        require(
            _blockHeader.previousHash == Utils.hashHeader(prevHeader),
            "Verify::verifyHeaderToHeader: Invalid previous hash"
        );
        require(
            _blockHeader.timestamp > _prevBlockHeader.timestamp,
            "Verify::verifyHeaderToHeader: Invalid block timestamp"
        );

        for (uint i = 0; i < _prevBlockHeader.validators.length; i++) {
            if (
                keccak256(_prevBlockHeader.validators[i].validator) ==
                keccak256(_blockHeader.author)
            ) {
                break;
            } else {
                if (i == _prevBlockHeader.validators.length - 1) {
                    revert("Verify::verifyHeaderToHeader: Invalid block author");
                }
            }
        }

        verifyFinalizationProof(
            _prevBlockHeader,
            _blockHeader.previousHash,
            _blockHeader.prevBlockFinalizationProof
        );
    }

    function verifyFinalizationProof(
        BlockHeader memory header,
        bytes32 headerHash,
        TypedSignature[] memory finalizationProof
    ) internal pure {
        uint256 _totalVotingPower;
        uint256 _votedVotingPower;
        for (uint i = 0; i < header.validators.length; i++) {
            _totalVotingPower += header.validators[i].votingPower;
        }
        uint k = 0;
        for (uint j = 0; j < finalizationProof.length; j++) {
            (bytes32 r, bytes32 s, uint8 v) = splitSignature(finalizationProof[j].signature);
            if (
                Utils.pubToAddress(finalizationProof[j].signer) ==
                ecrecover(Utils.toPrefixedHash(headerHash), v, r, s)
            ) {
                _votedVotingPower += header.validators[k].votingPower;
            }
            k++;
        }

        require(
            _votedVotingPower * 3 > _totalVotingPower * 2,
            "Verify::verifyFinalizationProof: Not enough voting power"
        );
    }

    function verifyTransactionCommitment(
        bytes memory transaction,
        bytes32[] memory commitRoots,
        bytes memory merkleProof,
        uint64 blockHeight,
        uint64 heightOffset
    ) internal pure {
        require(
            blockHeight < heightOffset || blockHeight >= heightOffset + commitRoots.length,
            "Verify::verifyTransactionCommitment: Invalid block height"
        );

        bytes32 root = commitRoots[blockHeight - heightOffset];
        bytes32 calculatedRoot = keccak256(transaction);

        uint256 offset = 0;
        uint64 lenOfProof = Utils.reverse64(merkleProof.slice(offset, 8).toUint64(0));
        offset += 8;

        for (uint i = 0; i < lenOfProof; i++) {
            uint64 enumOrder = Utils.reverse64(merkleProof.slice(offset, 4).toUint64(0));
            offset += 4;

            if (enumOrder == 1) {
                // Left child
                bytes32 leftPairHash = merkleProof.slice(offset, 32).toBytes32(0);
                calculatedRoot = keccak256(abi.encodePacked(leftPairHash, calculatedRoot));
                offset += 32;
            } else if (enumOrder == 2) {
                // Right child
                bytes32 rightPairHash = merkleProof.slice(offset, 32).toBytes32(0);
                calculatedRoot = keccak256(abi.encodePacked(calculatedRoot, rightPairHash));
                offset += 32;
            } else {
                revert("Invalid enum order in merkle proof");
            }
        }

        require(
            root == calculatedRoot,
            "Verify::verifyTransactionCommitment: Merkle proof verification fail"
        );
    }

    /* ========== DECODER ========== */
    function splitSignature(
        bytes memory signature
    ) internal pure returns (bytes32 r, bytes32 s, uint8 v) {
        require(signature.length == 65, "invalid signature length");

        assembly {
            // first 32 bytes, after the length prefix
            r := mload(add(signature, 32))
            // second 32 bytes
            s := mload(add(signature, 64))
            // final byte (first byte of the next 32 bytes)
            v := byte(0, mload(add(signature, 96)))
        }
    }

    function parseProof(bytes memory input) internal pure returns (TypedSignature[] memory) {
        uint64 len = Utils.reverse64(input.slice(0, 8).toUint64(0));
        require(
            len == (input.length - 8) / 98 && (input.length - 8) % 98 == 0,
            "Verify::parseProof: Invalid proof length"
        );

        TypedSignature[] memory fp = new TypedSignature[](len);

        uint256 offset = 8;

        for (uint256 i = 0; i < len; i++) {
            fp[i] = TypedSignature(input.slice(offset, 65), input.slice(offset + 65, 33));
            offset += 98;
        }

        return fp;
    }

    function parseHeader(
        bytes memory hexEncodedData
    ) internal pure returns (BlockHeader memory blockHeader) {
        uint offset = 0;

        blockHeader.author = hexEncodedData.slice(offset + 1, 33);
        offset += 33;

        {
            uint64 len = Utils.reverse64(hexEncodedData.slice(offset, 8).toUint64(0));
            offset += 8;
            blockHeader.prevBlockFinalizationProof = new TypedSignature[](len);

            bytes memory sig_;
            bytes memory signer_;

            if (len != 0) {
                for (uint i = 0; i < len; i++) {
                    sig_ = hexEncodedData.slice(offset, 65);
                    offset += 65;
                    signer_ = hexEncodedData.slice(offset + 1, 33);
                    offset += 33;

                    blockHeader.prevBlockFinalizationProof[i] = TypedSignature(sig_, signer_);
                }
            }
        }

        blockHeader.previousHash = hexEncodedData.slice(offset, 32).toBytes32(0);
        offset += 32;

        blockHeader.blockHeight = Utils.reverse64(hexEncodedData.slice(offset, 8).toUint64(0));
        offset += 8;

        blockHeader.timestamp = int64(Utils.reverse64(hexEncodedData.slice(offset, 8).toUint64(0)));
        offset += 8;

        blockHeader.commitMerkleRoot = hexEncodedData.slice(offset, 32).toBytes32(0);
        offset += 32;

        blockHeader.repositoryMerkleRoot = hexEncodedData.slice(offset, 32).toBytes32(0);
        offset += 32;

        {
            uint64 validatorsLen = Utils.reverse64(hexEncodedData.slice(offset, 8).toUint64(0));
            offset += 8;
            blockHeader.validators = new validatorSet[](validatorsLen);

            bytes memory validator_;
            uint64 votingPower_;

            for (uint i = 0; i < validatorsLen; i++) {
                validator_ = hexEncodedData.slice(offset + 1, 33);
                offset += 33;
                votingPower_ = Utils.reverse64(hexEncodedData.slice(offset, 8).toUint64(0));
                offset += 8;

                blockHeader.validators[i] = validatorSet(validator_, votingPower_);
            }
        }

        // length of version is always 5, so ignore it.
        blockHeader.version = hexEncodedData.slice(offset + 8, 5);
    }

    function parseFTTransaction(
        bytes memory transaction
    ) internal pure returns (IEVMTreasury.FungibleTokenTransfer memory fungibleTokenTransfer) {
        uint256 offset = 33;

        fungibleTokenTransfer.timestamp = int64(
            Utils.reverse64(transaction.slice(offset, 8).toUint64(0))
        );
        offset += 8;
        offset += 25;

        uint64 lenOfChain = Utils.reverse64(transaction.slice(offset, 8).toUint64(0));
        offset += 8;

        fungibleTokenTransfer.chain = transaction.slice(offset, lenOfChain);
        offset += lenOfChain;

        fungibleTokenTransfer.tokenAddress = transaction.slice(offset + 8, 20).toAddress(0);
        offset += 28;

        fungibleTokenTransfer.amount = Utils.reverse128(transaction.slice(offset, 16).toUint128(0));
        offset += 16;

        fungibleTokenTransfer.receiverAddress = transaction.slice(offset + 8, 20).toAddress(0);
    }

    function parseNFTTransaction(
        bytes memory transaction
    )
        internal
        pure
        returns (IEVMTreasury.NonFungibleTokenTransfer memory nonFungibleTokenTransfer)
    {
        uint256 offset = 33;

        nonFungibleTokenTransfer.timestamp = int64(
            Utils.reverse64(transaction.slice(offset, 8).toUint64(0))
        );
        offset += 8;
        offset += 25;

        uint64 lenOfChain = Utils.reverse64(transaction.slice(offset, 8).toUint64(0));
        offset += 8;

        nonFungibleTokenTransfer.chain = transaction.slice(offset, lenOfChain);
        offset += lenOfChain;

        nonFungibleTokenTransfer.collectionAddress = transaction.slice(offset + 8, 20).toAddress(0);
        offset += 28;

        uint64 lenOfTokenId = Utils.reverse64(transaction.slice(offset, 8).toUint64(0));
        offset += 8;

        nonFungibleTokenTransfer.tokenId = uint128(
            Utils.str2num(Utils.bytesToString(transaction.slice(offset, lenOfTokenId)))
        );
        offset += lenOfTokenId;

        nonFungibleTokenTransfer.receiverAddress = transaction.slice(offset + 8, 20).toAddress(0);
    }
}
