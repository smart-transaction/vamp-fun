// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {Vamp} from "../src/Vamp.sol";

contract VampTest is Test {
    Vamp public vamp;
    uint256 public fee = 100 * 10 ** 18; // 100 tokens
    address public CALLBREAKER = makeAddr("CALLBREAKER");

    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant VAMPER = keccak256("VAMPER");
    bytes32 public constant REQUEST_ID = keccak256("REQUEST_ID");

    function setUp() public {
        vamp = new Vamp(fee);
    }

    function test_Constructor() public view {
        assertEq(vamp.fee(), fee);
    }

    function test_SetFee() public {
        uint256 newFee = 200 * 10 ** 18;
        vamp.setFee(newFee);
        assertEq(vamp.fee(), newFee);
    }

    function test_Event_WhenSettingFee() public {
        uint256 newFee = 200 * 10 ** 18;
        vm.expectEmit(true, false, false, true);
        emit Vamp.FeeSet(newFee);
        vamp.setFee(newFee);
        assertEq(vamp.fee(), newFee);
    }

    function test_GrantRoleRevert() public {
        address vamper = makeAddr("vamper");

        // Grant role
        vm.prank(vamper);
        vm.expectRevert(
            abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", vamper, DEFAULT_ADMIN_ROLE)
        );
        vamp.grantRole(VAMPER, address(this));
    }

    function test_preApprove() public {
        // Grant role
        vamp.grantRole(VAMPER, address(this));

        // Initiate vamp
        vm.expectEmit(true, true, false, true);
        emit Vamp.VampApproved(REQUEST_ID, fee);
        vamp.preApprove{value: fee}(REQUEST_ID);

        // Check if request is approved
        assertTrue(vamp.approvedRequests(REQUEST_ID));

        // Check contract received fee
        assertEq(address(vamp).balance, fee);
    }

    function test_RevertWhen_preApprove_WithInsufficientNativeToken() public {
        // Grant role
        vamp.grantRole(VAMPER, address(this));

        // Initiate vamp
        vm.expectRevert();
        vamp.preApprove{value: fee - 1}(REQUEST_ID);

        // Check contract received fee
        assertEq(address(vamp).balance, 0);
    }

    function test_RevertWhen_preApproveByNotVamper() public {
        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", notOwner, VAMPER));
        vamp.preApprove(REQUEST_ID);
    }

    function test_RevertWhen_SetFeeByNotAdmin() public {
        uint256 newFee = 200 * 10 ** 18;

        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(
            abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", notOwner, ADMIN_ROLE)
        );
        vamp.setFee(newFee);
    }

    function test_RevertWhen_DirectEthTransferToContract() public {
        vm.expectRevert(Vamp.DirectETHTransferNotAllowed.selector);
        // before balance
        assertEq(address(vamp).balance, 0);
        payable(address(vamp)).transfer(1 ether);
        // after balance
        assertEq(address(vamp).balance, 0);
    }

    function test_RevertWhen_Fallback() public {
        vm.expectRevert(Vamp.DirectETHTransferNotAllowed.selector);
        // before balance
        assertEq(address(vamp).balance, 0);
        address(vamp).call{value: 1 ether}(abi.encodeWithSignature("nonExistentFunction()"));
        // after balance
        assertEq(address(vamp).balance, 0);
    }

    function test_RevokeFeeCollectorRole() public {
        address newFeeCollector = makeAddr("newFeeCollector");
        vamp.grantRole(VAMPER, newFeeCollector);
    }

    function test_RevertWhen_RevokeFeeCollectorRole_ByNotAdmin() public {
        address newOwner = makeAddr("newOwner");
        vm.prank(newOwner);

        vm.expectRevert(
            abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", newOwner, DEFAULT_ADMIN_ROLE)
        );
        vamp.revokeRole(VAMPER, makeAddr("temporary"));
    }

    function test_RevertWhen_GrantFeeCollectorRole_NotByAdmin() public {
        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);

        vm.expectRevert(
            abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", notOwner, DEFAULT_ADMIN_ROLE)
        );

        vamp.grantRole(VAMPER, makeAddr("newUser"));
    }

    function test_withdrawVampFee() public {
        vm.deal(address(vamp), 10);
        address newFeeCollector = makeAddr("newfeeCollector");
        vamp.withdrawVampFee(newFeeCollector);
        //check if fee is withdrawn
        assertEq(address(vamp).balance, 0);
    }

    function test_RevertWhen_withdrawVampFee_AccountAddress_Zero() public {
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.withdrawVampFee(address(0));
    }

    function test_RevertWhen_withdrawVampFee_ContractBalance_Zero() public {
        address newFeeCollector = makeAddr("newfeeCollector");
        vm.expectRevert(Vamp.ZeroBalance.selector);
        vamp.withdrawVampFee(newFeeCollector);
    }

    function test_RevertWhen_withdrawVampFee_NotByAdmin() public {
        address newFeeCollector = makeAddr("newfeeCollector");
        address notAdmin = makeAddr("notAdmin");
        vm.prank(notAdmin);
        vm.deal(address(vamp), 10);
        vm.expectRevert(
            abi.encodeWithSignature("AccessControlUnauthorizedAccount(address,bytes32)", notAdmin, ADMIN_ROLE)
        );
        vamp.withdrawVampFee(newFeeCollector);
    }

    function test_RevertWhen_withdrawVampFee_CallFails() public {
        address newFeeCollector = address(vamp);
        vm.deal(address(vamp), 10);
        vm.expectRevert(abi.encodeWithSelector(Vamp.FeeTransferFailed.selector, newFeeCollector));
        vamp.withdrawVampFee(newFeeCollector);
    }
}