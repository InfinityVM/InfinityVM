// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Consumer} from "./Consumer.sol";

abstract contract StatefulConsumer is Consumer {
    // Struct returned by zkVM program as the result
    struct StatefulProgramResult {
        bytes32 stateOutputHash;
        bytes result;
    }

    bytes32 public latestStateOutputHash;

    constructor(address __jobManager, uint64 _initialMaxNonce, bytes32 _latestStateOutputHash)
        Consumer(__jobManager, _initialMaxNonce)
    {
        latestStateOutputHash = _latestStateOutputHash;
    }

    function getLatestStateOutputHash() public view returns (bytes32) {
        return latestStateOutputHash;
    }

    // Override receiveResult to check state root provided in input
    function receiveResult(bytes32 jobID, bytes calldata result) external override onlyJobManager {
        StatefulProgramResult memory statefulResult = abi.decode(result, (StatefulProgramResult));

        bytes32 stateInputHash = getStateInputHashForJob(jobID);
        require(stateInputHash == latestStateOutputHash, "Invalid state input hash passed");

        // Update the state root hash
        latestStateOutputHash = statefulResult.stateOutputHash;

        // Only pass in the actual result (not including stateOutputHash) to _receiveResult()
        _receiveResult(jobID, statefulResult.result);
    }
}
