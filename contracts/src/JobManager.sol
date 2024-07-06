// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";

// TODO (Maanav): Add OwnableUpgradeable
contract JobManager is IJobManager {
    uint32 public jobIDCounter = 1;
    address relayer;
    address coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;

    constructor(address _relayer, address _coprocessorOperator) {
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
    }

    function setRelayer(address _relayer) external {
        relayer = _relayer;
    }

    function createJob(bytes calldata programID, bytes calldata inputs) external override returns (uint32 jobID) {
        jobID = jobIDCounter++;
        jobIDToMetadata[jobID] = JobMetadata(programID, msg.sender, JOB_STATE_PENDING);
        emit JobCreated(jobID, programID, inputs);
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

    // TODO: Add reentrancy check (probably easiest via OpenZeppelin's ReentrancyGuard)
    // This function is called by the relayer
    function submitResult(bytes calldata resultWithJobID, bytes calldata signature) external override {
        require(msg.sender == relayer, "JobManager.submitResult: caller is not the relayer");

        // Decode the resultWithJobID using abi.decode
        (uint32 jobID, bytes memory result) = abi.decode(resultWithJobID, (uint32, bytes));
        // Recover the signer address
        bytes32 messageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n", resultWithJobID.length, resultWithJobID));
        address signer = recoverSigner(messageHash, signature);
        require(signer == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        JobMetadata memory job = jobIDToMetadata[jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

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