// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {ClobConsumer} from "../src/clob/ClobConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import {E2EMockERC20} from "../test/mocks/E2EMockERC20.sol";

// To deploy and verify:
// forge script ClobDeployer.s.sol:ClobDeployer --sig "deployClobContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract ClobDeployer is Script, Utils {
    ClobConsumer public consumer;
    E2EMockERC20 baseToken;
    E2EMockERC20 quoteToken;

    function deployClobContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson) public {
        string memory coprocessorDeployedContracts = readOutput(
            "coprocessor_deployment_output"
        );

        address jobManager = stdJson.readAddress(
            coprocessorDeployedContracts,
            ".addresses.jobManager"
        );

        vm.startBroadcast();

        baseToken = new E2EMockERC20("Token A", "WETH");
        quoteToken = new E2EMockERC20("Token B", "USDC");

        bytes32 initialLatestStateHash = 0x0;
        consumer = new ClobConsumer(jobManager, offchainRequestSigner, initialMaxNonce, baseToken, quoteToken, initialLatestStateHash);

        if (writeJson) {
            // WRITE JSON DATA
            string memory parent_object = "parent object";

            string memory deployed_addresses = "addresses";

            vm.serializeAddress(
                deployed_addresses,
                "consumer",
                address(consumer)
            );

            vm.serializeAddress(
                deployed_addresses,
                "baseToken",
                address(baseToken)
            );

            string memory deployed_addresses_output = vm.serializeAddress(
                deployed_addresses,
                "quoteToken",
                address(quoteToken)
            );

            // serialize all the data
            string memory finalJson = vm.serializeString(
                parent_object,
                deployed_addresses,
                deployed_addresses_output
            );

            writeOutput(finalJson, "clob_deployment_output");
        }

        vm.stopBroadcast();
    }
}
