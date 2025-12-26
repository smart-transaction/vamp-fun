// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {VampTokenEmitterUpgradeable} from "../src/VampTokenEmitterUpgradeable.sol";

contract CounterScript is Script {
    VampTokenEmitterUpgradeable public vamper;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        vamper = new VampTokenEmitterUpgradeable();

        vm.stopBroadcast();
    }
}
