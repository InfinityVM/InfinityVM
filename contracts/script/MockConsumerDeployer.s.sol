// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {IJobManager} from "../src/coprocessor/IJobManager.sol";
import {Utils} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script MockConsumerDeployer.s.sol:MockConsumerDeployer --sig "deployMockConsumerContracts(address jobManager, address offchainSigner, uint64 initialMaxNonce, bool writeJson)" $JOB_MANAGER $OFFCHAIN_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract MockConsumerDeployer is Script, Utils {
    MockConsumer public consumer;

    function deployMockConsumerContracts(address offchainSigner, uint64 initialMaxNonce, bool writeJson) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        address jobManager = stdJson.readAddress(
            coprocessorDeployedContracts,
            ".addresses.jobManager"
        );

        vm.startBroadcast();

        consumer = new MockConsumer(jobManager, offchainSigner, initialMaxNonce);

        if (writeJson) {
            // WRITE JSON DATA
            string memory parent_object = "parent object";

            string memory deployed_addresses = "addresses";

            string memory deployed_addresses_output = vm.serializeAddress(
                deployed_addresses,
                "consumer",
                address(consumer)
            );

            // serialize all the data
            string memory finalJson = vm.serializeString(
                parent_object,
                deployed_addresses,
                deployed_addresses_output
            );

            writeOutput(finalJson, "mock_consumer_deployment_output");
        }

        vm.stopBroadcast();
    }
}
