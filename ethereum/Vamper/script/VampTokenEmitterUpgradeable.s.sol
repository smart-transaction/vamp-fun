// SPDX-License-Identifier: MIT
pragma solidity 0.8.33;

import {Script, console} from "forge-std/Script.sol";
import {BaseDeployer} from "./BaseDeployer.s.sol";
import {VampTokenEmitterUpgradeable} from "../src/VampTokenEmitterUpgradeable.sol";

contract DeployVampTokenEmitterUpgradeable is BaseDeployer {
    function run() external {
        uint256 deployerPrivateKey = _getPrivateKey();
        bytes32 _salt = _generateSalt();
        address owner = _getOwner();
        _deploy(_salt, deployerPrivateKey, owner);
    }

    function run(uint256 salt) external {
        uint256 deployerPrivateKey = _getPrivateKey();
        address owner = _getOwner();
        bytes32 _salt = bytes32(salt);
        _deploy(_salt, deployerPrivateKey, owner);
    }

    function _deploy(bytes32 salt, uint256 deployerPrivateKey, address owner) internal {
        for (uint256 i = 0; i < networks.length; i++) {
            NetworkConfig memory config = networks[i];
            console.log("Deploying VampTokenEmitterUpgradeable to:", config.name);

            vm.createSelectFork(config.rpcUrl);
            vm.startBroadcast(deployerPrivateKey);

            address contractAddress = address(new VampTokenEmitterUpgradeable{salt: salt}());
            address computedAddress = _computeCreate2Address(
                salt, hashInitCode(abi.encodePacked(type(VampTokenEmitterUpgradeable).creationCode, abi.encode(owner)))
            );
            require(contractAddress == computedAddress, "Contract address mismatch");
            console.log("VampTokenEmitterUpgradeable deployed to:", contractAddress);

            vm.stopBroadcast();
        }
    }
}
