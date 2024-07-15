// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.12;

contract EmptyContract {
    address private _owner;

    constructor() {
        _owner = msg.sender;
    }

    function owner() public view returns (address) {
        return _owner;
    }

}
