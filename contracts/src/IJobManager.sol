// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;

// CONSTANTS
uint8 constant JOB_STATE_PENDING = 0;
uint8 constant JOB_STATE_CANCELLED = 1;
uint8 constant JOB_STATE_COMPLETED = 2;

interface IJobManager {
    // EVENTS
    event JobCreated(uint32 indexed jobID, bytes programID, bytes inputs);
    event JobCancelled(uint32 indexed jobID);
    event JobCompleted(uint32 indexed jobID, bytes result);

    // STRUCTS
    struct JobMetadata {
        bytes programID;
        address caller;
        uint8 status;
    }

    // FUNCTIONS
    function createJob(bytes calldata programID, bytes calldata inputs) external returns (uint32 jobID);
    function getJobMetadata(uint32 jobID) external view returns (JobMetadata memory);
    function cancelJob(uint32 jobID) external;
    function submitResult(bytes calldata resultWithMetadata, bytes calldata signature) external;
    function setRelayer(address _relayer) external;
}
