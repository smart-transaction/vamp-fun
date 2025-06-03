// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract Vamp is Ownable {
    uint256 public fee;
    address public treasury;
    IERC20 public feeToken;

    error ZeroAddress();
    error InsufficientBalance();

    event TreasurySet(address indexed newTreasury);
    event VampInitiated(address indexed vamper, address indexed vampToken);
    event FeeSet(uint256 newFee);
    event FeeTokenSet(address feeToken);

    constructor(
        address _treasury,
        uint256 _fee,
        address _feeToken
    ) Ownable(msg.sender) {
        if (_treasury == address(0) || _feeToken == address(0))
            revert ZeroAddress();
        treasury = _treasury;
        fee = _fee;
        feeToken = IERC20(_feeToken);
    }

    function setTreasury(address newTreasury) external onlyOwner {
        if (newTreasury == address(0)) revert ZeroAddress();
        treasury = newTreasury;
        emit TreasurySet(newTreasury);
    }

    function setFee(uint256 newFee) external onlyOwner {
        fee = newFee;
        emit FeeSet(newFee);
    }

    function initiateVamp(
        address vamper,
        address vampToken
    ) external onlyOwner {
        if (vamper == address(0) || vampToken == address(0))
            revert ZeroAddress();
        bool success = feeToken.transferFrom(vamper, treasury, fee);
        if (!success) revert InsufficientBalance();
        emit VampInitiated(vamper, vampToken);
    }

    function setFeeToken(address _feeToken) external onlyOwner {
        if (_feeToken == address(0)) revert ZeroAddress();
        feeToken = IERC20(_feeToken);
        emit FeeTokenSet(_feeToken);
    }
}
