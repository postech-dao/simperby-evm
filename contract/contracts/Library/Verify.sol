// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./BytesLib.sol";
import "./Utils.sol";
import "./Strings.sol";
import "../Treasury/interfaces/IEVMTreasury.sol";

/**
 * @dev Functions to verify signature & decode bytes
 */
library Verify {
    using BytesLib for bytes;

    /**
     * @dev Bytes length for decoding data.
     * @notice Refer to https://github.com/bincode-org/bincode/blob/trunk/docs/spec.md for details.
     * @notice We need to remove first 1 bytes prefix from {pkLength}.
     */
    uint constant sigLength = 65;
    uint constant pkLength = 65;
    uint constant hashLength = 32;
    uint constant addressLength = 20;
    uint constant uint128Length = 16;
    uint constant strUint64Length = 8;
    uint constant enumLength = 4;

    struct TypedSignature {
        bytes signature;
        bytes signer;
    }

    struct ValidatorSet {
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
        ValidatorSet[] validators;
        bytes version;
    }

    /* ========== VERIFY FUNCTIONS ========== */
    /**
     * @dev Verifies new header is valid.
     * @param prevHeader Bytes of a previous header (Current lastHeader).
     * @param _prevBlockHeader Decoded BlockHeader of a previous header.
     * @param _blockHeader Decoded BlockHeader of a new BlockHeader.
     */
    function verifyHeaderToHeader(
        bytes memory prevHeader,
        BlockHeader memory _prevBlockHeader,
        BlockHeader memory _blockHeader
    ) internal pure {
        require(
            _prevBlockHeader.blockHeight + 1 == _blockHeader.blockHeight,
            "Verify::verifyHeaderToHeader: Invalid block height"
        );
        require(
            _blockHeader.previousHash == keccak256(prevHeader),
            "Verify::verifyHeaderToHeader: Invalid previous hash"
        );
        require(
            _blockHeader.timestamp >= _prevBlockHeader.timestamp,
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

    /**
     * @dev Verifies finalization proof with TypedSignature.
     * @param header Decoded header.
     * @param headerHash Keccak256 hashed header.
     * @param finalizationProof TypedSignatures of validators for header.
     */
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
            if (Utils.pkToAddress(finalizationProof[j].signer) == ecrecover(headerHash, v, r, s)) {
                require(
                    keccak256(finalizationProof[j].signer) ==
                        keccak256(header.validators[k].validator)
                );
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
            blockHeight >= heightOffset && blockHeight < heightOffset + commitRoots.length,
            "Verify::verifyTransactionCommitment: Invalid block height"
        );

        bytes32 root = commitRoots[blockHeight - heightOffset];
        bytes32 calculatedRoot = keccak256(transaction);

        uint offset = 0;
        uint64 lenOfProof = Utils.reverse64(merkleProof.slice(offset, strUint64Length).toUint64(0));
        offset += strUint64Length;

        for (uint i = 0; i < lenOfProof; i++) {
            uint32 enumOrder = Utils.reverse32(merkleProof.slice(offset, enumLength).toUint32(0));
            offset += enumLength;

            if (enumOrder == 0) {
                // Left child
                bytes32 leftPairHash = merkleProof.slice(offset, hashLength).toBytes32(0);
                calculatedRoot = keccak256(abi.encodePacked(leftPairHash, calculatedRoot));
                offset += hashLength;
            } else if (enumOrder == 1) {
                // Right child
                bytes32 rightPairHash = merkleProof.slice(offset, hashLength).toBytes32(0);
                calculatedRoot = keccak256(abi.encodePacked(calculatedRoot, rightPairHash));
                offset += hashLength;
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
            len == (input.length - 8) / 130 && (input.length - 8) % 130 == 0,
            "Verify::parseProof: Invalid proof length"
        );

        TypedSignature[] memory fp = new TypedSignature[](len);

        uint offset = strUint64Length;

        for (uint256 i = 0; i < len; i++) {
            fp[i] = TypedSignature(
                input.slice(offset, sigLength),
                input.slice(offset + sigLength + 1, pkLength - 1)
            );
            offset += (sigLength + pkLength);
        }

        return fp;
    }

    function parseHeader(
        bytes memory hexEncodedData
    ) internal pure returns (BlockHeader memory blockHeader) {
        uint offset = 0;

        blockHeader.author = hexEncodedData.slice(offset + 1, pkLength - 1);
        offset += pkLength;

        {
            uint64 len = Utils.reverse64(hexEncodedData.slice(offset, strUint64Length).toUint64(0));
            offset += strUint64Length;
            blockHeader.prevBlockFinalizationProof = new TypedSignature[](len);

            bytes memory _sig;
            bytes memory _signer;

            if (len != 0) {
                for (uint i = 0; i < len; i++) {
                    _sig = hexEncodedData.slice(offset, sigLength);
                    offset += sigLength;
                    _signer = hexEncodedData.slice(offset + 1, pkLength - 1);
                    offset += pkLength;

                    blockHeader.prevBlockFinalizationProof[i] = TypedSignature(_sig, _signer);
                }
            }
        }

        blockHeader.previousHash = hexEncodedData.slice(offset, hashLength).toBytes32(0);
        offset += hashLength;

        blockHeader.blockHeight = Utils.reverse64(
            hexEncodedData.slice(offset, strUint64Length).toUint64(0)
        );
        offset += strUint64Length;

        blockHeader.timestamp = int64(
            Utils.reverse64(hexEncodedData.slice(offset, strUint64Length).toUint64(0))
        );
        offset += strUint64Length;

        blockHeader.commitMerkleRoot = hexEncodedData.slice(offset, hashLength).toBytes32(0);
        offset += hashLength;

        // Skip repository root (32 bytes)
        offset += hashLength;

        {
            uint64 validatorsLen = Utils.reverse64(
                hexEncodedData.slice(offset, strUint64Length).toUint64(0)
            );
            offset += strUint64Length;
            blockHeader.validators = new ValidatorSet[](validatorsLen);

            bytes memory _validator;
            uint64 _votingPower;

            for (uint i = 0; i < validatorsLen; i++) {
                _validator = hexEncodedData.slice(offset + 1, pkLength - 1);
                offset += pkLength;
                _votingPower = Utils.reverse64(
                    hexEncodedData.slice(offset, strUint64Length).toUint64(0)
                );
                offset += strUint64Length;

                blockHeader.validators[i] = ValidatorSet(_validator, _votingPower);
            }
        }

        // length of version is always 5, so ignore it.
        blockHeader.version = hexEncodedData.slice(offset + strUint64Length, 5);
    }

    function parseFTExecution(
        bytes memory execution
    ) internal pure returns (IEVMTreasury.FungibleTokenTransfer memory fungibleTokenTransfer) {
        uint offset;

        uint64 lenOfChain = Utils.reverse64(execution.slice(0, strUint64Length).toUint64(0));
        offset += strUint64Length;

        fungibleTokenTransfer.chain = execution.slice(offset, lenOfChain);
        offset += lenOfChain;

        fungibleTokenTransfer.contractSequence = Utils.reverse128(
            execution.slice(offset, uint128Length).toUint128(0)
        );
        offset += uint128Length;

        // Skip decoding enum since we already know the type of execution
        offset += enumLength;

        fungibleTokenTransfer.tokenAddress = execution.slice(offset, addressLength).toAddress(0);
        offset += addressLength;

        fungibleTokenTransfer.amount = Utils.reverse128(
            execution.slice(offset, uint128Length).toUint128(0)
        );
        offset += uint128Length;

        fungibleTokenTransfer.receiverAddress = execution.slice(offset, addressLength).toAddress(0);
    }

    function parseNFTExecution(
        bytes memory execution
    )
        internal
        pure
        returns (IEVMTreasury.NonFungibleTokenTransfer memory nonFungibleTokenTransfer)
    {
        uint offset;

        uint64 lenOfChain = Utils.reverse64(execution.slice(0, strUint64Length).toUint64(0));
        offset += strUint64Length;

        nonFungibleTokenTransfer.chain = execution.slice(offset, lenOfChain);
        offset += lenOfChain;

        nonFungibleTokenTransfer.contractSequence = Utils.reverse128(
            execution.slice(offset, uint128Length).toUint128(0)
        );
        offset += uint128Length;

        // Skip decoding enum since we already know the type of execution
        offset += enumLength;

        nonFungibleTokenTransfer.collectionAddress = execution
            .slice(offset, addressLength)
            .toAddress(0);
        offset += addressLength;

        nonFungibleTokenTransfer.tokenId = Utils.reverse128(
            execution.slice(offset, uint128Length).toUint128(0)
        );
        offset += uint128Length;

        nonFungibleTokenTransfer.receiverAddress = execution.slice(offset, addressLength).toAddress(
            0
        );
    }
}
