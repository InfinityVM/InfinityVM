// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "forge-std/StdJson.sol";

contract PrintJobResult is Script, Utils {

    MockConsumer public consumer;

    function printJobResult(uint64 nonce) public {
        consumer = MockConsumer(0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6);
        bytes32 jobID = keccak256(abi.encodePacked(nonce, address(consumer)));

        bytes memory result = consumer.getJobResult(jobID);
        console.log("Job ID: ");
        console.logBytes32(jobID);
        console.log("Result: ");
        console.logBytes(result);

        uint256 balance = consumer.getBalance(0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6);
        console.log("Balance: ");
        console.log(balance);
    }

}
