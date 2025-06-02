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

    function test_Constructor() public {
        assertEq(vamp.treasury(), treasury);
        assertEq(vamp.fee(), fee);
        assertEq(address(vamp.feeToken()), address(feeToken));
    }

    function test_SetTreasury() public {
        address newTreasury = makeAddr("newTreasury");
        vm.prank(vamp.owner());
        vamp.setTreasury(newTreasury);
        assertEq(vamp.treasury(), newTreasury);
    }

    function test_SetFee() public {
        uint256 newFee = 200 * 10 ** 18;
        vm.prank(vamp.owner());
        vamp.setFee(newFee);
        assertEq(vamp.fee(), newFee);
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
        vm.prank(vamp.owner());
        vamp.initiateVamp(vamper, vampToken);

        // Check treasury received fee
        assertEq(feeToken.balanceOf(treasury), fee);
    }

    function test_RevertWhen_InitiateVampWithoutApproval() public {
        address vamper = makeAddr("vamper");
        address vampToken = makeAddr("vampToken");

        vm.prank(vamp.owner());
        vm.expectRevert(
            abi.encodeWithSignature("ERC20InsufficientAllowance(address,uint256,uint256)", address(vamp), 0, fee)
        );
        vamp.initiateVamp(vamper, vampToken);
    }

    function test_RevertWhen_SetTreasuryZeroAddress() public {
        vm.prank(vamp.owner());
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.setTreasury(address(0));
    }

    function test_RevertWhen_InitiateVampZeroAddress() public {
        vm.prank(vamp.owner());
        vm.expectRevert(Vamp.ZeroAddress.selector);
        vamp.initiateVamp(address(0), makeAddr("vampToken"));
    }
}
