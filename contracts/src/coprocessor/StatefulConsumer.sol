// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Consumer} from "./Consumer.sol";

abstract contract StatefulConsumer is Consumer {
    // Struct returned by zkVM program as the result
    struct StatefulProgramResult {
        bytes32 nextStateHash;
        bytes result;
    }

    bytes32 public latestStateHash;

    constructor(address __jobManager, uint64 _initialMaxNonce, bytes32 _latestStateHash)
        Consumer(__jobManager, _initialMaxNonce)
    {
        latestStateHash = _latestStateHash;
    }

    function getLatestStateHash() public view returns (bytes32) {
        return latestStateHash;
    }

    // Override receiveResult to check state root provided in input
    function receiveResult(bytes32 jobID, bytes calldata result) external override onlyJobManager {
        StatefulProgramResult memory statefulResult = abi.decode(result, (StatefulProgramResult));

        bytes32 inputPreviousStateHash = getProgramStateHashForJob(jobID);
        require(inputPreviousStateHash == latestStateHash, "Invalid state hash passed as input");

        // Update the state root hash
        latestStateHash = statefulResult.nextStateHash;

        // Only pass in the actual result (not including nextStateHash) to _receiveResult()
        _receiveResult(jobID, statefulResult.result);
    }
}
