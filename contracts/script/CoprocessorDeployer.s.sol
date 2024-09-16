// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {IJobManager} from "../src/coprocessor/IJobManager.sol";
import {Utils} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator, bool writeJson)" $RELAYER $COPROCESSOR_OPERATOR $WRITE_JSON --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract CoprocessorDeployer is Script, Utils {
    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;

    function deployCoprocessorContracts(address relayer, address coprocessorOperator, bool writeJson) public {
        vm.startBroadcast();
        // deploy proxy admin for ability to upgrade proxy contracts
        coprocessorProxyAdmin = new ProxyAdmin();

        jobManagerImplementation = new JobManager();
        jobManager = JobManager(
            address(
                new TransparentUpgradeableProxy(
                    address(jobManagerImplementation),
                    address(coprocessorProxyAdmin),
                    abi.encodeWithSelector(
                        jobManager.initializeJobManager.selector,
                        msg.sender,
                        relayer,
                        coprocessorOperator
                    )
                )
            )
        );

        if (writeJson) {
            // WRITE JSON DATA
            string memory parent_object = "parent object";

            string memory deployed_addresses = "addresses";
            vm.serializeAddress(
                deployed_addresses,
                "jobManager",
                address(jobManager)
            );

            vm.serializeAddress(
                deployed_addresses,
                "jobManagerImplementation",
                address(jobManagerImplementation)
            );

            string memory deployed_addresses_output = vm.serializeAddress(
                deployed_addresses,
                "coprocessorProxyAdmin",
                address(coprocessorProxyAdmin)
            );

            // serialize all the data
            string memory finalJson = vm.serializeString(
                parent_object,
                deployed_addresses,
                deployed_addresses_output
            );

            writeOutput(finalJson, "coprocessor_deployment_output");
        }

        vm.stopBroadcast();
    }
}
