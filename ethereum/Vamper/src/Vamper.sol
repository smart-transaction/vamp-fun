// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/// @dev ERC-1046 draft extension (not provided by OpenZeppelin)
interface IERC20MetadataURI {
    function tokenURI() external view returns (string memory);
}

/// @notice Gas-optimized intent emitter
contract VampTokenEmitter is ReentrancyGuard {
    // ---- custom errors (cheaper than revert strings) ----
    error NotOwner();
    error ZeroOwner();
    error ZeroToken();
    error BadFee();
    error FeeTransferFailed();

    address payable public owner;
    uint256 public feeWei;

    // Monotonic per-contract nonce used to make intentId deterministic & unique
    uint64 public nonce;

    event VampTokenIntent(
        uint256 chainId,
        uint256 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(uint256 _feeWei) {
        owner = payable(msg.sender);
        feeWei = _feeWei;
        // nonce starts at 0
    }

    function setFee(uint256 _feeWei) external onlyOwner {
        feeWei = _feeWei;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        if (newOwner == address(0)) revert ZeroOwner();
        owner = payable(newOwner);
    }

    /// @notice Called by frontend. Charges ETH fee and emits an intent event with token metadata.
    /// @param token ERC-20 token address
    function vampToken(address token) external payable nonReentrant {
        if (token == address(0)) revert ZeroToken();
        if (msg.value != feeWei) revert BadFee();

        address caller = msg.sender;
        uint64 currentNonce = nonce;

        // Deterministic globally-unique intentId
        bytes32 intentId = keccak256(
            abi.encodePacked(
                block.chainid,
                address(this),
                caller,
                currentNonce
            )
        );

        unchecked {
            nonce = currentNonce + 1;
        }

        // Forward ETH fee to owner
        (bool ok, ) = owner.call{value: msg.value}("");
        if (!ok) revert FeeTransferFailed();

        // ERC-20 metadata
        string memory tName = IERC20Metadata(token).name();
        string memory tSymbol = IERC20Metadata(token).symbol();

        // ERC-1046 tokenURI (best-effort)
        string memory tUri;
        try IERC20MetadataURI(token).tokenURI() returns (string memory uri) {
            tUri = uri;
        } catch {
            tUri = "";
        }

        emit VampTokenIntent(
            block.chainid,
            block.number,
            intentId,
            caller,
            token,
            tName,
            tSymbol,
            tUri
        );
    }
}
