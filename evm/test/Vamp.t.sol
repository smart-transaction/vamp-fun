// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/Test.sol";
import {Vamp} from "../src/Vamp.sol";

contract VampTest is Test {
    Vamp public vamp;

    // function setUp() public {
    //     vamp = new Vamp();
    //     vamp.setNumber(0);
    // }

    // function test_Increment() public {
    //     vamp.increment();
    //     assertEq(vamp.number(), 1);
    // }

    // function testFuzz_SetNumber(uint256 x) public {
    //     vamp.setNumber(x);
    //     assertEq(vamp.number(), x);
    // }
}
