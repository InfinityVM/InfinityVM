# Offchain Example: CLOB

- intro
- where the code lives and where the zkvm program and contract is
- why is clob a good use of app server
- high-level user flow
- state
- sending batches to infinity coprocessor
- zkvm program
- future improvements



## Overview

Fully onchain CLOBs are inefficient for two reasons:
1. It is expensive to run all order book matching logic onchain and store all order book state onchain.
2. Every time a user places or cancels an order, this needs to be a tx. Market makers place and cancel a lot of orders for every fill, and so needing to pay a fee for every action makes market making infeasible. This is free on CEXs.

To solve these problems, the InfinityVM CLOB runs a "CLOB server" offchain. This server accepts orders/cancellations from users directly. It then creates a batch of new orders/cancellations and sends this to the InfinityVM coprocessor along with the state of user balances and the existing order book. The InfinityVM coprocessor runs the CLOB matching logic, and posts the updated user balances onchain.

We solve (1) because the matching logic is now run offchain in the InfinityVM coprocessor, and only the user balances are stored onchain (instead of storing the entire order book onchain as well). This is done without sacrificing any trust assumptions on the correctness of the results (i.e. given a batch of orders, the balance updates returned from the matching logic are correct since the matching is run in a zkVM).

We solve (2) because all orders/cancellations are sent to the CLOB server and not to the chain, and so we can make these actions free. Also, we gain additional efficiencies because if a market maker creates 100 orders and cancels 95 of them in the same batch, only the 5 non-cancelled orders need to be included in the batch sent by the CLOB server to the InfinityVM coprocessor.

## PoC Components

- [clob-node](./node): the CLOB service. 
- [clob-client](./client): client for seeding accounts, depositing, placing orders, withdrawing, and viewing state.

## Spec

### High-level user flow

The PoC only allows users to exchange two tokens: a `baseToken` and a `quoteToken`.

The user flow looks like this:
1. A user deposits `baseToken` and/or `quoteToken` into the CLOB contract by sending a tx onchain.
3. The user can place an order by sending an order directly to the CLOB server. The CLOB processes these orders in real time. 
4. The CLOB server has a background process that regularly batches orders. It sends a batch along with the state of user balances + order book to the InfinityVM coprocessor.
5. The InfinityVM coprocessor runs the CLOB matching logic in a zkVM (without generating a proof) and posts the updates to user balances to the CLOB contract.
6. The CLOB contract updates the user balances stored on the contract.
7. If a user wants to withdraw their funds, they can send a withdraw action directly to the CLOB server and the CLOB will include this withdrawal in the next batch. The CLOB contract will then transfer the funds back to the user.

