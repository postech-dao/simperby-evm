// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "./interfaces/IEVMTreasury.sol";

// PDAO: https://github.com/postech-dao

contract EVMTreasury is Ownable, Pausable, ReentrancyGuard, IEVMTreasury {
    /// @notice The name of this contract
    string public constant name = "PDAO EVM COLONY CHAIN TREASURY V1";

    Client public client;

    mapping(uint256 => Client) public clients;

    /* ========== EVENTS ========== */

    event TransferFungibleToken(
        address indexed token_address,
        uint256 amount,
        address indexed receiver_address,
        uint256 contract_sequence
    );

    event TransferNonFungibleToken(
        address indexed collection_address,
        uint256 token_index,
        address indexed receiver_address,
        uint256 contract_sequence
    );

    event UpdateLightclient(uint256 indexed height, string last_header, string chain_name);

    /* ========== CONSTRUCTOR ========== */

    constructor(string memory initial_header, string memory chain_name) {
        // Genesis block
        client = Client(0, initial_header, chain_name);
        clients[0] = client;
    }

    /* ========== VIEWS ========== */

    /// TODO: add view functions if needed

    /* ========== TREASURY FUNCTIONS ========== */

    /// @notice The ```transfer_token``` function is used to transfer tokens from the treasury to the receiver
    /// @dev Since we can't have struct in enum in solidity, need to seperate message type and data
    /// @param _message The type of the message
    /// @param _data The data of the message
    /// @param height The height of the consensus block
    /// @param merkleProof The merkle proof of the message
    function transfer_token(
        DeliverableMessage _message,
        bytes memory _data,
        uint256 height,
        string memory merkleProof
    ) external onlyOwner whenNotPaused nonReentrant {
        require(
            verify_commitment(_message, height, merkleProof),
            "EVMTreasury::transfer_token: Invalid proof"
        );

        if (_message == DeliverableMessage.FungibleTokenTransfer) {
            FungibleTokenTransfer memory fungibleTokenTransfer = abi.decode(
                _data,
                (FungibleTokenTransfer)
            );
            if (fungibleTokenTransfer.token_address == address(0)) {
                withdrawETH(fungibleTokenTransfer.receiver_address, fungibleTokenTransfer.amount);
            } else {
                withdrawERC20(
                    fungibleTokenTransfer.token_address,
                    fungibleTokenTransfer.receiver_address,
                    fungibleTokenTransfer.amount
                );
            }
        } else if (_message == DeliverableMessage.NonFungibleTokenTransfer) {
            NonFungibleTokenTransfer memory nonFungibleTokenTransfer = abi.decode(
                _data,
                (NonFungibleTokenTransfer)
            );
            withdrawERC721(
                nonFungibleTokenTransfer.collection_address,
                nonFungibleTokenTransfer.receiver_address,
                nonFungibleTokenTransfer.token_index
            );
        } else {
            Custom memory custom = abi.decode(_data, (Custom));
            /// TODO: add custom message
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

    /// @notice The argument types and logic need to be replaced with the proper types
    function update_light_client(
        string memory header,
        string memory proof
    ) public onlyOwner whenNotPaused {
        require(
            keccak256(abi.encodePacked(proof)) == keccak256(abi.encodePacked("valid")),
            "EVMTreasury::update_light_client: Invalid block finalization proof"
        );
        client.height += 1;
        client.last_header = header;
        clients[client.height] = client;

        emit UpdateLightclient(client.height, header, client.chain_name);
    }

    /// @notice The argument types and logic need to be replaced with the proper types
    function verify_commitment(
        DeliverableMessage message,
        uint256 height,
        string memory merkleProof
    ) public view returns (bool is_valid) {
        is_valid =
            client.height == height &&
            keccak256(abi.encodePacked(merkleProof)) == keccak256(abi.encodePacked("valid"));
    }
}
