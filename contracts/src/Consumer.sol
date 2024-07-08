// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    mapping(uint32 => bytes) internal jobIDToInputs;

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
        bytes calldata inputs
    ) internal returns (uint32) {
        uint32 jobID = _jobManager.createJob(programID, inputs);
        jobIDToInputs[jobID] = inputs;
        return jobID;
    }

    function getJobInputs(uint32 jobID) public view returns (bytes memory) {
        return jobIDToInputs[jobID];
    }

    function receiveResult(uint32 jobID, bytes calldata result) public virtual onlyJobManager {
        // Decode the coprocessor result into app-specific struct

        // Perform app-specific logic using the result
    }
}
