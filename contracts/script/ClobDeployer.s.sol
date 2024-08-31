// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/JobManager.sol";
import {IJobManager} from "../src/IJobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {ClobConsumer} from "../src/ClobConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";
import {MockERC20} from "../test/mocks/MockERC20.sol";

// To deploy and verify:
// forge script ClobDeployer.s.sol:ClobDeployer --sig "deployClobContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce)" $RELAYER $COPROCESSOR_OPERATOR $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract ClobDeployer is Script, Utils {
    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    ClobConsumer public consumer;
    MockERC20 baseToken;
    MockERC20 quoteToken;

    function deployClobContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce) public {
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
                        jobManager.initializeJobManager.selector, msg.sender, relayer, coprocessorOperator
                    )
                )
            )
        );

        baseToken = new MockERC20("Token A", "WETH");
        quoteToken = new MockERC20("Token B", "USDC");

        bytes32 initialLatestStateRootHash = 0x0;
        consumer = new ClobConsumer(address(jobManager), offchainRequestSigner, initialMaxNonce, baseToken, quoteToken, initialLatestStateRootHash);

        vm.stopBroadcast();
    }
}
