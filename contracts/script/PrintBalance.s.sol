// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Utils} from "./utils/Utils.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "forge-std/StdJson.sol";

contract PrintBalance is Script, Utils {

    MockConsumer public consumer;

    function printBalance(address addr) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        consumer = MockConsumer(
            stdJson.readAddress(
                coprocessorDeployedContracts,
                ".addresses.consumer"
            )
        );

        uint256 balance = consumer.getBalance(addr);
        console.log("Balance for the given address is: ", balance);
    }

}
