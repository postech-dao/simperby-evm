// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC721/IERC721.sol";
// import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "../Library/Verify.sol";
import "./interfaces/IEVMTreasury.sol";

contract EVMTreasury is Pausable, ReentrancyGuard, IEVMTreasury {
    /// @notice The name of this contract
    string public constant name = "EVM SETTLEMENT CHAIN TREASURY V1";

    Client public client;

    mapping(uint256 => Client) public clients;
    // TODO: add/delete validator set
    mapping(bytes => uint64) public validatorSet;
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

    event UpdateLightClient(uint256 indexed height, bytes lastHeader, string chainName);

    /* ========== CONSTRUCTOR ========== */

    constructor(bytes memory initialHeader, string memory chainName) {
        // Genesis block
        client = Client(0, initialHeader, chainName);
        clients[0] = client;
    }

    /* ========== VIEWS ========== */

    /// TODO: add view functions if needed

    /* ========== TREASURY FUNCTIONS ========== */

    /// @notice The ```transferToken``` function is used to transfer tokens from the treasury to the receiver
    /// @dev Since we can't have struct in enum in solidity, need to seperate message type and data
    /// @param _message The type of the message
    /// @param _data The data of the message
    /// @param height The height of the consensus block
    /// @param merkleProof The merkle proof of the message
    function transferToken(
        DeliverableMessage _message,
        bytes memory _data,
        uint256 height,
        string memory merkleProof
    ) external whenNotPaused nonReentrant {
        require(
            verifyTransactionCommitment(_message, height, merkleProof),
            "EVMTreasury::transferToken: Invalid proof"
        );

        if (_message == DeliverableMessage.FungibleTokenTransfer) {
            FungibleTokenTransfer memory fungibleTokenTransfer = abi.decode(
                _data,
                (FungibleTokenTransfer)
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
        } else if (_message == DeliverableMessage.NonFungibleTokenTransfer) {
            NonFungibleTokenTransfer memory nonFungibleTokenTransfer = abi.decode(
                _data,
                (NonFungibleTokenTransfer)
            );
            withdrawERC721(
                nonFungibleTokenTransfer.collectionAddress,
                nonFungibleTokenTransfer.receiverAddress,
                nonFungibleTokenTransfer.tokenIndex
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
    function updateLightClient(bytes calldata header, bytes[] calldata proof) public whenNotPaused {
        Verify.BlockHeader memory _blockHeader = Verify.parseHeader(header);

        Verify.verifyHeaderToHeader(client.lastHeader, header);
        Verify.verifyFinalizationProof(_blockHeader, keccak256(header), proof);

        clients[_blockHeader.blockHeight] = Client(
            _blockHeader.blockHeight,
            header,
            client.chainName
        );
        client.height = _blockHeader.blockHeight;
        client.lastHeader = header;

        emit UpdateLightClient(client.height, header, client.chainName);
    }

    /// @notice The argument types and logic need to be replaced with the proper types
    function verifyTransactionCommitment(
        DeliverableMessage message,
        uint256 height,
        string memory merkleProof
    ) public view returns (bool isValid) {
        isValid =
            client.height == height &&
            keccak256(abi.encodePacked(merkleProof)) == keccak256(abi.encodePacked("valid"));
    }
}
