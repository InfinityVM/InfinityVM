// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Consumer} from "./Consumer.sol";

abstract contract StatefulConsumer is Consumer {
    // Struct passed to zkVM program as onchain input
    struct StatefulAppOnchainInput {
        bytes32 inputStateRoot;
        bytes result;
    }

    // Struct returned by zkVM program as the result
    struct StatefulAppResult {
        bytes32 outputStateRoot;
        bytes result;
    }

    bytes32 public latestStateRoot;

    constructor(address __jobManager, uint64 _initialMaxNonce, bytes32 _latestStateRoot)
        Consumer(__jobManager, _initialMaxNonce)
    {
        latestStateRoot = _latestStateRoot;
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
