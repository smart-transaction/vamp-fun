// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

contract Vamp is AccessControl {
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant VAMPER = keccak256("VAMPER");

    uint256 public fee;
    address public treasury;
    address public feeToken;

    /// @dev Thrown when a zero address is provided
    /// @dev Selector 0x8b6f91a3
    error ZeroAddress();

    /// @dev Thrown when fee transfer fails
    /// @dev Selector 0x7c8c2c0b
    error FeeTransferFailed();

    /// @dev Thrown when insufficient fee is provided
    /// @dev Selector 0x8f9f5e44
    error InsufficientFee();

    /// @dev Error thrown when direct ETH transfer is attempted
    /// @dev Selector 0x157bd4c3
    error DirectETHTransferNotAllowed();

    event TreasurySet(address indexed newTreasury);
    event VampInitiated(address indexed vamper, address indexed vampToken);
    event FeeSet(uint256 newFee);
    event FeeTokenSet(address indexed feeToken);

    constructor(address _treasury, uint256 _fee, address _feeToken) {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);

        if (_treasury == address(0) || _feeToken == address(0))
            revert ZeroAddress();
        treasury = _treasury;
        fee = _fee;
        feeToken = _feeToken;
    }

    function initiateVamp(
        address vamper,
        address vampToken
    ) external payable onlyRole(VAMPER) returns (bool success) {
        if (vamper == address(0) || vampToken == address(0))
            revert ZeroAddress();

        if (msg.value == 0) {
            success = IERC20(feeToken).transferFrom(vamper, treasury, fee);
            if (!success) revert FeeTransferFailed();
        } else {
            if (msg.value < fee) {
                revert InsufficientFee();
            }
            (success, ) = payable(treasury).call{value: msg.value}("");
            if (!success) revert FeeTransferFailed();
        }
        emit VampInitiated(vamper, vampToken);
    }

    function setTreasury(address newTreasury) external onlyRole(ADMIN_ROLE) {
        if (newTreasury == address(0)) revert ZeroAddress();
        treasury = newTreasury;
        emit TreasurySet(newTreasury);
    }

    function setFee(uint256 newFee) external onlyRole(ADMIN_ROLE) {
        fee = newFee;
        emit FeeSet(newFee);
    }

    function setFeeToken(address _feeToken) external onlyRole(ADMIN_ROLE) {
        if (_feeToken == address(0)) revert ZeroAddress();
        feeToken = _feeToken;
        emit FeeTokenSet(_feeToken);
    }

    // TODO: Claim extra ETH transferred by user

    /// @notice Prevents direct native currency transfers to the contract
    receive() external payable {
        revert DirectETHTransferNotAllowed();
    }

    /// @notice Prevents native currency transfers via fallback
    fallback() external payable {
        revert DirectETHTransferNotAllowed();
    }
}
