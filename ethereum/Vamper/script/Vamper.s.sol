// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Vamper} from "../src/Vamper.sol";

contract CounterScript is Script {
    Vamper public vamper;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        vamper = new Vamper(1000000000);  // 1 GWei

        vm.stopBroadcast();
    }
}
