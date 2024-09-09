// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import "forge-std/StdJson.sol";

contract SubmitResult is Script, Utils {

    JobManager public jobManager;

    function submitResult(bytes calldata resultWithMetadata, bytes calldata signature) public {
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
        jobManager.submitResult(resultWithMetadata, signature);
        vm.stopBroadcast();
        console.log("Result submitted!");
    }

}