**Note:** For the PoC, we decided to make users first deposit funds into the CLOB contract, instead of the contract just using the user's funds stored in the `baseToken` and `quoteToken`'s ERC20 contracts. We did this because if the CLOB uses the ERC20 contracts directly, users could maliciously make CLOB batches invalid if they transfer their tokens to some other address in the ERC20 contract right before the CLOB batch is posted (since the CLOB contract will try to transfer the user's funds if the user's order was filled but the user doesn't have these funds anymore). This issue can be fixed; we just didn't think it was necessary for a PoC.

### State

The only state stored onchain on the CLOB contract are the user balances for each token. This includes these mappings:
```solidity=
mapping(address => uint256) public depositedBalanceBase;
mapping(address => uint256) public depositedBalanceQuote;
mapping(address => uint256) public freeBalanceBase;
mapping(address => uint256) public freeBalanceQuote;
mapping(address => uint256) public lockedBalanceBase;
mapping(address => uint256) public lockedBalanceQuote;
```
This is what each balance is used for:
1. `depositedBalance`: Stores the balance of funds that a user has deposited, but are not available for use yet. The CLOB server listens for deposit events and will mark these funds as available to use when it posts the next batch.
2. `freeBalance`: Stores the balance of funds that a user can use to place orders or withdraw from the CLOB.
3. `lockedBalance`: Stores the balance of funds that a user has locked in existing orders on the order book. Once an order is cancelled or filled, the balance moves back from `locked` to `free`.

### User actions

The user can perform these actions:

1. `Deposit`: User sends a tx onchain to transfer some amount of `baseToken` and/or `quoteToken` to the CLOB contract. 
    - This increases the `depositedBalance`.
    - When the CLOB server picks up this deposit event and includes it in the next batch, the contract moves these funds from `depositedBalance` to `freeBalance`.
2. `Create`: User places an order by sending this directly to the CLOB server. 
    - If the order is filled in the same batch, the contract updates the user's balances accordingly. 
    - If the order is not filled in the batch, the contract decreases the user's `freeBalance` and increases their `lockedBalance`.
3. `Cancel`: User cancels an order by sending this directly to the CLOB server.
    - If this order was just added to the order book in the current batch, this doesn't result in any state update on the contract since the CLOB server just nets out the order.
    - If this order was added to the order book in a previous batch, this causes the contract to decrease the user's `lockedBalance` and increase `freeBalance`.
4. `Withdraw`: User withdraws funds from the CLOB contract by sending this directly to the CLOB server. A user can only withdraw from their `freeBalance`.
    - The contract decreases the user's `freeBalance` and transfers the funds back to the user.

### Sending batches to the InfinityVM coprocessor

The InfinityVM coprocessor allows apps to submit jobs directly to the coprocessor. Apps can choose which inputs they want to post onchain vs. to some DA layer. Some apps are also "stateful" and can pass in state. For example, the CLOB state contains all user balances in the CLOB along with the order book.

The API for submitting a job to the coprocessor is:
```rust=
struct JobRequest {
    nonce: u64,
    consumer_address: [u8; 20],
    program_id: Vec<u8>,
    onchain_input: Vec<u8>,
    offchain_input: Vec<u8>,
    offchain_input_hash: Vec<u8>,
    state: Vec<u8>,
    state_hash: Vec<u8>,
}
```

`offchain_input` contains:
- the new batch of orders/cancels/deposits/withdraws
- user signature for each order in the batch

The `offchain_input` and `state` will be borsh-encoded before submitting to the coprocessor.

### zkVM program

The zkVM program takes in `state` and `offchain_input` as inputs. It does these things:
- Decodes `state` and `offchain_input`
- Verifies that the signature on every order in the batch is valid
- Runs the CLOB matching function, which takes in the batch and the existing order book as inputs. We won't explain this function in detail here, but the code for this is in `zkvm_stf` in `clob/core/src/lib.rs` in the `InfinityVM` monorepo.
- Returns an ABI-encoded output, which includes the hash of the new CLOB state and a list of state updates which will be processed by the CLOB contract.

The list of state updates sent to the CLOB contract is structured like this:
```solidity=
struct ClobResultDeltas {
    DepositDelta[] depositDeltas;
    OrderDelta[] orderDeltas;
    WithdrawDelta[] withdrawDeltas;
}
```
where each type of update is defined as follows:
```solidity=
struct DepositDelta {
    address user;
    uint256 baseDelta;
    uint256 quoteDelta;
}

struct OrderDelta {
    address user;
    int256 freeBaseDelta;
    int256 lockedBaseDelta;
    int256 freeQuoteDelta;
    int256 lockedQuoteDelta;
}

struct WithdrawDelta {
    address user;
    uint256 baseDelta;
    uint256 quoteDelta;
}
```

The CLOB contract receives this list of state updates and processes it to update user balances. `DepositDelta` is used to update a user's `depositedBalance` and `freeBalance` due to deposits. `OrderDelta` is used to update a user's `freeBalance` and/or `lockedBalance` due to orders being placed, filled, and/or cancelled. `WithdrawDelta` is used to update a user's `freeBalance` and transfer funds to the user due to withdrawals.

### Ensuring correctness of the state passed as input

The coprocessor verifies that the `state` hashes to the same value as `state_hash` signed by the CLOB server in the job request. But, this is still not fully secure since the CLOB server can provide any arbitrary value for `state` and just include the corresponding hash for this arbitrary state in `state_hash`.

To solve this, we add logic in the CLOB contract to keep track of the state hash for each batch:
```solidity=
contract ClobConsumer {
    bytes32 latestStateHash;
    
    function receiveResult(bytes32 jobID, bytes result) {
        bytes32 stateHash = getStateHashForJob(jobID);
        
        // Check that state_hash passed by CLOB 
        // server matches the most recent state hash
        require(stateHash == latestStateHash);
        
        // Update the most recent state hash with the new state
        latestStateHash = abi.decode(result).nextStateHash;
    }
}
```
By verifying that `state_hash` matches the state from the most recent batch, this logic ensures that the CLOB server can only submit valid state to the InfinityVM coprocessor.

### Future improvements

This CLOB is a PoC and so there are many components that would be interesting to research further. A few things:

1. The CLOB server could potentially give commitments to users when users place orders. If this is combined with some kind of slashing mechanism, this could be used to guarantee price-time priority for all users of the CLOB.
2. The current system of requiring users to deposit funds into the CLOB contract sacrifices some composability since these funds are "locked" in the CLOB contract until the user withdraws. It would be valuable to look into how to improve composability.
