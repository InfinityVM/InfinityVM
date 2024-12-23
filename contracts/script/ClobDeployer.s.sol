// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import "forge-std/StdJson.sol";
import {ClobConsumer} from "../src/clob/ClobConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import {E2EMockERC20} from "../test/mocks/E2EMockERC20.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";

// To deploy and verify:
// forge script ClobDeployer.s.sol:ClobDeployer --sig "deployClobContracts(address offchainRequestSigner, uint64 initialMaxNonce, bool writeJson)" $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract ClobDeployer is Script, Utils {
    ClobConsumer public consumerImplementation;
    ClobConsumer public consumer;
    ProxyAdmin public proxyAdmin;
    E2EMockERC20 public baseToken;
    E2EMockERC20 public quoteToken;

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
        
        // Deploy implementation contract
        consumerImplementation = new ClobConsumer(baseToken, quoteToken);

        // Deploy ProxyAdmin
        proxyAdmin = new ProxyAdmin();

        // Prepare initialization data
        bytes32 initialLatestStateRoot = 0x0;
        bytes memory initData = abi.encodeWithSelector(
            ClobConsumer.initialize.selector,
            msg.sender, // initial owner
            jobManager,
            initialMaxNonce,
            initialLatestStateRoot,
            offchainRequestSigner
        );

        // Deploy proxy contract
        TransparentUpgradeableProxy proxyContract = new TransparentUpgradeableProxy(
            address(consumerImplementation),
            address(proxyAdmin),
            initData
        );

        consumer = ClobConsumer(address(proxyContract));

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

            vm.serializeAddress(
                deployed_addresses,
                "proxyAdmin",
                address(proxyAdmin)
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
