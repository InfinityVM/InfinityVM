// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

abstract contract OffchainRequester {
    address private signer;

    constructor(address signer_) {
        signer = signer_;
    }

    function _updateSigner(address updatedSigner) internal {
        signer = updatedSigner;
    }

    function getSigner() external view returns (address) {
        return signer;
    }
}