// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "forge-std/Test.sol";
import {FleekToken} from "../src/token/FleekToken.sol";
import {Staking} from "../src/staking/Staking.sol";

contract StakingTest is Test{
    FleekToken token;
    Staking staking;

    address bob = address(0x1);
    address mary = address(0x2);
    address dalton = address(0x3);
    address slasher = address(0x4);

    function setUp() public {
        // Deploy with initial supply of 1 million
        token = new FleekToken(1000000);

        // Deploy staking contract
        staking = new Staking();
        staking.initialize(address(this), address(token), 100, 1, 1,1);
        staking.setSlasher(slasher, true);

        //mint bob/mary some tokens
        token.mint(bob, 1000);
        token.mint(mary, 1000);
    }

    function testStakeParameters() public {
        assertEq(staking.minimumNodeStake(), 100);
        assertEq(staking.lockTime(), 1);
        assertEq(staking.nodeElegibiliyPeriod(), 1);
        assertEq(staking.protocolPercentage(), 1);

        staking.setMinimumNodeStake(200);
        staking.setLockTime(2);
        staking.setNodeElegibilityPeriod(2);
        staking.setProtocolPercentage(2);

        assertEq(staking.minimumNodeStake(), 200);
        assertEq(staking.lockTime(), 2);
        assertEq(staking.nodeElegibiliyPeriod(), 2);
        assertEq(staking.protocolPercentage(), 2);
    }

    function testStake() public {
        //Make sure bob has a blanace of 1000
        uint256 balance = token.balanceOf(bob);
        assertEq(balance, 1000);

        vm.startPrank(bob);
        //Have bob stake 500 tokens
        token.approve(address(staking), 500);
        staking.stake(500);

        uint256 bobsStake = staking.getNodeStakedTokens(address(bob));
        assertEq(bobsStake, 500);
        
        uint256 bobsNewBalance = token.balanceOf(bob);
        assertEq(bobsNewBalance, 500);

        uint256 elegibleAt = staking.getEligibleBlock(address(bob));
        assertEq(elegibleAt, block.number + 1);

        uint256 lockedTokens = staking.getLockedTokens(address(bob));
        assertEq(lockedTokens, 0);

        staking.unstake(500);

        uint256 unstakedBalance = staking.getNodeStakedTokens(address(bob));
        assertEq(unstakedBalance, 0);

        uint256 newLocked = staking.getLockedTokens(address(bob));
        uint256 lockDate = staking.getLockedUntil(address(bob));
        uint256 bobBalance = token.balanceOf(bob);
        elegibleAt = staking.getEligibleBlock(address(bob));

        assertEq(newLocked, 500);
        assertEq(lockDate, block.number + 1);
        assertEq (bobBalance, 500);
        assertEq(elegibleAt, 0);

        vm.expectRevert("No tokens eligable for withdraw");
        staking.withdraw();

        vm.roll(block.number + 1);

        staking.withdraw();

        lockedTokens = staking.getLockedTokens(address(bob));
        unstakedBalance = staking.getNodeStakedTokens(address(bob));
        bobBalance = token.balanceOf(bob);

        assertEq(lockedTokens, 0);
        assertEq(unstakedBalance, 0);
        assertEq(bobBalance, 1000);
    }

    function testSlash() public {
        vm.startPrank(bob);

        //Have bob stake 500 tokens but unstake 250 so he should have 250 locked/250 staked
        token.approve(address(staking), 500);
        staking.stake(500);
        staking.unstake(250);

        uint256 bobStaked = staking.getNodeStakedTokens(bob);
        uint256 bobLocked = staking.getLockedTokens(bob);
        uint256 bobsEligibleAt = staking.getEligibleBlock(bob);

        assertEq(bobStaked, 250);
        assertEq(bobLocked, 250);
        assertEq(bobsEligibleAt, block.number + 1);

        vm.stopPrank();

        vm.startPrank(mary);
        token.approve(address(staking), 500);
        staking.stake(500);
        staking.unstake(250);

        vm.stopPrank();

        //Have slasher slash bob for 401 tokens
        //This should take all of his staked tokens and 150 of his locked tokens
        //leaving him with just 99 tokens locked and 0 staked
        vm.startPrank(slasher);

        staking.slash(bob, 401, 250, dalton);

        bobStaked = staking.getNodeStakedTokens(bob);
        bobLocked = staking.getLockedTokens(bob);
        bobsEligibleAt = staking.getEligibleBlock(bob);
        uint256 daltonsBalance = token.balanceOf(dalton);

        assertEq(bobStaked, 0);
        assertEq(bobLocked, 99);
        assertEq(bobsEligibleAt, 0);
        assertEq(daltonsBalance, 250);

        //Have slasher slash mary for 150 tokens, that should still make her eligable as a node
        //then have him slash for 2 more and make sure  her eligiblity is removed
        staking.slash(mary, 150, 100, dalton);

        uint256 marysEligibleAt = staking.getEligibleBlock(mary);
        uint256 marysStaked = staking.getNodeStakedTokens(mary);
        uint256 marysLocked = staking.getLockedTokens(mary);

        assertEq(marysEligibleAt, block.number + 1);
        assertEq(marysStaked, 100);
        assertEq(marysLocked, 250);

        staking.slash(mary, 2, 1, dalton);

        marysEligibleAt = staking.getEligibleBlock(mary);
        marysStaked = staking.getNodeStakedTokens(mary);
        marysLocked = staking.getLockedTokens(mary);

        assertEq(marysEligibleAt, 0);
        assertEq(marysStaked, 98);
        assertEq(marysLocked, 250);

        //Slash more tokens than mary has and make sure everyone everything adjust right
        staking.slash(mary, 3000, 2000, dalton);

        marysEligibleAt = staking.getEligibleBlock(mary);
        marysStaked = staking.getNodeStakedTokens(mary);
        marysLocked = staking.getLockedTokens(mary);
        //Daltons balance should be 250+100+1+98+250 = 699
        daltonsBalance = token.balanceOf(dalton);

        assertEq(marysEligibleAt, 0);
        assertEq(marysStaked, 0);
        assertEq(marysLocked, 0);
        assertEq(daltonsBalance, 699);

    }

}

