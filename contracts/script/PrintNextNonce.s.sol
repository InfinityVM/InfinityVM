// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "forge-std/StdJson.sol";

contract PrintNextNonce is Script, Utils {

    MockConsumer public consumer;

    function printNextNonce() public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        consumer = MockConsumer(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.consumer"
            )
        );

        uint64 nextNone = consumer.getNextNonce();
        console.log("Next nonce: ", nextNone);
    }

}
