// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract JobManager is IJobManager, OwnableUpgradeable, ReentrancyGuard {
    uint32 public jobIDCounter = 1;
    address relayer;
    address coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;

    constructor(address _relayer, address _coprocessorOperator) {
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
        _disableInitializers();
    }

    function initializeJobManager(address initialOwner) public initializer {
        _transferOwnership(initialOwner);
    }

    function setRelayer(address _relayer) external onlyOwner {
        relayer = _relayer;
    }

    function setCoprocessorOperator(address _coprocessorOperator) external onlyOwner {
        coprocessorOperator = _coprocessorOperator;
    }

    function createJob(bytes calldata programID, bytes calldata programInput) external override returns (uint32 jobID) {
        jobID = jobIDCounter++;
        jobIDToMetadata[jobID] = JobMetadata(programID, msg.sender, JOB_STATE_PENDING);
        emit JobCreated(jobID, programID, programInput);
    }

    function getJobMetadata(uint32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function cancelJob(uint32 jobID) external override {
        JobMetadata memory job = jobIDToMetadata[jobID];
        require(msg.sender == job.caller, "JobManager.cancelJob: caller is not the job creator");

        job.status = JOB_STATE_CANCELLED;
        jobIDToMetadata[jobID] = job;

        emit JobCancelled(jobID);
    }

    // This function is called by the relayer
    function submitResult(
        bytes calldata resultWithMetadata, // Includes result value + job ID + program ID + hash of program input
        bytes calldata signature
    ) external override nonReentrant {
        require(msg.sender == relayer, "JobManager.submitResult: caller is not the relayer");

        // Recover the signer address
        bytes32 messageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n", resultWithMetadata.length, resultWithMetadata));
        address signer = recoverSigner(messageHash, signature);
        require(signer == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        // Decode the resultWithMetadata using abi.decode
        (bytes memory result, uint32 jobID, bytes memory programID, bytes32 programInputHash) = abi.decode(resultWithMetadata, (bytes, uint32, bytes, bytes32));

        JobMetadata memory job = jobIDToMetadata[jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

        // This is to prevent coprocessor from using a different program ID to produce a malicious result
        require(keccak256(job.programID) == keccak256(programID), 
            "JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");

        // This prevents the coprocessor from using arbitrary inputs to produce a malicious result
        require(keccak256(Consumer(job.caller).getProgramInputsForJob(jobID)) == programInputHash, 
            "JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");

        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;

        emit JobCompleted(jobID, result);

        Consumer(job.caller).receiveResult(jobID, result);
    }

    function recoverSigner(bytes32 _ethSignedMessageHash, bytes memory _signature) internal pure returns (address) {
        (bytes32 r, bytes32 s, uint8 v) = splitSignature(_signature);
        return ecrecover(_ethSignedMessageHash, v, r, s);
    }

    function splitSignature(bytes memory sig) internal pure returns (bytes32 r, bytes32 s, uint8 v) {
        require(sig.length == 65, "invalid signature length");

        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }

        return (r, s, v);
    }
}
