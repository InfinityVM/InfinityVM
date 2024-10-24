// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {MatchingGameConsumer} from "../src/matching-game/MatchingGameConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import {E2EMockERC20} from "../test/mocks/E2EMockERC20.sol";

// To deploy and verify:
// forge script MatchingGameDeployer.s.sol:MatchingGameDeployer --sig "deployMatchingGameContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract MatchingGameDeployer is Script, Utils {
    MatchingGameConsumer public consumer;
    
    function deployMatchingGameContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        address jobManager = stdJson.readAddress(
            coprocessorDeployedContracts,
            ".addresses.jobManager"
        );

        vm.startBroadcast();

        bytes32 initialLatestStateRoot = 0x0;
        consumer = new MatchingGameConsumer(jobManager, offchainRequestSigner, initialMaxNonce, initialLatestStateRoot);

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

            writeOutput(finalJson, "matching_game_deployment_output");
        }

        vm.stopBroadcast();
    }
}
