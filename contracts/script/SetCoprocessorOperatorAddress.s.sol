// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/JobManager.sol";
import "forge-std/StdJson.sol";

contract SetCoprocessorOperatorAddress is Script, Utils {

    JobManager public jobManager;

    function setCoprocessorOperatorAddress(address coprocessorOperator) public {
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
        jobManager.setCoprocessorOperator(coprocessorOperator);
        vm.stopBroadcast();
        console.log("Coprocessor operator address set to: ", coprocessorOperator);
    }

}