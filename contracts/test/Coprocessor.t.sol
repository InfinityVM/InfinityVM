// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
// import {Counter} from "../src/Counter.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "./mocks/MockConsumer.sol";

contract CounterTest is Test {
    JobManager public jobManager;
    MockConsumer public consumer;

    function setUp() public {
        // counter = new Counter();
        // counter.setNumber(0);
        jobManager = new JobManager(address(0), address(1));
        consumer = new MockConsumer(address(jobManager));
    }

    function test_Consumer_GetBalance() public {
        assertEq(consumer.getBalance(address(0)), 0);
    }
}
