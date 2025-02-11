// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";

abstract contract OffchainRequester is Initializable, OwnableUpgradeable {
    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal EIP1271_MAGIC_VALUE = 0x1626ba7e;

    bytes4 constant internal INVALID_SIGNATURE = 0xffffffff;

    // Storage gap for upgradeability.
    uint256[50] private __GAP;

    // EIP-1271
    function isValidSignature(bytes32 hash, bytes memory signature) public virtual view returns (bytes4);
}
