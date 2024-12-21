// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../coprocessor/JobManager.sol";
import {Consumer} from "../coprocessor/Consumer.sol";
import {StatefulConsumer} from "../coprocessor/StatefulConsumer.sol";
import {OffchainRequester} from "../coprocessor/OffchainRequester.sol";
import {SingleOffchainSigner} from "../coprocessor/SingleOffchainSigner.sol";
import {console} from "forge-std/Script.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";

contract MatchingGameConsumer is StatefulConsumer, SingleOffchainSigner {
    struct Match {
        address user1;
        address user2;
    }

    // Mapping to store user --> partner
    mapping(address => address) public userToPartner;

    constructor(address jobManager, address offchainSigner) StatefulConsumer(jobManager) SingleOffchainSigner(offchainSigner) {}

    function initialize(address initialOwner, uint64 initialMaxNonce, bytes32 initialStateRoot) public override initializer {
        StatefulConsumer.initialize(initialOwner, initialMaxNonce, initialStateRoot);
    }

    // Getter function for matched users
    function getPartner(address user) external view returns (address) {
        return userToPartner[user];
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override  {
        Match[] memory matches = abi.decode(result, (Match[]));

        // Store the matches in the contract state
        for (uint256 i = 0; i < matches.length; i++) {
            userToPartner[matches[i].user1] = matches[i].user2;
            userToPartner[matches[i].user2] = matches[i].user1;
        }
    }
}
