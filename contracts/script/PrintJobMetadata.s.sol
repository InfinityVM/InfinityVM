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
        jobManager = JobManager(0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512);

        consumerAddress = 0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6;
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
