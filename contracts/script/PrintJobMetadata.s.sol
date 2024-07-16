// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/JobManager.sol";
import "forge-std/StdJson.sol";

contract PrintJobMetadata is Script, Utils {

    JobManager public jobManager;

    function printJobMetadata(uint32 jobID) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        jobManager = JobManager(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.jobManager"
            )
        );

        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        console.log("Job ID: ", jobID);
        console.log("Max cycles: ", jobMetadata.maxCycles);
        console.log("Caller: ", jobMetadata.caller);

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
