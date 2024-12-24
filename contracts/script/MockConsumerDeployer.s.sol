// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";

// To deploy and verify:
// forge script MockConsumerDeployer.s.sol:MockConsumerDeployer --sig "deployMockConsumerContracts(address jobManager, address offchainSigner, uint64 initialMaxNonce, bool writeJson)" $JOB_MANAGER $OFFCHAIN_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract MockConsumerDeployer is Script, Utils {
    MockConsumer public consumerImplementation;
    MockConsumer public consumer;
    ProxyAdmin public proxyAdmin;

    function deployMockConsumerContracts(
        address offchainSigner, 
        uint64 initialMaxNonce, 
        bool writeJson
    ) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        address jobManager = stdJson.readAddress(
            coprocessorDeployedContracts,
            ".addresses.jobManager"
        );

        vm.startBroadcast();

        // Deploy ProxyAdmin
        proxyAdmin = new ProxyAdmin();

        // Deploy implementation contract
        consumerImplementation = new MockConsumer();

        // Deploy proxy contract
        consumer = MockConsumer(address(new TransparentUpgradeableProxy(
            address(consumerImplementation),
            address(proxyAdmin),
            abi.encodeWithSelector(
                MockConsumer.initialize.selector,
                msg.sender, // initial owner
                jobManager,
                initialMaxNonce,
                offchainSigner
            )
        )));

        if (writeJson) {
            string memory parent_object = "parent object";
            string memory deployed_addresses = "addresses";

            // Serialize implementation, proxy, and admin addresses
            vm.serializeAddress(
                deployed_addresses,
                "consumerImplementation",
                address(consumerImplementation)
            );
            
            vm.serializeAddress(
                deployed_addresses,
                "consumer",
                address(consumer)
            );

            string memory deployed_addresses_output = vm.serializeAddress(
                deployed_addresses,
                "proxyAdmin",
                address(proxyAdmin)
            );

            // Serialize all the data
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
