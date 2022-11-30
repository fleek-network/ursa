// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "forge-std/Script.sol";
import "../src/token/FleekToken.sol"; 

contract DeployToken is Script {
    function setUp() public {}

    function run() public {
        vm.broadcast();

        FleekToken token = new FleekToken(1000000000000000);

        vm.stopBroadcast();
    }
}