// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

contract Vamp is AccessControl {
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant VAMPER = keccak256("VAMPER");

    uint256 public fee;

    mapping(bytes32 => bool) public approvedRequests;

    /// @dev Thrown when a zero address is provided
    /// @dev Selector 0xd92e233d
    error ZeroAddress();

    /// @dev Error thrown when direct ETH transfer is attempted
    /// @dev Selector 0xb15db189
    error DirectETHTransferNotAllowed();
    
    /// @dev Error thrown when preApproval failed
    /// @dev Selector 0x5ddc0f29
    error VampRejected(bytes32 requestId);

    /// @dev Error thrown when account balance is zero
    /// @dev Selector 0x669567ea
    error ZeroBalance();

    /// @dev Error thrown when fee transfer failed
    /// @dev Selector 0xa8718bae
    error FeeTransferFailed(address receiver);

    event VampApproved(bytes32 indexed requestId, uint256 indexed feeAmount);
    event FeeSet(uint256 newFee);
    event VampFeeWithdrawn(address indexed receiver, uint256 indexed feeAmount);

    constructor(uint256 _fee) {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);

        fee = _fee;
    }
    
    function preApprove(bytes32 requestId) external payable onlyRole(VAMPER) returns (bool) {
        if (msg.value < fee) {
            revert VampRejected(requestId);
        } else {
                approvedRequests[requestId] = true;
                emit VampApproved(requestId, fee);
                return true;
        }
    }

    function withdrawVampFee(address receiver) external onlyRole(ADMIN_ROLE) {
        if (receiver == address(0)) {
            revert ZeroAddress();
        } 
        uint256 totalBalance = address(this).balance;
        if (totalBalance == 0){
            revert ZeroBalance();
        } else {
            (bool success, ) = receiver.call{value: totalBalance}("");
            if (!success) {
                revert FeeTransferFailed(receiver);
            }
            emit VampFeeWithdrawn(receiver, totalBalance);
        }
    }

    function setFee(uint256 newFee) external onlyRole(ADMIN_ROLE) {
        fee = newFee;
        emit FeeSet(newFee);
    }

    function getTotalFee() external view returns(uint256) {
        return address(this).balance;
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
