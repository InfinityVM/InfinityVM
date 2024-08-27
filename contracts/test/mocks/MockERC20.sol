// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockERC20 is ERC20 {
    constructor(string memory name, string memory symbol) ERC20(name, symbol) {
    }

    // We have this function to make it easier to setup tests
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}
