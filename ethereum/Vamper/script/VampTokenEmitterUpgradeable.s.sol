// SPDX-License-Identifier: MIT
pragma solidity 0.8.33;

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
