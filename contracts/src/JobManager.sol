// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OffchainRequester} from "./OffchainRequester.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import {Script, console} from "forge-std/Script.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract JobManager is 
    IJobManager,
    Initializable,
    OwnableUpgradeable, 
    ReentrancyGuard
{
    using Strings for uint;

    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal EIP1271_MAGIC_VALUE = 0x1626ba7e;

    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    mapping(bytes32 => JobMetadata) public jobIDToMetadata;
    // storage gap for upgradeability
    uint256[50] private __GAP;

    constructor() {
        _disableInitializers();
    }

    function initializeJobManager(address initialOwner, address _relayer, address _coprocessorOperator) public initializer {
        _transferOwnership(initialOwner);
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
    }

    function getRelayer() external view returns (address) {
        return relayer;
    }


    function getCoprocessorOperator() external view returns (address) {
        return coprocessorOperator;
    }

    function getJobMetadata(bytes32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function setRelayer(address _relayer) external onlyOwner {
        relayer = _relayer;
    }

    function setCoprocessorOperator(address _coprocessorOperator) external onlyOwner {
        coprocessorOperator = _coprocessorOperator;
    }

    function createJob(uint32 nonce, bytes calldata programID, bytes calldata programInput, uint64 maxCycles) external override returns (bytes32) {
        address consumer = msg.sender;
        bytes32 jobID = keccak256(abi.encodePacked(nonce, consumer));
       _createJob(nonce, jobID, programID, maxCycles, consumer);
        emit JobCreated(jobID, maxCycles, programID, programInput);
        return jobID;
    }

    function _createJob(uint32 nonce, bytes32 jobID, bytes memory programID, uint64 maxCycles, address consumer) internal {
        require(jobIDToMetadata[jobID].status == 0, "JobManager.createJob: job already exists with this nonce and consumer");
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, consumer, JOB_STATE_PENDING);
        uint32 maxNonce = Consumer(consumer).getMaxNonce();
        if (nonce > maxNonce) {
            Consumer(consumer).setMaxNonce(nonce);
        }
    }

    function cancelJob(bytes32 jobID) external override {
        JobMetadata memory job = jobIDToMetadata[jobID];
        // We allow the JobManager owner to also cancel jobs so Ethos admin can veto any jobs
        require(msg.sender == job.caller || msg.sender == owner(), "JobManager.cancelJob: caller is not the job creator or JobManager owner");

        require(job.status == JOB_STATE_PENDING, "JobManager.cancelJob: job is not in pending state");
        job.status = JOB_STATE_CANCELLED;
        jobIDToMetadata[jobID] = job;

        emit JobCancelled(jobID);
    }

    function submitResult(
        bytes memory resultWithMetadata, // Includes job ID + program input hash + max cycles + program ID + result value
        bytes memory signature
    ) public override nonReentrant {
        require(msg.sender == relayer, "JobManager.submitResult: caller is not the relayer");

        // Verify signature on result with metadata
        bytes32 messageHash = ECDSA.toEthSignedMessageHash(resultWithMetadata);
        require(ECDSA.tryRecover(messageHash, signature) == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        // Decode the resultWithMetadata using abi.decode
        ResultWithMetadata memory result = decodeResultWithMetadata(resultWithMetadata);
        _submitResult(result.jobID, result.maxCycles, result.programInputHash, result.programID, result.result);
    }

    function submitResultForOffchainJob(
        bytes calldata offchainResultWithMetadata,
        bytes calldata signatureOnResult,
        bytes calldata jobRequest,
        bytes calldata signatureOnRequest
    ) public override {
        // Decode the job request using abi.decode
        OffchainJobRequest memory request = decodeJobRequest(jobRequest);
        bytes32 jobID = keccak256(abi.encodePacked(request.nonce, request.consumer));

        // Verify signature on job request
        bytes32 requestHash = ECDSA.toEthSignedMessageHash(jobRequest);
        require(OffchainRequester(request.consumer).isValidSignature(requestHash, signatureOnRequest) == EIP1271_MAGIC_VALUE, "JobManager.submitResultForOffchainJob: Invalid signature on job request");

        // Verify signature on result with metadata
        bytes32 resultHash = ECDSA.toEthSignedMessageHash(offchainResultWithMetadata);
        require(ECDSA.tryRecover(resultHash, signatureOnResult) == coprocessorOperator, "JobManager.submitResultForOffchainJob: Invalid signature on result");

        // Create a job without emitting an event and set program inputs on consumer
        _createJob(request.nonce, jobID, request.programID, request.maxCycles, request.consumer);
        Consumer(request.consumer).setProgramInputsForJob(jobID, request.programInput);

        // Decode the result using abi.decode
        OffChainResultWithMetadata memory result = decodeOffchainResultWithMetadata(offchainResultWithMetadata);
        _submitResult(jobID, result.maxCycles, result.programInputHash, result.programID, result.result);
    }

    function _submitResult(
        bytes32 jobID,
        uint64 maxCycles,
        bytes32 programInputHash,
        bytes memory programID,
        bytes memory result
    ) internal {
        JobMetadata memory job = jobIDToMetadata[jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

        // This prevents the coprocessor from using arbitrary inputs to produce a malicious result
        require(keccak256(Consumer(job.caller).getProgramInputsForJob(jobID)) == programInputHash, 
            "JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");
        
        // This is to prevent coprocessor from using a different program ID to produce a malicious result
        require(keccak256(job.programID) == keccak256(programID),
            "JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");
        
        require(job.maxCycles == maxCycles, "JobManager.submitResult: max cycles signed by coprocessor doesn't match max cycles submitted with job");

        // Update job status to COMPLETED
        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;
        emit JobCompleted(jobID, result);

        // Forward result to consumer
        Consumer(job.caller).receiveResult(jobID, result);
    }

    function decodeResultWithMetadata(bytes memory resultWithMetadata) public pure returns (ResultWithMetadata memory) {
        (bytes32 jobID, bytes32 programInputHash, uint64 maxCycles, bytes memory programID, bytes memory result) = abi.decode(resultWithMetadata, (bytes32, bytes32, uint64, bytes, bytes));
        return ResultWithMetadata(jobID, programInputHash, maxCycles, programID, result);
    }

    function decodeOffchainResultWithMetadata(bytes memory offChainResultWithMetadata) public pure returns (OffChainResultWithMetadata memory) {
        (bytes32 programInputHash, uint64 maxCycles, bytes memory programID, bytes memory result) = abi.decode(offChainResultWithMetadata, (bytes32, uint64, bytes, bytes));
        return OffChainResultWithMetadata(programInputHash, maxCycles, programID, result);
    }

    function decodeJobRequest(bytes memory jobRequest) public pure returns (OffchainJobRequest memory) {
        (uint32 nonce, uint64 maxCycles, address consumer, bytes memory programID, bytes memory programInput) = abi.decode(jobRequest, (uint32, uint64, address, bytes, bytes));
        return OffchainJobRequest(nonce, maxCycles, consumer, programID, programInput);
    }
}
