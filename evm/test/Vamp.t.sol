// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {Vamp} from "../src/Vamp.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockToken is ERC20 {
    constructor() ERC20("Mock Token", "MTK") {
        _mint(msg.sender, 1000000 * 10 ** 18);
    }
}

contract VampTest is Test {
    Vamp public vamp;
    MockToken public feeToken;
    address public treasury;
    uint256 public fee = 100 * 10 ** 18; // 100 tokens

    function setUp() public {
        treasury = makeAddr("treasury");
        feeToken = new MockToken();
        vamp = new Vamp(treasury, fee, address(feeToken));
    }

    function test_Constructor() public view {
        assertEq(vamp.treasury(), treasury);
        assertEq(vamp.fee(), fee);
        assertEq(address(vamp.feeToken()), address(feeToken));
    }

    function test_RevertWhen_TreasuryZeroAddress() public {
        treasury = address(0);
        feeToken = new MockToken();

        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp = new Vamp(treasury, fee, address(feeToken));
    }

    function test_RevertWhen_FeeTokenZeroAddress() public {
        treasury = makeAddr("treasury");

        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp = new Vamp(treasury, fee, address(0));
    }

    function test_SetTreasury() public {
        address newTreasury = makeAddr("newTreasury");
        vamp.setTreasury(newTreasury);
        assertEq(vamp.treasury(), newTreasury);
    }

    function test_Event_WhenSettingTreasury() public {
        address newTreasury = makeAddr("newTreasury");
        vm.expectEmit(true, false, false, true);
        emit Vamp.TreasurySet(newTreasury);
        vamp.setTreasury(newTreasury);
        assertEq(vamp.treasury(), newTreasury);
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

    function test_SetFeeToken() public {
        address newFeeToken = makeAddr("feeToken");
        vamp.setFeeToken(newFeeToken);
        assertEq(vamp.getFeeToken(), newFeeToken);
    }

    function test_Event_WhenSettingFeeToken() public {
        address newFeeToken = makeAddr("feeToken");
        vm.expectEmit(true, false, false, true);
        emit Vamp.FeeTokenSet(newFeeToken);
        vamp.setFeeToken(newFeeToken);
        assertEq(vamp.getFeeToken(), newFeeToken);
    }

    function test_InitiateVamp() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        // Fund vamper with fee tokens
        feeToken.transfer(vamper, fee);

        // Approve fee tokens
        vm.prank(vamper);
        feeToken.approve(address(vamp), fee);

        // Initiate vamp
        vamp.initiateVamp(vamper, vampToken);

        // Check treasury received fee
        assertEq(feeToken.balanceOf(treasury), fee);
    }

    function test_InitiateVamp_WithNativeToken() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        // Initiate vamp

        vm.expectEmit(true, true, false, true);
        emit Vamp.VampInitiated(vamper, vampToken);
        vamp.initiateVamp{value: fee}(vamper, vampToken);

        // Check treasury received fee
        assertEq(treasury.balance, fee);
    }

    function test_RevertWhen_InitiateVamp_WithInsufficientNativeToken() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        // Initiate vamp

        vm.expectRevert(Vamp.InsufficientFee.selector);
        vamp.initiateVamp{value: fee - 1}(vamper, vampToken);

        // Check treasury received fee
        assertEq(treasury.balance, 0);
    }

    function test_RevertWhen_InitiateVampWithoutApproval() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        vm.expectRevert(
            abi.encodeWithSignature(
                "ERC20InsufficientAllowance(address,uint256,uint256)",
                address(vamp),
                0,
                fee
            )
        );
        vamp.initiateVamp(vamper, vampToken);
    }

    function test_RevertWhen_InitiateVampZeroAddress() public {
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.initiateVamp(address(0), makeAddr("vampToken"));
    }

    function test_RevertWhen_InitiateVampByNotAdmin() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        // Fund vamper with fee tokens
        feeToken.transfer(vamper, fee);

        // Approve fee tokens
        vm.prank(vamper);
        feeToken.approve(address(vamp), fee);

        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(Vamp.NotFeeCollector.selector);
        vamp.initiateVamp(vamper, vampToken);
    }

    function test_RevertWhen_SetTreasuryZeroAddress() public {
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.setTreasury(address(0));
    }

    function test_RevertWhen_SetTreasuryByNotAdmin() public {
        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(Vamp.NotAdminRole.selector);
        vamp.setTreasury(makeAddr("newTreasury"));
    }

    function test_RevertWhen_SetFeeTokenZeroAddress() public {
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.setFeeToken(address(0));
    }

    function test_RevertWhen_SetFeeTokenByNotAdmin() public {
        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(Vamp.NotAdminRole.selector);
        vamp.setFeeToken(makeAddr("feeToken"));
    }

    function test_RevertWhen_SetFeeByNotAdmin() public {
        uint256 newFee = 200 * 10 ** 18;

        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(Vamp.NotAdminRole.selector);
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
        (bool success, ) = address(vamp).call{value: 1 ether}(
            abi.encodeWithSignature("nonExistentFunction()")
        );
        // after balance
        assertEq(address(vamp).balance, 0);
    }

    function test_RevokeFeeCollectorRole() public {
        address newFeeCollector = makeAddr("newFeeCollector");
        vamp.grantFeeCollectorRole(newFeeCollector);
    }

    function test_RevertWhen_RevokeFeeCollectorRole_ByNotAdmin() public {
        address newOwner = makeAddr("newOwner");
        vm.prank(newOwner);
        vm.expectRevert(Vamp.NotAdminRole.selector);
        vamp.revokeFeeCollectorRole(makeAddr("temporary"));
    }

    function test_RevertWhen_GrantFeeCollectorRole_ToZeroAddress() public {
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.grantFeeCollectorRole(address(0));
    }

    function test_RevertWhen_GrantFeeCollectorRole_NotByAdmin() public {
        address notOwner = makeAddr("notOwner");
        vm.prank(notOwner);
        vm.expectRevert(Vamp.NotAdminRole.selector);

        vamp.grantFeeCollectorRole(makeAddr("newUser"));
    }
}
