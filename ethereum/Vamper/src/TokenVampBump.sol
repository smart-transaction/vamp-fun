// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/// @dev ERC-1046 draft extension (not provided by OpenZeppelin)
interface IERC20MetadataURI {
    function tokenURI() external view returns (string memory);
}

/// @notice UUPS-upgradeable, reentrancy-protected intent emitter
contract TokenVampBump is ReentrancyGuard, Ownable
{
    // ---- custom errors ----
    error NotAToken();
    error ZeroToken();
    error BadFee();
    error FeeTransferFailed();
    error NotAnOwner();
    error ZeroOwner();

    uint256 public feeWei;

    // Monotonic per-contract nonce used to make intentId deterministic & unique
    uint64 public nonce;

    event VampTokenIntent(
        uint64 chainId,
        uint64 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );

    /// @custom:oz-upgrades-unsafe-allow constructor
    /// @param _feeWei Required ETH fee in wei
    constructor(address _owner, uint256 _feeWei) Ownable(_owner) {
        feeWei = _feeWei;
        nonce = 0;
    }

    function isContract(address addr) internal view returns (bool) {
        return addr.code.length > 0;
    }

    function setFee(uint256 _feeWei) external onlyOwner {
        feeWei = _feeWei;
    }

    /// @notice Called by frontend. Charges ETH fee and emits an intent event with token metadata.
    /// @param token ERC-20 token address
    function vampToken(address token) external payable nonReentrant {
        if (token == address(0)) revert ZeroToken();
        if (!isContract(token)) revert NotAToken();
        if (msg.value != feeWei) revert BadFee();

        address caller = msg.sender;

        uint64 currentNonce = nonce;

        // Deterministic globally-unique intentId
        bytes32 intentId = keccak256(
            abi.encodePacked(block.chainid, address(this), caller, currentNonce)
        );

        unchecked {
            nonce = currentNonce + 1;
        }

        // Forward ETH fee to owner if fee is set
        if (feeWei > 0) {
            address owner_ = owner();
            (bool ok, ) = payable(owner_).call{value: msg.value}("");
            if (!ok) revert FeeTransferFailed();
        }

        // ERC-20 metadata (may revert for non-conforming tokens)
        string memory tName;
        try IERC20Metadata(token).name() returns (string memory name) {
            tName = name;
        } catch {
            tName = "";
        }
        string memory tSymbol;
        try IERC20Metadata(token).symbol() returns (string memory symbol) {
            tSymbol = symbol;
        } catch {
            tSymbol = "";
        }

        // ERC-1046 tokenURI (best-effort)
        string memory tUri;
        try IERC20MetadataURI(token).tokenURI() returns (string memory uri) {
            tUri = uri;
        } catch {
            tUri = "";
        }

        emit VampTokenIntent(
            uint64(block.chainid),
            uint64(block.number),
            intentId,
            caller,
            token,
            tName,
            tSymbol,
            tUri
        );
    }
}
