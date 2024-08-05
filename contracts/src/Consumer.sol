// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    mapping(uint32 => bytes) internal jobIDToProgramInput;

    constructor(address __jobManager) {
        _jobManager = JobManager(__jobManager);
    }

    modifier onlyJobManager() {
        require(
            msg.sender == address(_jobManager),
            "Consumer.onlyJobManager: caller is not the job manager"
        );
        _;
    }

    function getProgramInputsForJob(uint32 jobID) public view returns (bytes memory) {
        return jobIDToProgramInput[jobID];
    }

    function setProgramInputsForJob(uint32 jobID, bytes memory programInput) public onlyJobManager() {
        jobIDToProgramInput[jobID] = programInput;
    }

    function requestJob(
        bytes memory programID,
        bytes memory programInput,
        uint64 maxCycles
    ) internal returns (uint32) {
        uint32 jobID = _jobManager.createJob(programID, programInput, maxCycles);
        jobIDToProgramInput[jobID] = programInput;
        return jobID;
    }

    function cancelJob(uint32 jobID) internal {
        _jobManager.cancelJob(jobID);
    }

    function receiveResult(uint32 jobID, bytes calldata result) external onlyJobManager {
        _receiveResult(jobID, result);
    }

    // This function must be overridden by the app-specific Consumer contract
    // to decode the coprocessor result into any app-specific struct and
    // perform app-specific logic using the result
    function _receiveResult(uint32 jobID, bytes memory result) internal virtual;
}
