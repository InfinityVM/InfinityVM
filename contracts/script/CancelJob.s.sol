// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/JobManager.sol";
import "forge-std/StdJson.sol";

contract CancelJob is Script, Utils {

    JobManager public jobManager;

    function cancelJob(bytes32 jobID) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        jobManager = JobManager(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.jobManager"
            )
        );

        vm.startBroadcast();
        jobManager.cancelJob(jobID);
        vm.stopBroadcast();
        console.log("Job cancelled with job ID: ");
        console.logBytes32(jobID);
    }

}
