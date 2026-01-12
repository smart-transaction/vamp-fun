// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import "forge-std/Test.sol";

// OZ proxy
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import "../src/TokenClaim.sol";

contract ClaimTokenTest is Test {
    TokenClaim internal claim;
    TokenClaim internal zeroClaim;

    bytes32 internal intentId = bytes32(uint256(123438574032938745049687));
    address internal owner  = address(0xABCD);
    address internal user  = address(0xBEEF);
    uint256 internal amount = 10_000_000_000;
    bytes internal signature = initSignature();
    bytes20 claimerSolana = bytes20(address(0x12345678));

    uint256 internal feeWei = 1 gwei;

    // Re-declare the event so we can use vm.expectEmit with it
    event ClaimToken(
        bytes32 intentId,
        address claimer,
        uint256 amount,
        bytes signature,
        bytes20 claimerColana
    );

    function initSignature() pure internal returns (bytes memory sig) {
        uint8[16] memory sigArr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        sig = new bytes(sigArr.length);
        for (uint256 i = 0; i < sigArr.length; ++i) {
            sig[i] = bytes1(sigArr[i]);
        }
    }

    function setUp() public {
        // Fund accounts
        vm.deal(owner, 10 ether);
        vm.deal(user, 10 ether);

        // Deploy implementation
        claim = new TokenClaim(owner, feeWei);
        zeroClaim = new TokenClaim(owner, 0);
    }

    function test_initialize_setsOwnerFeeNonce() public view {
        assertEq(claim.feeWei(), feeWei);
    }

    function test_setFee_owner_success() public {
        vm.prank(claim.owner());
        claim.setFee(2 gwei);
        assertEq(claim.feeWei(), 2 gwei);
    }

    function test_setFee_non_owner_fail() public {
        vm.prank(user);
        vm.expectRevert();
        claim.setFee(2 gwei);
    }

    function test_claimToken_revertsOnBadFee() public {
        vm.prank(user);
        vm.expectRevert();
        claim.claimToken{value: feeWei - 1}(intentId, amount, signature, claimerSolana);
    }

    function test_claimToken_transfersFee_andEmitsEvent() public {
        uint256 ownerBalBefore = claim.owner().balance;

        // Expect emit with exact values
        
        vm.expectEmit(true, true, true, true, address(claim));
        emit ClaimToken(
            intentId,
            user,
            amount,
            signature,
            claimerSolana
        );

        vm.prank(user);
        claim.claimToken{value: feeWei}(intentId, amount, signature, claimerSolana);

        // Owner received the fee
        assertEq(claim.owner().balance - ownerBalBefore, feeWei);
    }

    function test_claimToken_transfersZeroFee_emitsEventOnly() public {
        uint256 ownerBalBefore = zeroClaim.owner().balance;

        // Expect emit with exact values
        vm.expectEmit(true, true, true, true, address(zeroClaim));
        emit ClaimToken(
            intentId,
            user,
            amount,
            signature,
            claimerSolana
        );

        vm.prank(user);
        zeroClaim.claimToken{value: 0}(intentId, amount, signature, claimerSolana);

        // Owner received the fee
        assertEq(zeroClaim.owner().balance - ownerBalBefore, 0);
    }
}
