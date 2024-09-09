// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "forge-std/StdJson.sol";

contract RequestJob is Script, Utils {

    MockConsumer public consumer;

    function requestJob(bytes calldata programID, address balanceAddr) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        consumer = MockConsumer(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.consumer"
            )
        );

        vm.startBroadcast();
        bytes32 jobID = consumer.requestBalance(programID, balanceAddr);
        vm.stopBroadcast();

        console.log("Requested balance for address: ", balanceAddr);
        console.log("Job ID: ");
        console.logBytes32(jobID);
    }

}
