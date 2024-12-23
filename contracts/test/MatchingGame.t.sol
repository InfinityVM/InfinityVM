// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {Consumer} from "../src/coprocessor/Consumer.sol";
import {StatefulConsumer} from "../src/coprocessor/StatefulConsumer.sol";
import {MatchingGameConsumer} from "../src/matching-game/MatchingGameConsumer.sol";
import {CoprocessorDeployer} from "../script/CoprocessorDeployer.s.sol";
import {MatchingGameDeployer} from "../script/MatchingGameDeployer.s.sol";

contract MatchingGameTest is Test, CoprocessorDeployer, MatchingGameDeployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;
    address OFFCHAIN_REQUEST_SIGNER = 0xaF6Bcd673C742723391086C1e91f0B29141D2381;
    address alice = address(2);
    address bob = address(3);
    uint64 DEFAULT_NONCE = 1;
    bytes32 DEFAULT_JOB_ID;

    function setUp() public {
        uint64 initialMaxNonce = 0;
        deployCoprocessorContracts(RELAYER, COPROCESSOR_OPERATOR, true);
        deployMatchingGameContracts(OFFCHAIN_REQUEST_SIGNER, initialMaxNonce, false);
        DEFAULT_JOB_ID = keccak256(abi.encodePacked(DEFAULT_NONCE, address(consumer)));
    }

    // function test_MatchingGameConsumer_ProcessMatches() public {
    //     // Explicitly define the size of the array
    //     MatchingGameConsumer.Match[] memory matches = new MatchingGameConsumer.Match[](1);
    //     matches[0] = MatchingGameConsumer.Match({user1: alice, user2: bob});

    //     StatefulConsumer.StatefulAppResult memory matchingGameResult = StatefulConsumer.StatefulAppResult({
    //         outputStateRoot: 0x0,
    //         result: abi.encode(matches)
    //     });

    //     bytes memory encodedResult = abi.encode(matchingGameResult);
        
    //     // Set the onchain input for the job here so we pass the StatefulConsumer state root check
    //     vm.prank(address(jobManager));
    //     consumer.setInputsForJob(DEFAULT_JOB_ID, abi.encode(0x0, ""), 0x0);

    //     vm.prank(address(jobManager));
    //     consumer.receiveResult(DEFAULT_JOB_ID, encodedResult);

    //     assertEq(consumer.getPartner(alice), bob);
    //     assertEq(consumer.getPartner(bob), alice);
    // }
}
