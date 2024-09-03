// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../../src/coprocessor/JobManager.sol";
import {Consumer} from "../../src/coprocessor/Consumer.sol";
import {OffchainRequester} from "../../src/coprocessor/OffchainRequester.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract MockConsumer is Consumer, OffchainRequester {
    address private offchainSigner;
    mapping(address => uint256) public addressToBalance;
    mapping(bytes32 => bytes) public jobIDToResult;

    constructor(address jobManager, address _offchainSigner, uint64 initialMaxNonce) Consumer(jobManager, initialMaxNonce) OffchainRequester() {
        // MockConsumer allows a single offchainSigner address to sign all offchain job requests
        offchainSigner = _offchainSigner;
    }

    function getBalance(address addr) public view returns (uint256) {
        return addressToBalance[addr];
    }

    function getJobResult(bytes32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function getOffchainSigner() external view returns (address) {
        return offchainSigner;
    }

    // It doesn't really make sense for the contract to accept programID
    // as a parameter here (this would usually be hard-coded), but we do
    // it here so we can pass in arbitrary program IDs while testing and
    // in the CLI.
    function requestBalance(bytes calldata programID, address addr) public returns (bytes32) {
        return requestJob(programID, abi.encode(addr), 1_000_000);
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (address addr, uint256 balance) = abi.decode(result, (address, uint256));

        // Perform app-specific logic using the result
        addressToBalance[addr] = balance;
        jobIDToResult[jobID] = result;
    }

    // EIP-1271
    function isValidSignature(bytes32 messageHash, bytes memory signature) public view override returns (bytes4) {
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        if (recoveredSigner == offchainSigner) {
            return EIP1271_MAGIC_VALUE;
        } else {
            return INVALID_SIGNATURE;
        }
    }

    function _updateOffchainSigner(address updatedSigner) internal {
        offchainSigner = updatedSigner;
    }
}
