// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {MatchingGameConsumer} from "../src/matching-game/MatchingGameConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import {E2EMockERC20} from "../test/mocks/E2EMockERC20.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";

// To deploy and verify:
// forge script MatchingGameDeployer.s.sol:MatchingGameDeployer --sig "deployMatchingGameContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract MatchingGameDeployer is Script, Utils {
    MatchingGameConsumer public consumerImplementation;
    MatchingGameConsumer public consumer;
    ProxyAdmin public proxyAdmin;
    
    function deployMatchingGameContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        address jobManager = stdJson.readAddress(
            coprocessorDeployedContracts,
            ".addresses.jobManager"
        );

        vm.startBroadcast();

        // Deploy implementation contract
        consumerImplementation = new MatchingGameConsumer();

        // Deploy ProxyAdmin
        proxyAdmin = new ProxyAdmin();

        bytes32 initialLatestStateRoot = 0x0;
        // Deploy proxy contract
        consumer = MatchingGameConsumer(address(new TransparentUpgradeableProxy(
            address(consumerImplementation),
            address(proxyAdmin),
            abi.encodeWithSelector(
                MatchingGameConsumer.initialize.selector,
                msg.sender, // initial owner
                jobManager,
                initialMaxNonce,
                initialLatestStateRoot,
                offchainRequestSigner
            )
        )));

        if (writeJson) {
            // WRITE JSON DATA
            string memory parent_object = "parent object";

            string memory deployed_addresses = "addresses";

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

            // serialize all the data
            string memory finalJson = vm.serializeString(
                parent_object,
                deployed_addresses,
                deployed_addresses_output
            );

            writeOutput(finalJson, "matching_game_deployment_output");
        }

        vm.stopBroadcast();
    }
}
