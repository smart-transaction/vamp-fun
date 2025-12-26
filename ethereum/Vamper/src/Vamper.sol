// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20MetadataURI.sol";

contract VampTokenEmitter {
    address public owner;
    uint256 public feeWei;

    // Monotonic per-contract nonce used to make intentId deterministic & unique
    uint256 public nonce;

    event VampTokenIntent(
        uint256 chainId,
        uint256 blockNumber,
        bytes32 intentId,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );

    modifier onlyOwner() {
        require(msg.sender == owner, "NOT_OWNER");
        _;
    }

    constructor(uint256 _feeWei) {
        owner = msg.sender;
        feeWei = _feeWei;
        nonce = 0;
    }

    function setFee(uint256 _feeWei) external onlyOwner {
        feeWei = _feeWei;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "ZERO_OWNER");
        owner = newOwner;
    }

    /// @notice Called by frontend. Charges ETH fee and emits an intent event with token metadata.
    /// @param token ERC-20 token address
    function vampToken(address token) external payable {
        require(token != address(0), "ZERO_TOKEN");
        require(msg.value == feeWei, "BAD_FEE");

        // Forward ETH fee to owner
        (bool ok, ) = owner.call{value: msg.value}("");
        require(ok, "FEE_TRANSFER_FAILED");

        // Compute deterministic globally-unique intentId
        uint256 currentNonce = nonce;
        bytes32 intentId = keccak256(
            abi.encodePacked(
                block.chainid,
                address(this),
                msg.sender,
                currentNonce
            )
        );

        unchecked {
            nonce = currentNonce + 1;
        }

        // ERC-20 metadata (name + symbol)
        string memory tName = IERC20Metadata(token).name();
        string memory tSymbol = IERC20Metadata(token).symbol();

        // ERC-1046 tokenURI (best-effort)
        string memory tUri = "";
        try IERC20MetadataURI(token).tokenURI() returns (string memory uri) {
            tUri = uri;
        } catch {
            // token does not support ERC-1046
        }

        emit VampTokenIntent(
            block.chainid,
            block.number,
            intentId,
            token,
            tName,
            tSymbol,
            tUri
        );
    }
}
