// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Consumer} from "./Consumer.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";

abstract contract StatefulConsumer is Consumer {
    // Struct passed to zkVM program as onchain input
    struct StatefulAppOnchainInput {
        bytes32 inputStateRoot;
        bytes onchainInput;
    }

    // Struct returned by zkVM program as the result
    struct StatefulAppResult {
        bytes32 outputStateRoot;
        bytes result;
    }

    bytes32 public latestStateRoot;
    // Storage gap for upgradeability.
    uint256[49] private __GAP;

    function initialize(
        address initialOwner,
        address jobManager,
        uint64 initialMaxNonce,
        bytes32 initialStateRoot
    ) public virtual onlyInitializing {
        // Call parent initializer
        Consumer.initialize(initialOwner, jobManager, initialMaxNonce);
        
        // Initialize StatefulConsumer state
        latestStateRoot = initialStateRoot;
    }

    function getLatestStateRoot() public view returns (bytes32) {
        return latestStateRoot;
    }

    // Override receiveResult to check state root provided in input
    function receiveResult(bytes32 jobID, bytes calldata result) external override onlyJobManager {
        StatefulAppResult memory statefulResult = abi.decode(result, (StatefulAppResult));

        bytes memory onchainInput = getOnchainInputForJob(jobID);
        StatefulAppOnchainInput memory statefulInput = abi.decode(onchainInput, (StatefulAppOnchainInput));

        require(statefulInput.inputStateRoot == latestStateRoot, "Invalid state root passed as input");

        // Update the latest state root
        latestStateRoot = statefulResult.outputStateRoot;

        // Only pass in the actual result (not including outputStateRoot) to _receiveResult()
        _receiveResult(jobID, statefulResult.result);
    }
}
