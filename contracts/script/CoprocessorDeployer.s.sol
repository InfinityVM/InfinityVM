// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import {Utils} from "./utils/Utils.sol";

// To deploy and verify:
// forge script script/CoprocessorDeployer.s.sol:CoprocessorDeployer --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast -vvvv
contract CoprocessorDeployer is Script, Utils {

    JobManager public jobManager;
    MockConsumer public consumer;

    function run() public {
        vm.startBroadcast();
        _deployCoprocessorContracts();
        vm.stopBroadcast();
    }

    function _deployCoprocessorContracts() internal {
        jobManager = new JobManager(msg.sender, msg.sender);
        consumer = new MockConsumer(address(jobManager));

        // WRITE JSON DATA
        string memory parent_object = "parent object";

        string memory deployed_addresses = "addresses";
        vm.serializeAddress(
            deployed_addresses,
            "jobManager",
            address(jobManager)
        );

        string memory deployed_addresses_output = vm.serializeAddress(
            deployed_addresses,
            "consumer",
            address(consumer)
        );

        // serialize all the data
        string memory finalJson = vm.serializeString(
            parent_object,
            "deployed_addresses",
            deployed_addresses_output
        );

        writeOutput(finalJson, "coprocessor_deployment_output");
    }

}
