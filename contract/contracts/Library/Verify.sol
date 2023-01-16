// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @dev Functions to verify signature
 */
library Verify {
    /// @dev Since nested mapping makes struct and logic too complicated, we use a simple array to store validator set.
    struct BlockHeader {
        bytes author;
        bytes[] prevBlockFinalizationProof;
        bytes32 previousHash;
        uint64 blockHeight;
        int64 timestamp;
        bytes32 commitMerkleRoot;
        bytes32 repositoryMerkleRoot;
        bytes[] validators;
        uint64[] votingPowers;
        string version;
    }

    function verifyHeaderToHeader(
        bytes memory prevHeader,
        bytes memory header
    ) internal pure returns (bool) {
        BlockHeader memory _prevBlockHeader = parseHeader(prevHeader);
        BlockHeader memory _blockHeader = parseHeader(header);
        require(
            _prevBlockHeader.blockHeight + 1 == _blockHeader.blockHeight,
            "Verify::verifyHeaderToHeader: Invalid block height"
        );
        require(
            _blockHeader.previousHash == keccak256(prevHeader),
            "Verify::verifyHeaderToHeader: Invalid previous hash"
        );
        require(
            _blockHeader.timestamp > _prevBlockHeader.timestamp,
            "Verify::verifyHeaderToHeader: Invalid block timestamp"
        );

        for (uint i = 0; i < _prevBlockHeader.validators.length; i++) {
            if (keccak256(_prevBlockHeader.validators[i]) == keccak256(_blockHeader.author)) {
                break;
            } else {
                if (i == _prevBlockHeader.validators.length - 1) {
                    revert("Verify::verifyHeaderToHeader: Invalid block author");
                }
            }
        }

        require(
            verifyFinalizationProof(
                _prevBlockHeader,
                _blockHeader.previousHash,
                _blockHeader.prevBlockFinalizationProof
            ),
            "Verify::verifyHeaderToHeader: Invalid finalization proof"
        );

        return true;
    }

    function verifyFinalizationProof(
        BlockHeader memory header,
        bytes32 headerHash,
        bytes[] memory finalizationProof
    ) internal pure returns (bool) {
        uint64 _totalVotingPower;
        uint64 _votedVotingPower;
        for (uint i = 0; i < header.validators.length; i++) {
            _totalVotingPower += header.votingPowers[i];
        }
        uint k = 0;
        for (uint j = 0; j < finalizationProof.length; j++) {
            (bytes memory signer, bytes memory signature) = abi.decode(
                finalizationProof[j],
                (bytes, bytes)
            );
            (bytes32 r, bytes32 s, uint8 v) = splitSignature(signature);
            if (pubToAddress(signer) == ecrecover(toPrefixedHash(headerHash), v, r, s)) {
                _votedVotingPower += header.votingPowers[k];
            }
            k++;
        }

        require(
            _votedVotingPower * 3 > _totalVotingPower * 2,
            "Verify::verifyFinalizationProof: Not enough voting power"
        );
        return true;
    }

    function splitSignature(
        bytes memory signature
    ) public pure returns (bytes32 r, bytes32 s, uint8 v) {
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

    /// @notice The ```parseHeader``` function is used to decode bytes to BlockHeader struct
    /// @dev Since we can't directly decode dynamic array in solidity, we need to decode it manually (Tricky way)
    function parseHeader(
        bytes memory header
    ) internal pure returns (BlockHeader memory blockHeader) {
        (
            bytes memory author,
            bytes[] memory prevBlockFinalizationProof,
            bytes32 previousHash,
            uint64 blockHeight,
            int64 timestamp,
            bytes32 commitMerkleRoot,
            bytes32 repositoryMerkleRoot,
            bytes[] memory validators,
            uint64[] memory votingPowers,
            string memory version
        ) = abi.decode(
                header,
                (
                    bytes,
                    bytes[],
                    bytes32,
                    uint64,
                    int64,
                    bytes32,
                    bytes32,
                    bytes[],
                    uint64[],
                    string
                )
            );

        blockHeader = BlockHeader({
            author: author,
            prevBlockFinalizationProof: prevBlockFinalizationProof,
            previousHash: previousHash,
            blockHeight: blockHeight,
            timestamp: timestamp,
            commitMerkleRoot: commitMerkleRoot,
            repositoryMerkleRoot: repositoryMerkleRoot,
            validators: validators,
            votingPowers: votingPowers,
            version: version
        });
    }

    function toPrefixedHash(bytes32 hash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", hash));
    }

    function pubToAddress(bytes memory pk) internal pure returns (address) {
        bytes32 _hash = keccak256(pk);

        return address(uint160(uint256(_hash)));
    }
}
