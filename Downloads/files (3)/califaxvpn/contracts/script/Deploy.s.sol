// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/CalifaxIdentity.sol";
import "../src/CalifaxAccess.sol";
import "../src/CalifaxCanary.sol";
import "../src/CalifaxStaking.sol";

/// @title Deploy - Deploys all Califax contracts in dependency order
contract Deploy is Script {
    function run() external {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");

        vm.startBroadcast(deployerKey);

        // 1. Identity registry (no constructor args)
        CalifaxIdentity identity = new CalifaxIdentity();
        console.log("CalifaxIdentity deployed at:", address(identity));

        // 2. Access control (needs Identity address)
        CalifaxAccess access = new CalifaxAccess(address(identity));
        console.log("CalifaxAccess deployed at:", address(access));

        // 3. Warrant canary (24-hour chirp interval = 86400 seconds)
        CalifaxCanary canary = new CalifaxCanary(86400);
        console.log("CalifaxCanary deployed at:", address(canary));

        // 4. Staking (0.1 ETH minimum stake, 5% slash = 500 basis points)
        CalifaxStaking staking = new CalifaxStaking(0.1 ether, 500);
        console.log("CalifaxStaking deployed at:", address(staking));

        vm.stopBroadcast();

        console.log("--- All contracts deployed ---");
    }
}
