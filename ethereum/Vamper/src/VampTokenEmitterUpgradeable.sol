// SPDX-License-Identifier: MIT
pragma solidity 0.8.33;

import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

/// @dev ERC-1046 draft extension (not provided by OpenZeppelin)
interface IERC20MetadataURI {
    function tokenURI() external view returns (string memory);
}

/// @notice UUPS-upgradeable, reentrancy-protected intent emitter
contract VampTokenEmitterUpgradeable is
    Initializable,
    UUPSUpgradeable,
    OwnableUpgradeable,
    ReentrancyGuard
{
    // ---- custom errors (cheaper than revert strings) ----
    error ZeroToken();
    error BadFee();
    error FeeTransferFailed();

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

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        // Prevent the implementation contract from being initialized.
        _disableInitializers();
    }

    /// @notice Initializer (replaces constructor for upgradeable contracts)
    /// @param initialOwner Owner address for OwnableUpgradeable
    /// @param _feeWei Required ETH fee in wei
    function initialize(address initialOwner, uint256 _feeWei) external initializer {
        __Ownable_init(initialOwner);

        feeWei = _feeWei;
        nonce = 0;
    }

    function setFee(uint256 _feeWei) external onlyOwner {
        feeWei = _feeWei;
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
            abi.encodePacked(block.chainid, address(this), caller, currentNonce)
        );

        unchecked {
            nonce = currentNonce + 1;
        }

        // Forward ETH fee to owner
        address owner_ = owner();
        (bool ok, ) = payable(owner_).call{value: msg.value}("");
        if (!ok) revert FeeTransferFailed();

        // ERC-20 metadata (may revert for non-conforming tokens)
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

    /// @dev UUPS authorization hook
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}
}
