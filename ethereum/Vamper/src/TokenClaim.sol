// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/// @notice UUPS-upgradeable, reentrancy-protected intent emitter
contract TokenClaim is ReentrancyGuard, Ownable
{
    // ---- custom errors ----
    error BadFee();
    error FeeTransferFailed();
    error NotAnOwner();
    error ZeroOwner();

    uint256 public feeWei;

    event ClaimToken(
        bytes32 intentId,
        address claimer,
        uint256 amount,
        bytes signature,
        bytes20 claimerSolana
    );

    /// @custom:oz-upgrades-unsafe-allow constructor
    /// @param _feeWei Required ETH fee in wei
    constructor(address _owner, uint256 _feeWei) Ownable(_owner) {
        feeWei = _feeWei;
    }

    function setFee(uint256 _feeWei) external onlyOwner {
        feeWei = _feeWei;
    }

    /// @notice Called by frontend. Charges ETH fee and emits an event with claim data.
    /// @param _intentId The intent ID for given claiming
    /// @param _tokenAmount The amount of tokens to be claimed
    /// @param _signature The owner's account and amount signature
    function claimToken(bytes32 _intentId, uint256 _tokenAmount, bytes memory _signature, bytes20 _claimerSolana) external payable nonReentrant {
        if (msg.value != feeWei) revert BadFee();

        address claimer = msg.sender;

        // Forward ETH fee to owner if fee is set
        if (feeWei > 0) {
            address owner_ = owner();
            (bool ok, ) = payable(owner_).call{value: msg.value}("");
            if (!ok) revert FeeTransferFailed();
        }

        emit ClaimToken(
            _intentId,
            claimer,
            _tokenAmount,
            _signature,
            _claimerSolana
        );
    }
}
