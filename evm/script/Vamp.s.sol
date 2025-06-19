// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {Vamp} from "../src/Vamp.sol";

contract VampScript is Script {
    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address callbreakerAddress = vm.envAddress("CALL_BREAKER_ADDRESS");
        uint256 fee = vm.envUint("FEE");

        vm.startBroadcast(deployerPrivateKey);

        Vamp vamp = new Vamp(fee);
        console.log("Vamp deployed at:", address(vamp));

        // Set the call breaker address as Vamper
        bytes32 vamperRole = vamp.VAMPER();
        vamp.grantRole(vamperRole, callbreakerAddress);
        console.log("Call Breaker address set as Vamper:", callbreakerAddress);

        vm.stopBroadcast();
    }
}
