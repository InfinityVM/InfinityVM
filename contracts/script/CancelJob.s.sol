// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import "forge-std/StdJson.sol";

contract CancelJob is Script, Utils {

    JobManager public jobManager;
    address public consumerAddress;

    function cancelJob(uint64 nonce) public {
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

        vm.startBroadcast();
        jobManager.cancelJob(jobID);
        vm.stopBroadcast();
        console.log("Job cancelled with job ID: ");
        console.logBytes32(jobID);
    }

}
