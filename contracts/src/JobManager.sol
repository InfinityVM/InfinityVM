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
import {EIP712} from "solady/utils/EIP712.sol"; 

contract JobManager is 
    IJobManager,
    Initializable,
    OwnableUpgradeable, 
    ReentrancyGuard
{
    using Strings for uint;

    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal VALID = 0x1626ba7e;

    uint32 internal jobIDCounter;
    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;
    mapping(bytes32 => uint32) public nonceHashToJobID;
    mapping(address => uint32) public consumerToMaxNonce;
    // storage gap for upgradeability
    uint256[50] private __GAP;

    constructor() {
        _disableInitializers();
    }

    function initializeJobManager(address initialOwner, address _relayer, address _coprocessorOperator) public initializer {
        _transferOwnership(initialOwner);
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
        jobIDCounter = 1;
    }

    function getRelayer() external view returns (address) {
        return relayer;
    }


    function getCoprocessorOperator() external view returns (address) {
        return coprocessorOperator;
    }

    function getJobMetadata(uint32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function getMaxNonce(address consumer) public view returns (uint32) {
        return consumerToMaxNonce[consumer];
    }

    function setRelayer(address _relayer) external onlyOwner {
        relayer = _relayer;
    }

    function setCoprocessorOperator(address _coprocessorOperator) external onlyOwner {
        coprocessorOperator = _coprocessorOperator;
    }

    function createJob(bytes calldata programID, bytes calldata programInput, uint64 maxCycles) external override returns (uint32) {
        uint32 jobID = jobIDCounter;
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, msg.sender, JOB_STATE_PENDING);
        emit JobCreated(jobID, maxCycles, programID, programInput);
        jobIDCounter++;
        return jobID;
    }

    function cancelJob(uint32 jobID) external override {
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

        // Recover and verify the signer address
        bytes32 messageHash = ECDSA.toEthSignedMessageHash(resultWithMetadata);
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        require(recoveredSigner == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        // Decode the resultWithMetadata using abi.decode
        ResultWithMetadata memory result = decodeResultWithMetadata(resultWithMetadata);

        JobMetadata memory job = jobIDToMetadata[result.jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

        // This is to prevent coprocessor from using a different program ID to produce a malicious result
        require(keccak256(job.programID) == keccak256(result.programID), 
            "JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");

        // This prevents the coprocessor from using arbitrary inputs to produce a malicious result
        require(keccak256(Consumer(job.caller).getProgramInputsForJob(result.jobID)) == result.programInputHash, 
            "JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");
        
        require(result.maxCycles == job.maxCycles, "JobManager.submitResult: max cycles signed by coprocessor doesn't match max cycles submitted with job");

        _submitResult(result.jobID, job.caller, result.result);
    }

    function _submitResult(
        uint32 jobID,
        address consumer,
        bytes memory result
    ) internal {
        JobMetadata memory job = jobIDToMetadata[jobID];
        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;
        emit JobCompleted(jobID, result);

        Consumer(consumer).receiveResult(jobID, result);
    }

    function submitOffchainResult(
        bytes calldata offchainResultWithMetadata,
        bytes calldata signatureOnResult,
        bytes calldata jobRequest,
        bytes calldata signatureOnRequest
    ) public override returns (uint32) {
        // Decode the job request using abi.decode
        OffchainJobRequest memory request = decodeJobRequest(jobRequest);

        // Check if nonce already exists
        bytes32 nonceHash = keccak256(abi.encodePacked(request.nonce, request.consumer));
        require(nonceHashToJobID[keccak256(abi.encodePacked(request.nonce, request.consumer))] == 0, "JobManager.submitOffChainResult: Nonce already exists for this consumer");

        require(OffchainRequester(request.consumer).isValidSignature(ECDSA.toEthSignedMessageHash(jobRequest), signatureOnRequest) == VALID, "JobManager.submitOffChainResult: Invalid signature on job request");

        // Perform the logic from createJob() without emitting an event
        uint32 jobID = jobIDCounter;
        jobIDToMetadata[jobID] = JobMetadata(request.programID, request.maxCycles, request.consumer, JOB_STATE_PENDING);
        jobIDCounter++;

        // Update nonce-relevant storage
        nonceHashToJobID[nonceHash] = jobID;
        if (request.nonce > consumerToMaxNonce[request.consumer]) {
            consumerToMaxNonce[request.consumer] = request.nonce;
        }

        // Recover and verify signer on the result
        require(ECDSA.tryRecover(ECDSA.toEthSignedMessageHash(offchainResultWithMetadata), signatureOnResult) == coprocessorOperator, "JobManager.submitOffchainResult: Invalid signature on result");

        // Decode the result using abi.decode
        OffChainResultWithMetadata memory result = decodeOffchainResultWithMetadata(offchainResultWithMetadata);

        require(result.programInputHash == keccak256(request.programInput), "JobManager.submitOffChainResult: program input hash doesn't match");
        require(keccak256(result.programID) == keccak256(request.programID), "JobManager.submitOffChainResult: program ID doesn't match");
        require(result.maxCycles == request.maxCycles, "JobManager.submitOffChainResult: max cycles doesn't match");

        _submitResult(jobID, request.consumer, result.result);

        return jobID;
    }

    function decodeResultWithMetadata(bytes memory resultWithMetadata) public pure returns (ResultWithMetadata memory) {
        (uint32 jobID, bytes32 programInputHash, uint64 maxCycles, bytes memory programID, bytes memory result) = abi.decode(resultWithMetadata, (uint32, bytes32, uint64, bytes, bytes));
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
