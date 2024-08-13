// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

abstract contract OffchainRequester {
    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal EIP1271_MAGIC_VALUE = 0x1626ba7e;

    bytes4 constant internal INVALID_SIGNATURE = 0xffffffff;

    // EIP-1271
    function isValidSignature(bytes32 hash, bytes memory signature) public virtual view returns (bytes4);
}