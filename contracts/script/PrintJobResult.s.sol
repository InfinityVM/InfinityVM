// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "forge-std/StdJson.sol";

contract PrintJobResult is Script, Utils {

    MockConsumer public consumer;

    function printJobResult(uint32 jobID) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        consumer = MockConsumer(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.consumer"
            )
        );

        bytes memory result = consumer.getJobResult(jobID);
        console.log("Result for job ID ", jobID, ": ");
        console.logBytes(result);
    }

}
