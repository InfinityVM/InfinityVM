// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import "forge-std/StdJson.sol";

contract PrintCoprocessorOperatorAddress is Script, Utils {

    JobManager public jobManager;

    function printCoprocessorOperatorAddress() public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        jobManager = JobManager(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.jobManager"
            )
        );

        address coprocessorOperator = jobManager.getCoprocessorOperator();
        console.log("Coprocessor operator address: ", coprocessorOperator);
    }

}
