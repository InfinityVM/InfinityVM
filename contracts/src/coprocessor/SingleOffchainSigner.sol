// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {OffchainRequester} from "./OffchainRequester.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import {console} from "forge-std/console.sol";

// SingleOffchainSigner allows a single offchainSigner address to sign all offchain job requests
abstract contract SingleOffchainSigner is OffchainRequester {
    address private offchainSigner;
    // Storage gap for upgradeability.
    uint256[49] private __GAP;

    function initialize(address _offchainSigner) public virtual onlyInitializing {
        offchainSigner = _offchainSigner;
    }

    function getOffchainSigner() external view returns (address) {
        return offchainSigner;
    }

    function _updateOffchainSigner(address updatedSigner) internal {
        offchainSigner = updatedSigner;
    }

    // Included for EIP-1271. The JobManager calls this function to verify the signature on
    // an offchain job request.
    function isValidSignature(bytes32 messageHash, bytes memory signature) public view override returns (bytes4) {
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        // Checks that the job request was signed by the offchainSigner
        if (recoveredSigner == offchainSigner) {
            return EIP1271_MAGIC_VALUE;
        } else {
            return INVALID_SIGNATURE;
        }
    }
}
