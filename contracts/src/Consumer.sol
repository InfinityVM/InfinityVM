// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    uint32 public maxNonce;
    mapping(bytes32 => bytes) internal jobIDToProgramInput;

    constructor(address __jobManager, uint32 _initialMaxNonce) {
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

    function getProgramInputsForJob(bytes32 jobID) public view virtual returns (bytes memory) {
        return jobIDToProgramInput[jobID];
    }

    function getMaxNonce() public view virtual returns (uint32) {
        return maxNonce;
    }

    function setProgramInputsForJob(bytes32 jobID, bytes memory programInput) public virtual onlyJobManager() {
        jobIDToProgramInput[jobID] = programInput;
    }

    function setMaxNonce(uint32 nonce) public virtual onlyJobManager() {
        maxNonce = nonce;
    }

    function requestJob(
        bytes memory programID,
        bytes memory programInput,
        uint64 maxCycles
    ) internal virtual returns (bytes32) {
        bytes32 jobID = _jobManager.createJob(getMaxNonce(), programID, programInput, maxCycles);
        setProgramInputsForJob(jobID, programInput);
        return jobID;
    }

    function cancelJob(bytes32 jobID) internal virtual {
        _jobManager.cancelJob(jobID);
    }

    function receiveResult(bytes32 jobID, bytes calldata result) external onlyJobManager() {
        _receiveResult(jobID, result);
    }

    // This function must be overridden by the app-specific Consumer contract
    // to decode the coprocessor result into any app-specific struct and
    // perform app-specific logic using the result
    function _receiveResult(bytes32 jobID, bytes memory result) internal virtual;
}
