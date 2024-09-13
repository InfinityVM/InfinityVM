// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    uint64 public maxNonce;
    mapping(bytes32 => bytes) internal jobIDToOnchainInput;
    mapping(bytes32 => bytes32) internal jobIDToStateHash;

    constructor(address __jobManager, uint64 _initialMaxNonce) {
        _jobManager = JobManager(__jobManager);
        maxNonce = _initialMaxNonce;
    }

    modifier onlyJobManager() {
        require(
            msg.sender == address(_jobManager),
            "Consumer.onlyJobManager: caller is not the job manager"
        );
        _;
    }

    function getOnchainInputForJob(bytes32 jobID) public view virtual returns (bytes memory) {
        return jobIDToOnchainInput[jobID];
    }

    function getStateHashForJob(bytes32 jobID) public view virtual returns (bytes32) {
        return jobIDToStateHash[jobID];
    }

    function getNextNonce() public view virtual returns (uint64) {
        return maxNonce + 1;
    }

    function setOnchainInputForJob(bytes32 jobID, bytes memory onchainInput) public virtual onlyJobManager() {
        jobIDToOnchainInput[jobID] = onchainInput;
    }

    function setStateHashForJob(bytes32 jobID, bytes32 stateHash) public virtual onlyJobManager() {
        jobIDToStateHash[jobID] = stateHash;
    }

    function updateLatestNonce(uint64 nonce) public virtual onlyJobManager() {
        if (nonce > maxNonce) {
            maxNonce = nonce;
        }
    }

    function requestJob(
        bytes memory programID,
        bytes memory onchainInput,
        uint64 maxCycles
    ) internal virtual returns (bytes32) {
        bytes32 jobID = _jobManager.createJob(getNextNonce(), programID, onchainInput, maxCycles);
        jobIDToOnchainInput[jobID] = onchainInput;
        return jobID;
    }

    function cancelJob(bytes32 jobID) internal virtual {
        _jobManager.cancelJob(jobID);
    }

    function receiveResult(bytes32 jobID, bytes calldata result) external virtual onlyJobManager() {
        _receiveResult(jobID, result);
    }

    // This function must be overridden by the app-specific Consumer contract
    // to decode the coprocessor result into any app-specific struct and
    // perform app-specific logic using the result
    function _receiveResult(bytes32 jobID, bytes memory result) internal virtual;
}
