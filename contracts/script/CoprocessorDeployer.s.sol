// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/JobManager.sol";
import {IJobManager} from "../src/IJobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";
import "./utils/Utils.sol";

// To deploy and verify:
// forge script script/CoprocessorDeployer.s.sol:CoprocessorDeployer --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast -vvvv
contract CoprocessorDeployer is Script, Utils {

    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    MockConsumer public consumer;

    function run() public {
        vm.startBroadcast();
        _deployCoprocessorContracts();
        vm.stopBroadcast();
    }

    function _deployCoprocessorContracts() internal {
        // deploy proxy admin for ability to upgrade proxy contracts
        coprocessorProxyAdmin = new ProxyAdmin();

        EmptyContract emptyContract = new EmptyContract();

        jobManager = JobManager(
            address(
                new TransparentUpgradeableProxy(
                    address(emptyContract),
                    address(coprocessorProxyAdmin),
                    ""
                )
            )
        );

        jobManagerImplementation = new JobManager(msg.sender, msg.sender);

        coprocessorProxyAdmin.upgradeAndCall(
            TransparentUpgradeableProxy(
                payable(address(jobManager))
            ),
            address(jobManagerImplementation),
            abi.encodeWithSelector(
                jobManager.initializeJobManager.selector,
                jobManager.owner()
            )
        );

        consumer = new MockConsumer(address(jobManager));

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
