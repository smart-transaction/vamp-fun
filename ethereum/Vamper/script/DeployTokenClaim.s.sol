// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import {Script, console} from "forge-std/Script.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {BaseDeployer} from "./BaseDeployer.s.sol";
import {TokenClaim} from "../src/TokenClaim.sol";

contract DeployTokenClaim is BaseDeployer {
    function run(uint256 salt, uint256 feeWei) external {
        uint256 deployerPrivateKey = _getPrivateKey();
        bytes32 _salt = bytes32(salt);
        _deploy(_salt, deployerPrivateKey, feeWei);
    }

    function _deploy(bytes32 salt, uint256 deployerPrivateKey, uint256 feeWei) internal {
        for (uint256 i = 0; i < networks.length; i++) {
            NetworkConfig memory config = networks[i];
            console.log("Deploying TokenClaim to:", config.name);

            vm.createSelectFork(config.rpcUrl);
            vm.startBroadcast(deployerPrivateKey);

            address deployer = vm.addr(deployerPrivateKey);
            address contractAddress = address(new TokenClaim{salt: salt}(deployer, feeWei));

            address create2Factory = 0x4e59b44847b379578588920cA78FbF26c0B4956C;
            bytes32 initCodeHash = keccak256(
                abi.encodePacked(
                    type(TokenClaim).creationCode,
                    abi.encode(deployer),
                    abi.encode(feeWei)
                )
            );

            address computedAddress = _computeCreate2Address(
                salt,
                initCodeHash,
                create2Factory
            );
            require(contractAddress == computedAddress, "Contract address mismatch");
            console.log("TokenClaim deployed to:", contractAddress);

            vm.stopBroadcast();
        }
    }
}
