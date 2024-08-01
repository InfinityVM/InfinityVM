// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

abstract contract OffchainRequester {
    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal VALID = 0x1626ba7e;

    bytes4 constant internal INVALID = 0xffffffff;

    function isValidSignature(bytes32 hash, bytes memory signature) public virtual view returns (bytes4);
}