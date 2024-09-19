// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import "forge-std/StdJson.sol";

contract PrintJobMetadata is Script, Utils {

    JobManager public jobManager;
    address public consumerAddress;

    function printJobMetadata(uint64 nonce) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        jobManager = JobManager(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.jobManager"
            )
        );

        consumerAddress = stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.consumer"
            );
        bytes32 jobID = keccak256(abi.encodePacked(nonce, consumerAddress));

        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        console.log("Job ID: ");
        console.logBytes32(jobID);
        console.log("Max cycles: ", jobMetadata.maxCycles);
        console.log("Consumer: ", jobMetadata.consumer);

        if (jobMetadata.status == 1) {
            console.log("Status: ", "Pending");
        } else if (jobMetadata.status == 2) {
            console.log("Status: ", "Cancelled");
        } else if (jobMetadata.status == 3) {
            console.log("Status: ", "Completed");
        } else {
            console.log("Status: ", "Unknown");
        }

        console.log("Program ID: ");
        console.logBytes(jobMetadata.programID);
    }

}
