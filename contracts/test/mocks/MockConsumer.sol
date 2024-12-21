// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../../src/coprocessor/JobManager.sol";
import {Consumer} from "../../src/coprocessor/Consumer.sol";
import {SingleOffchainSigner} from "../../src/coprocessor/SingleOffchainSigner.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";

contract MockConsumer is Consumer, SingleOffchainSigner {
    mapping(address => uint256) public addressToBalance;
    mapping(bytes32 => bytes) public jobIDToResult;

    constructor(address jobManager, address _offchainSigner) Consumer(jobManager) SingleOffchainSigner(_offchainSigner) {}

    function initialize(address initialOwner, uint64 initialMaxNonce) public override initializer {
        Consumer.initialize(initialOwner, initialMaxNonce);
    }

    function getBalance(address addr) public view returns (uint256) {
        return addressToBalance[addr];
    }

    function getJobResult(bytes32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
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
}
