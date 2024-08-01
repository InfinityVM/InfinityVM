// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../../src/JobManager.sol";
import {Consumer} from "../../src/Consumer.sol";
import {OffchainRequester} from "../../src/OffchainRequester.sol";

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

    function isValidSignature(bytes32 hash, bytes memory signature) public view override returns (bytes4) {
        address signer = recoverSigner(hash, signature);
        if (signer == offchainSigner) {
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
