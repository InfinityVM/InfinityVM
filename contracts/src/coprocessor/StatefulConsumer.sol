// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Consumer} from "./Consumer.sol";

abstract contract StatefulConsumer is Consumer {
    // Struct passed into zkVM program as input
    struct StatefulProgramInput {
        bytes32 previousStateHash;
        bytes input;
    }

    // Struct returned by zkVM program as the result
    struct StatefulProgramResult {
        bytes32 nextStateRootHash;
        bytes result;
    }

    bytes32 public latestStateRootHash;

    constructor(address __jobManager, uint64 _initialMaxNonce, bytes32 _latestStateRootHash)
        Consumer(__jobManager, _initialMaxNonce)
    {
        latestStateRootHash = _latestStateRootHash;
    }

    function getLatestStateRootHash() public view returns (bytes32) {
        return latestStateRootHash;
    }    

    // Override receiveResult to check state root provided in input
    function receiveResult(bytes32 jobID, bytes calldata result) external override onlyJobManager {
        StatefulProgramResult memory statefulResult = abi.decode(result, (StatefulProgramResult));

        bytes memory encodedInput = getProgramInputsForJob(jobID);
        StatefulProgramInput memory statefulInput = abi.decode(encodedInput, (StatefulProgramInput));
        require(statefulInput.previousStateHash == latestStateRootHash, "Invalid state hash passed as input");

        // Update the state root hash
        latestStateRootHash = statefulResult.nextStateRootHash;

        // Only pass in the actual result (not including nextStateRootHash) to _receiveResult()
        _receiveResult(jobID, statefulResult.result);
    }
}