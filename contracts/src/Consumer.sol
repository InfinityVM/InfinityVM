// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    mapping(uint32 => bytes) internal jobIDToProgramInput;

    constructor(address jobManager) {
        _jobManager = JobManager(jobManager);
    }

    modifier onlyJobManager() {
        require(
            msg.sender == address(_jobManager),
            "Consumer.onlyJobManager: caller is not the job manager"
        );
        _;
    }

    function requestJob(
        bytes calldata programID,
        bytes calldata programInput
    ) internal returns (uint32) {
        uint32 jobID = _jobManager.createJob(programID, programInput);
        jobIDToProgramInput[jobID] = programInput;
        return jobID;
    }

    function getProgramInputsForJob(uint32 jobID) public view returns (bytes memory) {
        return jobIDToProgramInput[jobID];
    }

    // This function must be overriden by the app-specific Consumer contract
    function receiveResult(uint32 jobID, bytes calldata result) public virtual onlyJobManager {
        // Decode the coprocessor result into app-specific struct
        // ...
        // Perform app-specific logic using the result
        // ...
    }
}
