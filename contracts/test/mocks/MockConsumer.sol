// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;
import {JobManager} from "../../src/JobManager.sol";
import {Consumer} from "../../src/Consumer.sol";

contract MockConsumer is Consumer {

    mapping(address => uint256) public addressToBalance;
    mapping(uint32 => bytes) public jobIDToResult;

    constructor(address jobManager) Consumer(jobManager) {}

    function requestBalance(address addr) public returns (uint32) {
        return requestJob("programID", abi.encode(addr), 1000000);
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
