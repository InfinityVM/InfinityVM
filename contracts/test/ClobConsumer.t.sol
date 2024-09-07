// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {Consumer} from "../src/coprocessor/Consumer.sol";
import {StatefulConsumer} from "../src/coprocessor/StatefulConsumer.sol";
import {ClobConsumer} from "../src/clob/ClobConsumer.sol";
import {ClobDeployer} from "../script/ClobDeployer.s.sol";

contract ClobConsumerTest is Test, ClobDeployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;
    address OFFCHAIN_REQUEST_SIGNER = 0xaF6Bcd673C742723391086C1e91f0B29141D2381;
    address alice = address(2);
    address bob = address(3);
    uint64 DEFAULT_NONCE = 1;
    bytes32 DEFAULT_JOB_ID;

    event Deposit(address indexed user, uint256 baseAmount, uint256 quoteAmount);

    function setUp() public {
        uint64 initialMaxNonce = 0;
        deployClobContracts(RELAYER, COPROCESSOR_OPERATOR, OFFCHAIN_REQUEST_SIGNER, initialMaxNonce);
        DEFAULT_JOB_ID = keccak256(abi.encodePacked(DEFAULT_NONCE, address(consumer)));

        baseToken.mint(alice, 1000);
        quoteToken.mint(bob, 1000);

        // Approve the Clob contract to transfer tokens on behalf of alice and bob
        vm.prank(alice);
        baseToken.approve(address(consumer), 1000 ether);
        vm.prank(bob);
        quoteToken.approve(address(consumer), 1000 ether);
    }

    function test_ClobConsumer_DepositInitiated() public {
        vm.prank(alice);
        vm.expectEmit(true, true, true, false);
        emit Deposit(alice, 200, 0);
        consumer.deposit(200, 0);
        vm.prank(bob);
        vm.expectEmit(true, true, true, false);
        emit Deposit(bob, 0, 800);
        consumer.deposit(0, 800);

        assertEq(consumer.getDepositedBalanceBase(alice), 200);
        assertEq(consumer.getDepositedBalanceQuote(bob), 800);
        assertEq(baseToken.balanceOf(alice), 800);
        assertEq(quoteToken.balanceOf(bob), 200);
        assertEq(consumer.getFreeBalanceBase(alice), 0);
        assertEq(consumer.getFreeBalanceQuote(bob), 0);
    }

    function test_ClobConsumer_DepositConfirmed() public {
        test_ClobConsumer_DepositInitiated();

        vm.prank(address(jobManager));
        consumer.setProgramInputsForJob(DEFAULT_JOB_ID, abi.encode(0x0, ""));
    
        ClobConsumer.DepositDelta[] memory depositDeltas = new ClobConsumer.DepositDelta[](2);
        depositDeltas[0] = ClobConsumer.DepositDelta({
            user: alice,
            baseDelta: 200,
            quoteDelta: 0
        });
        depositDeltas[1] = ClobConsumer.DepositDelta({
            user: bob,
            baseDelta: 0,
            quoteDelta: 800
        });

        ClobConsumer.OrderDelta[] memory orderDeltas = new ClobConsumer.OrderDelta[](0);
        ClobConsumer.WithdrawDelta[] memory withdrawDeltas = new ClobConsumer.WithdrawDelta[](0);

        ClobConsumer.ClobResultDeltas memory deltas = ClobConsumer.ClobResultDeltas({
            depositDeltas: depositDeltas,
            orderDeltas: orderDeltas,
            withdrawDeltas: withdrawDeltas
        });

        StatefulConsumer.StatefulProgramResult memory clobResult = StatefulConsumer.StatefulProgramResult({
            nextStateRootHash: 0x0,
            result: abi.encode(deltas)
        });

        bytes memory encodedResult = abi.encode(clobResult);

        vm.prank(address(jobManager));
        consumer.receiveResult(DEFAULT_JOB_ID, encodedResult);

        assertEq(consumer.getDepositedBalanceBase(alice), 0);
        assertEq(consumer.getDepositedBalanceQuote(bob), 0);
        assertEq(consumer.getFreeBalanceBase(alice), 200);
        assertEq(consumer.getFreeBalanceQuote(bob), 800);
    }

    function test_ClobConsumer_FillOrder() public {
        test_ClobConsumer_DepositConfirmed();

        ClobConsumer.DepositDelta[] memory depositDeltas = new ClobConsumer.DepositDelta[](0);

        ClobConsumer.OrderDelta[] memory orderDeltas = new ClobConsumer.OrderDelta[](2);
        orderDeltas[0] = ClobConsumer.OrderDelta({
            user: alice,
            freeBaseDelta: -200,
            lockedBaseDelta: 0,
            freeQuoteDelta: 300,
            lockedQuoteDelta: 0
        });
        orderDeltas[1] = ClobConsumer.OrderDelta({
            user: bob,
            freeBaseDelta: 200,
            lockedBaseDelta: 0,
            freeQuoteDelta: -700,
            lockedQuoteDelta: 400 // Bob placed an order for 700 but only 300 got filled
        });

        ClobConsumer.WithdrawDelta[] memory withdrawDeltas = new ClobConsumer.WithdrawDelta[](0);

        ClobConsumer.ClobResultDeltas memory deltas = ClobConsumer.ClobResultDeltas({
            depositDeltas: depositDeltas,
            orderDeltas: orderDeltas,
            withdrawDeltas: withdrawDeltas
        });

        StatefulConsumer.StatefulProgramResult memory clobResult = StatefulConsumer.StatefulProgramResult({
            nextStateRootHash: 0x0,
            result: abi.encode(deltas)
        });

        bytes memory encodedResult = abi.encode(clobResult);
        vm.prank(address(jobManager));
        consumer.receiveResult(DEFAULT_JOB_ID, encodedResult);

        assertEq(consumer.getFreeBalanceBase(alice), 0);
        assertEq(consumer.getFreeBalanceQuote(alice), 300);
        assertEq(consumer.getFreeBalanceBase(bob), 200);
        assertEq(consumer.getFreeBalanceQuote(bob), 100);
        assertEq(consumer.getLockedBalanceBase(alice), 0);
        assertEq(consumer.getLockedBalanceQuote(bob), 400);
    }

    function test_ClobConsumer_Withdraw() public {
        test_ClobConsumer_FillOrder();

        ClobConsumer.DepositDelta[] memory depositDeltas = new ClobConsumer.DepositDelta[](0);
        ClobConsumer.OrderDelta[] memory orderDeltas = new ClobConsumer.OrderDelta[](0);
        ClobConsumer.WithdrawDelta[] memory withdrawDeltas = new ClobConsumer.WithdrawDelta[](2);
        withdrawDeltas[0] = ClobConsumer.WithdrawDelta({
            user: alice,
            baseDelta: 0,
            quoteDelta: 300
        });
        withdrawDeltas[1] = ClobConsumer.WithdrawDelta({
            user: bob,
            baseDelta: 200,
            quoteDelta: 100
        });

        ClobConsumer.ClobResultDeltas memory deltas = ClobConsumer.ClobResultDeltas({
            depositDeltas: depositDeltas,
            orderDeltas: orderDeltas,
            withdrawDeltas: withdrawDeltas
        });

        StatefulConsumer.StatefulProgramResult memory clobResult = StatefulConsumer.StatefulProgramResult({
            nextStateRootHash: 0x0,
            result: abi.encode(deltas)
        });

        bytes memory encodedResult = abi.encode(clobResult);
        vm.prank(address(jobManager));
        consumer.receiveResult(DEFAULT_JOB_ID, encodedResult);

        assertEq(consumer.getFreeBalanceBase(alice), 0);
        assertEq(consumer.getFreeBalanceQuote(alice), 0);
        assertEq(consumer.getFreeBalanceBase(bob), 0);
        assertEq(consumer.getFreeBalanceQuote(bob), 0);

        assertEq(baseToken.balanceOf(alice), 800); // Alice only deposited 200 earlier
        assertEq(baseToken.balanceOf(bob), 200);
        assertEq(quoteToken.balanceOf(alice), 300);
        assertEq(quoteToken.balanceOf(bob), 200 + 100); // Bob only deposited 800 earlier
    }
}
