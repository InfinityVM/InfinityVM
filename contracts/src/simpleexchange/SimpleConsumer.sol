// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../coprocessor/JobManager.sol";
import {Consumer} from "../coprocessor/Consumer.sol";
import {StatefulConsumer} from "../coprocessor/StatefulConsumer.sol";
import {OffchainRequester} from "../coprocessor/OffchainRequester.sol";
import {SingleOffchainSigner} from "../coprocessor/SingleOffchainSigner.sol";
import {console} from "forge-std/Script.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract SimpleConsumer is StatefulConsumer, SingleOffchainSigner {
    constructor(address jobManager, address offchainSigner, uint64 initialMaxNonce, IERC20 _baseToken, IERC20 _quoteToken, bytes32 latestStateHash) StatefulConsumer(jobManager, initialMaxNonce, latestStateHash) SingleOffchainSigner(offchainSigner) {}

    function _receiveResult(bytes32 jobID, bytes memory result) internal override  {
        require(false, "Receiving result");
    }
}
