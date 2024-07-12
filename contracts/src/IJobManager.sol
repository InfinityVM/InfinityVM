// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

// CONSTANTS
uint8 constant JOB_STATE_PENDING = 1; // We start from 1 to avoid the default value of 0 for an empty item in a mapping in Solidity
uint8 constant JOB_STATE_CANCELLED = 2;
uint8 constant JOB_STATE_COMPLETED = 3;

interface IJobManager {
    // EVENTS
    event JobCreated(uint32 indexed jobID, uint64 indexed maxCycles, bytes indexed programID, bytes programInput);
    event JobCancelled(uint32 indexed jobID);
    event JobCompleted(uint32 indexed jobID, bytes indexed result);

    // STRUCTS
    struct JobMetadata {
        bytes programID;
        uint64 maxCycles;
        address caller;
        uint8 status;
    }

    // FUNCTIONS
    function createJob(bytes calldata programID, bytes calldata programInput, uint64 maxCycles) external returns (uint32 jobID);
    function getJobMetadata(uint32 jobID) external view returns (JobMetadata memory);
    function cancelJob(uint32 jobID) external;
    function submitResult(bytes calldata resultWithMetadata, bytes calldata signature) external;
    function setRelayer(address _relayer) external;
}
