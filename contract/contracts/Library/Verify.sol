// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @dev Functions to verify signature
 */
library Verify {
    /// @dev Since nested mapping makes struct and logic too complicated, we use a simple array to store validator set.
    struct BlockHeader {
        bytes author;
        bytes[] prev_block_finalization_proof;
        bytes32 previous_hash;
        uint64 block_height;
        int64 timestamp;
        bytes32 commit_merkle_root;
        bytes32 repository_merkle_root;
        bytes[] validators;
        uint64[] voting_power;
        bytes32 version;
    }

    function verify_header_to_header(
        bytes memory prev_header,
        bytes memory header
    ) internal pure returns (bool) {
        BlockHeader memory prev_block_header = parse_header(prev_header);
        BlockHeader memory block_header = parse_header(header);
        require(
            prev_block_header.block_height + 1 == block_header.block_height,
            "Verify::verify_header_to_header: Invalid block height"
        );
        require(
            block_header.previous_hash == keccak256(prev_header),
            "Verify::verify_header_to_header: Invalid previous hash"
        );
        require(
            block_header.timestamp > prev_block_header.timestamp,
            "Verify::verify_header_to_header: Invalid block timestamp"
        );

        for (uint i = 0; i < prev_block_header.validators.length; i++) {
            if (keccak256(prev_block_header.validators[i]) == keccak256(block_header.author)) {
                break;
            } else {
                if (i == prev_block_header.validators.length - 1) {
                    return false;
                }
            }
        }

        require(
            verify_finalization_proof(
                prev_block_header,
                block_header.previous_hash,
                block_header.prev_block_finalization_proof
            ),
            "Verify::verify_header_to_header: Invalid finalization proof"
        );

        return true;
    }

    function verify_finalization_proof(
        BlockHeader memory header,
        bytes32 header_hash,
        bytes[] memory finalization_proof
    ) internal pure returns (bool) {
        uint64 total_voting_power;
        uint64 voted_voting_power;
        for (uint i = 0; i < header.validators.length; i++) {
            total_voting_power += header.voting_power[i];
        }
        uint k = 0;
        for (uint j = 0; j < finalization_proof.length; j++) {
            (bytes memory signer, bytes memory signature) = abi.decode(
                finalization_proof[j],
                (bytes, bytes)
            );
            (bytes32 r, bytes32 s, uint8 v) = split_signature(signature);
            if (pub_to_address(signer) == ecrecover(to_prefixed_hash(header_hash), v, r, s)) {
                voted_voting_power += header.voting_power[k];
            }
            k++;
        }

        require(
            voted_voting_power * 3 > total_voting_power * 2,
            "Verify::verify_finalization_proof: Not enough voting power"
        );
        return true;
    }

    function split_signature(
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

    /// @notice The ```parse_header``` function is used to decode bytes to BlockHeader struct
    /// @dev Since we can't directly decode dynamic array in solidity, we need to decode it manually (Tricky way)
    function parse_header(
        bytes memory header
    ) internal pure returns (BlockHeader memory block_header) {
        (
            bytes memory author,
            bytes[] memory prev_block_finalization_proof,
            bytes32 previous_hash,
            uint64 block_height,
            int64 timestamp,
            bytes32 commit_merkle_root,
            bytes32 repository_merkle_root,
            bytes[] memory validators,
            uint64[] memory voting_power,
            bytes32 version
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
                    bytes32
                )
            );

        block_header = BlockHeader({
            author: author,
            prev_block_finalization_proof: prev_block_finalization_proof,
            previous_hash: previous_hash,
            block_height: block_height,
            timestamp: timestamp,
            commit_merkle_root: commit_merkle_root,
            repository_merkle_root: repository_merkle_root,
            validators: validators,
            voting_power: voting_power,
            version: version
        });
    }

    function to_prefixed_hash(bytes32 hash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", hash));
    }

    function pub_to_address(bytes memory pk) internal pure returns (address) {
        bytes32 hash = keccak256(pk);

        return address(uint160(uint256(hash)));
    }
}
