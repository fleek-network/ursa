// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "forge-std/Test.sol";
import {FleekToken} from "../src/token/FleekToken.sol";
import {Staking} from "../src/staking/Staking.sol";
import {NodeRegistry} from "../src/registry/NodeRegistry.sol";


contract StakingTest is Test {
    FleekToken token;
    Staking staking;
    NodeRegistry nodeRegistry;

    address bob = address(0x1);
    address mary = address(0x2);
    address dalton = address(0x3);
    address slasher = address(0x4);

    uint256 bobNode = uint256(1);
    uint256 maryNode = uint256(2);
    uint256 daltonNode = uint256(3);

    function setUp() public {
        // Deploy with initial supply of 1 million
        token = new FleekToken(1000000);

        // Deploy staking contract
        staking = new Staking();
        staking.initialize(address(this), address(token), 100, 1, 1, 1);
        staking.setSlasher(slasher, true);

        // Deploy Node Registry
        nodeRegistry = new NodeRegistry();
        nodeRegistry.initialize(address(this), address(staking));

        staking.setNodeRegistryContract(address(nodeRegistry));


        //mint bob/mary some tokens
        token.mint(bob, 1000);
        token.mint(mary, 1000);

        vm.startPrank(bob);
        token.approve(address(staking), 1000);
        staking.stake(1000, bobNode);
        vm.stopPrank();

        vm.startPrank(mary);
        token.approve(address(staking),1000);
        staking.stake(1000, maryNode);
        vm.stopPrank();
    }

    function testWhitelist() public{
         vm.roll(block.number + 1);
        
        vm.startPrank(bob);

        staking.whitelistNode();
        bool isWhitelist = nodeRegistry.isWhitelisted(bobNode);

        assertEq(isWhitelist,true, "Whitelist failed");

        vm.stopPrank();
    }

    function testWhitelistFail() public {
        vm.startPrank(mary);

        vm.expectRevert("Node is not elegible");
        staking.whitelistNode();

        bool isWhitelist = nodeRegistry.isWhitelisted(maryNode);
        assertEq(isWhitelist,false, "Node should not be on whitelist");

        vm.roll(block.number + 1);

        staking.whitelistNode();
        isWhitelist = nodeRegistry.isWhitelisted(maryNode);
        assertEq(isWhitelist,true, "Whitelist failed");

        vm.stopPrank();
    }

    function testSlashOffWhitelist() public {
        vm.startPrank(bob);
        vm.roll(block.number + 1);

        staking.whitelistNode();
        bool isWhitelist = nodeRegistry.isWhitelisted(bobNode);

        assertEq(isWhitelist,true, "Whitelist failed");
              
        vm.stopPrank();
        //Now that bob is whitelisted, slash him under the min value and make sure he is removed from whitelist

        vm.startPrank(slasher);

        staking.slash(bob, 500, 250, dalton);
        
        isWhitelist = nodeRegistry.isWhitelisted(bobNode);

        assertEq(isWhitelist, true, "Removed from whitelist with still above min stake");

        //slash the rest
        staking.slash(bob, 500, 250, dalton);
        
        isWhitelist = nodeRegistry.isWhitelisted(bobNode);

        assertEq(isWhitelist, false, "Not removed from whitelist after being slashed under min value");
  
    }

}