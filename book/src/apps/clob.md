# Offchain App (Advanced): CLOB

In this section, we walk through a more complex example of an offchain app server: a central-limit order book (CLOB).

## Why build a CLOB with InfinityVM

Fully onchain CLOBs are inefficient for two reasons:
1. It is expensive to run all order book matching logic onchain and store all order book state onchain.
1. Users have to pay for every order creation/cancellation, making it economically infeasible.

With InfinityVM, a CLOB is able to solve these problems by running an App Server. This server processes orders from users offchain in real-time, at zero cost to users. Meanwhile, by running the matching logic in the InfinityVM coprocessor, the CLOB is able to efficiently update the orderbook without sacrificing any verifiability.

## High-level user flow

The CLOB example only allows users to exchange two tokens: a `baseToken` and a `quoteToken`.

The user flow looks like this:

![clob app server](../assets/clob.png)

1. A user deposits `baseToken` and/or `quoteToken` into the CLOB contract.
1. The user can continuously place orders by sending them directly to the CLOB server. The CLOB processes these orders in real time, and returns a response to the user (includes info on whether a user's order has been fulfilled and by how much).
1. The CLOB server has a background process that regularly batches orders. It sends each batch along with the relevant state of user balances + order book to the InfinityVM coprocessor.
1. The InfinityVM coprocessor runs the CLOB matching logic in the zkVM program to match orders.
1. The coprocessor posts the result to the CLOB contract.
1. The CLOB contract uses the result to update the user balances stored on the contract.

## Code Overview

The CLOB app contract is [`ClobConsumer.sol`](https://github.com/InfinityVM/InfinityVM/blob/main/contracts/src/clob/ClobConsumer.sol).

All code for the CLOB server lives in [`clob/`](https://github.com/InfinityVM/InfinityVM/tree/main/clob) in the InfinityVM repo. Specifically:

- [node](https://github.com/InfinityVM/InfinityVM/tree/main/clob/node): the CLOB service.
  - This is similar to [server](https://github.com/InfinityVM/InfinityVM/tree/main/matching-game/server) from [Offchain App (Simple): Matching Game](./matching-game.md).
- [client](https://github.com/InfinityVM/InfinityVM/tree/main/clob/client): client for seeding accounts, depositing, placing orders, withdrawing, and viewing state.
- [core](https://github.com/InfinityVM/InfinityVM/tree/main/clob/core): Types and functions with the CLOB matching logic which is shared by the app server and zkVM program.

The zkVM program for the CLOB is [`clob.rs`](https://github.com/InfinityVM/InfinityVM/blob/main/clob/programs/app/src/clob.rs).

#### Note: Code organization

We defined a lot of the CLOB logic in a shared `core` crate so the code can be easily reused in both the app server and the zkVM program.

For example, the CLOB engine has a [`tick` function](https://github.com/InfinityVM/InfinityVM/blob/main/clob/core/src/lib.rs#L282) in the `core` crate which processes a single request. This is used in the [app server code](https://github.com/InfinityVM/InfinityVM/blob/main/clob/node/src/engine.rs) to process each request sent to the server. This same code is reused in the zkVM program's [state transition function](https://github.com/InfinityVM/InfinityVM/blob/main/clob/core/src/lib.rs#L275), to process each request in the batch given as input.

This also allows the `tick` function to be easily unit tested without the restrictions of the zkVM.

## Onchain state

The only state stored onchain in the CLOB app contract are the user balances for each token.

## User actions

A user can perform these actions:

1. `Deposit`: User sends a transaction onchain to transfer some amount of `baseToken` and/or `quoteToken` to the CLOB contract. 
1. `Create`: User places an order by sending this directly to the CLOB server. 
1. `Cancel`: User cancels an order by sending this directly to the CLOB server.
1. `Withdraw`: User withdraws funds from the CLOB contract by sending this directly to the CLOB server.

## Sending job requests to the InfinityVM coprocessor

The CLOB server batches user orders and sends offchain job requests to the InfinityVM coprocessor.

From [Offchain Jobs](./offchain.md), the API for submitting a job to the coprocessor is:

```rust,ignore
message SubmitJobRequest {
  bytes request = 1; // ABI-encoded offchain job request
  bytes signature = 2;
  bytes offchain_input = 3;
}

struct OffchainJobRequest {
    uint64 nonce;
    uint64 maxCycles;
    address consumer;
    bytes programID;
    bytes onchainInput;
    bytes32 offchainInputHash;
}
```

The CLOB uses `offchain_input` since we need to pass in a large number of orders as input in each batch. This `offchain_input` contains the new batch of orders/cancels/deposits/withdraws. The `offchain_input` is borsh-encoded by the CLOB server before submitting to the coprocessor.

The CLOB is a stateful app server, and the CLOB's state contains all user balances along with the order book. The CLOB stores its state as a merkle trie. To pass this state to the zkVM program, the CLOB submits the state root in `onchain_input` and submits a merkle proof against this state root for the relevant balances + order book values in `offchain_input` (Note: The merkle trie logic still needs to be implemented for the CLOB. Currently, we just pass in the entire CLOB state to the zkVM program.)

## zkVM program

The zkVM program takes in `onchain_input` and `offchain_input` as inputs. It does these things:

1. Decodes `onchain_input` and `offchain_input`.
2. Verifies the merkle proof against the state root provided in `onchain_input`.
1. Runs the CLOB app's state transition function, which matches orders given the inputs from the batch in `offchain_input` and the existing order book + balances in the CLOB's state. We won't explain this function in detail here, but the code for this is in [`zkvm_stf`](https://github.com/InfinityVM/InfinityVM/blob/main/clob/core/src/lib.rs#L275).
1. Returns an ABI-encoded output, which includes the new CLOB state root and a list of balance updates which will be processed by the CLOB app contract.

The list of state updates sent to the CLOB contract is structured like this:

```rust,ignore
struct ClobResultDeltas {
    DepositDelta[] depositDeltas;
    OrderDelta[] orderDeltas;
    WithdrawDelta[] withdrawDeltas;
}
```

The CLOB contract receives this list of state updates and processes it to update user balances.

## Ensuring correctness of the state root

In [Stateful App Servers](./offchain.md#stateful-app-servers), we discussed the problem of ensuring the correctness of the state root submitted by an app server to the coprocessor.

The `ClobConsumer` contract implements the [`StatefulConsumer`](https://github.com/InfinityVM/InfinityVM/blob/main/contracts/src/coprocessor/StatefulConsumer.sol) interface, to verify that the state root submitted by the CLOB server in the job request is correct.
