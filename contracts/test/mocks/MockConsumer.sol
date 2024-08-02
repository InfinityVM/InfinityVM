// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../../src/JobManager.sol";
import {Consumer} from "../../src/Consumer.sol";
import {OffchainRequester} from "../../src/OffchainRequester.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract MockConsumer is Consumer, OffchainRequester {
    address private offchainSigner;
    mapping(address => uint256) public addressToBalance;
    mapping(uint32 => bytes) public jobIDToResult;

    constructor(address jobManager, address _offchainSigner) Consumer(jobManager) OffchainRequester() {
        offchainSigner = _offchainSigner;
    }

    // It doesn't really make sense for the contract to accept programID
    // as a parameter here (this would usually be hard-coded), but we do
    // it here so we can pass in arbitrary program IDs while testing and
    // in the CLI.
    function requestBalance(bytes calldata programID, address addr) public returns (uint32) {
        return requestJob(programID, abi.encode(addr), 1_000_000);
    }

    function getBalance(address addr) public view returns (uint256) {
        return addressToBalance[addr];
    }

    function getJobResult(uint32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function _receiveResult(uint32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (address addr, uint256 balance) = abi.decode(result, (address, uint256));

        // Perform app-specific logic using the result
        addressToBalance[addr] = balance;
        jobIDToResult[jobID] = result;
    }

    function isValidSignature(bytes32 messageHash, bytes memory signature) public view override returns (bytes4) {
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        if (recoveredSigner == offchainSigner) {
            return VALID;
        } else {
            return INVALID;
        }
    }

    function _updateSigner(address updatedSigner) internal {
        offchainSigner = updatedSigner;
    }

    function getSigner() external view returns (address) {
        return offchainSigner;
    }
}
