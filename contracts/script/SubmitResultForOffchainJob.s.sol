// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import "forge-std/StdJson.sol";

contract SubmitResultForOffchainJob is Script, Utils {

    JobManager public jobManager;

    function submitResultForOffchainJob(bytes calldata offchainResultWithMetadata, bytes calldata signatureOnResult, bytes calldata jobRequest, bytes calldata signatureOnRequest) public {
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
        jobManager.submitResultForOffchainJob(offchainResultWithMetadata, signatureOnResult, jobRequest, signatureOnRequest, 0);
        vm.stopBroadcast();
        console.log("Result for offchain job submitted!");
    }

}
