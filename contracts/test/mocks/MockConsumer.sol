// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../../src/JobManager.sol";
import {Consumer} from "../../src/Consumer.sol";

contract MockConsumer is Consumer {

    mapping(address => uint256) public addressToBalance;
    mapping(uint32 => bytes) public jobIDToResult;

    constructor(address jobManager) Consumer(jobManager) {}

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

}
