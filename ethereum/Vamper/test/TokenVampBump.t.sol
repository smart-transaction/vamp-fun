// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import "forge-std/Test.sol";

// OZ proxy
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

import "../src/TokenVampBump.sol";
import "../src/MockERC20MetadataURI.sol";

contract VampTokenEmitterUpgradeableTest is Test {
    TokenVampBump internal vamp;
    MockERC20MetadataURI internal token;

    address internal owner  = address(0xABCD);
    address internal user  = address(0xBEEF);

    uint256 internal feeWei = 1 gwei;

    // Re-declare the event so we can use vm.expectEmit with it
    event VampTokenIntent(
        uint256 chainId,
        uint256 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );

    function setUp() public {
        // Fund accounts
        vm.deal(owner, 10 ether);
        vm.deal(user, 10 ether);

        // Deploy mock token
        token = new MockERC20MetadataURI("Mock Token", "MOCK", "ipfs://mock-token-uri");

        // Deploy implementation
        vamp = new TokenVampBump(owner, feeWei);
    }

    function test_initialize_setsOwnerFeeNonce() public view {
        assertEq(vamp.feeWei(), feeWei);
        assertEq(vamp.nonce(), 0);
    }

    function test_setFee_owner_success() public {
        vm.prank(vamp.owner());
        vamp.setFee(2 gwei);
        assertEq(vamp.feeWei(), 2 gwei);
    }

    function test_setFee_non_owner_fail() public {
        vm.prank(user);
        vm.expectRevert();
        vamp.setFee(2 gwei);
    }

    function test_vampToken_revertsOnZeroToken() public {
        vm.prank(user);
        vm.expectRevert();
        vamp.vampToken{value: feeWei}(address(0));
    }

    function test_vampToken_revertsOnBadFee() public {
        vm.prank(user);
        vm.expectRevert();
        vamp.vampToken{value: feeWei - 1}(address(token));
    }

    function test_vampToken_transfersFee_emitsEvent_andIncrementsNonce() public {
        uint256 ownerBalBefore = vamp.owner().balance;

        // Expect emit with exact values
        // - chainId: block.chainid
        // - blockNumber: current block number at execution time
        // - intentId: keccak256(abi.encodePacked(chainid, address(thisProxy), caller, nonceBefore))
        // - caller: user
        // - token: mock token address
        // - name/symbol/tokenURI: from mock
        bytes32 expectedIntentId = keccak256(
            abi.encodePacked(block.chainid, address(vamp), user, uint64(0))
        );

        vm.expectEmit(true, true, true, true, address(vamp));
        emit VampTokenIntent(
            block.chainid,
            block.number,
            expectedIntentId,
            user,
            address(token),
            "Mock Token",
            "MOCK",
            "ipfs://mock-token-uri"
        );

        vm.prank(user);
        vamp.vampToken{value: feeWei}(address(token));

        // Owner received the fee
        assertEq(vamp.owner().balance - ownerBalBefore, feeWei);

        // Nonce increments
        assertEq(vamp.nonce(), 1);
    }

    function test_intentId_isDeterministicAndUniqueAcrossCalls() public {
        // Call #1 (nonce 0)
        bytes32 expected0 = keccak256(
            abi.encodePacked(block.chainid, address(vamp), user, uint64(0))
        );
        vm.prank(user);
        vm.recordLogs();
        vamp.vampToken{value: feeWei}(address(token));
        Vm.Log[] memory logs0 = vm.getRecordedLogs();
        bytes32 id0 = _extractIntentIdFromLogs(logs0);
        assertEq(id0, expected0);

        // Call #2 (nonce 1)
        bytes32 expected1 = keccak256(
            abi.encodePacked(block.chainid, address(vamp), user, uint64(1))
        );
        vm.prank(user);
        vm.recordLogs();
        vamp.vampToken{value: feeWei}(address(token));
        Vm.Log[] memory logs1 = vm.getRecordedLogs();
        bytes32 id1 = _extractIntentIdFromLogs(logs1);
        assertEq(id1, expected1);

        assertTrue(id0 != id1);
    }

    // --- helper: parse intentId from the VampTokenIntent log ---
    function _extractIntentIdFromLogs(Vm.Log[] memory logs) internal pure returns (bytes32) {
        // topic0 = keccak256("VampTokenIntent(uint256,uint256,bytes32,address,address,string,string,string)")
        bytes32 topic0 = keccak256(
            "VampTokenIntent(uint256,uint256,bytes32,address,address,string,string,string)"
        );

        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].topics.length > 0 && logs[i].topics[0] == topic0) {
                // Since we removed `indexed`, ALL args are in data (not in topics),
                // so we decode the whole payload.
                (
                    uint256 chainId,
                    uint256 blockNumber,
                    bytes32 intentId,
                    address caller,
                    address tokenAddr,
                    string memory tokenName,
                    string memory tokenSymbol,
                    string memory tokenURI
                ) = abi.decode(
                    logs[i].data,
                    (uint256, uint256, bytes32, address, address, string, string, string)
                );

                // silence unused warnings by referencing variables (optional)
                chainId; blockNumber; caller; tokenAddr; tokenName; tokenSymbol; tokenURI;

                return intentId;
            }
        }
        revert("VampTokenIntent log not found");
    }
}
