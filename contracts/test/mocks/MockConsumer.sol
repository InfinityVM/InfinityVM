// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.13;
import {JobManager} from "../../src/JobManager.sol";
import {Consumer} from "../../src/Consumer.sol";

contract MockConsumer is Consumer {

    struct AddressWithBalance {
        address addr;
        uint256 balance;
    }

    mapping(address => uint256) public addressToBalance;
    mapping(uint32 => bytes) public jobIDToResult;

    constructor(address jobManager) Consumer(jobManager) {}

    function getBalance(address addr) public view returns (uint256) {
        return addressToBalance[addr];
    }

    function getJobResult(uint32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function receiveResult(uint32 jobID, bytes calldata result) public override onlyJobManager {
        // Decode the coprocessor result into AddressWithBalance
        (AddressWithBalance memory resultStruct) = abi.decode(result, (AddressWithBalance));

        // Perform app-specific logic using the result
        addressToBalance[resultStruct.addr] = resultStruct.balance;
        jobIDToResult[jobID] = result;
    }

}
