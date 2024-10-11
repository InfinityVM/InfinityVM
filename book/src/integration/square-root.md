# Onchain Example: Square Root

In this section, we walk through a simple example of a square root app from the [InfinityVM foundry template](https://github.com/InfinityVM/infinityVM-foundry-template). There is no native operation in Solidity to calculate a square root, so we can just write this in Rust to compute square roots in InfinityVM.

The zkVM program used by this app is [`square_root.rs`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/programs/app/src/square_root.rs). The contract for the square root app is [`SquareRootConsumer.sol`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/contracts/src/SquareRootConsumer.sol), and stores a `numberToSquareRoot` mapping from each number to its square root.

## Writing the zkVM Program

The zkVM program does three things:

1. Read and decode the input data (an ABI-encoded integer)
1. Calculate the square root
1. Commit the result (encoded using ABI for easy decoding in the app contract)

```rust,ignore
fn main() {
    // Read the input data for this application.
    let mut input_bytes = Vec::<u8>::new();
    env::stdin().read_to_end(&mut input_bytes).unwrap();

    // Decode and parse the input
    let number = <U256>::abi_decode(&input_bytes, true).unwrap();

    // Calculate square root
    let square_root = number.root(2);

    // Commit the result that will be received by the application contract.
    // Result is encoded using Solidity ABI for easy decoding in the app contract.
    env::commit_slice(NumberWithSquareRoot::abi_encode(&(number, square_root)).as_slice());
}
```

Running `cargo build` in the foundry template builds the program and generates a unique program ID, which is added to the `ProgramID.sol` contract in the template repo.

## Making an onchain job request 

`SquareRootConsumer.sol` has a [`requestSquareRoot()`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/2d10113f1e01ac314c7b9fb96b1a40d640d53a4b/contracts/src/SquareRootConsumer.sol#L35) function:
```rust,ignore
function requestSquareRoot(uint256 number) public returns (bytes32) {
    return requestJob(ProgramID.SQUARE_ROOT_ID, abi.encode(number), DEFAULT_MAX_CYCLES);
}
```

This function calls `requestJob()` to make an onchain job request to the InfinityVM coprocessor. In this request, it passes in the program ID `SQUARE_ROOT_ID` of the zkVM program and the input `number` (after ABI encoding) that we want to calculate the square root of. 

It also passes in `DEFAULT_MAX_CYCLES` (max cycles is the max number of execution cycles that we want the zkVM to run for while computing a job).

## Receiving the result onchain

This is the [`_receiveResult()`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/2d10113f1e01ac314c7b9fb96b1a40d640d53a4b/contracts/src/SquareRootConsumer.sol#L39) function in `SquareRootConsumer.sol`:
```rust,ignore
function _receiveResult(bytes32 jobID, bytes memory result) internal override {
    // Decode the coprocessor result into AddressWithBalance
    (uint256 originalNumber, uint256 squareRoot) = abi.decode(result, (uint256, uint256));

    // Perform app-specific logic using the result
    numberToSquareRoot[originalNumber] = squareRoot;
    jobIDToResult[jobID] = result;
}
```

This is a callback function called when the InfinityVM coprocessor submits the result of a job back to the square root app contract. This function decodes the result and then stores the square root value.

## Testing the end-to-end flow

To test the end-to-end flow of requesting an onchain job in the square root app, we have written [`test_Consumer_RequestJob()`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/2d10113f1e01ac314c7b9fb96b1a40d640d53a4b/contracts/test/SquareRootConsumer.t.sol#L26) in [`SquareRootConsumer.t.sol`](https://github.com/InfinityVM/infinityVM-foundry-template/blob/main/contracts/test/SquareRootConsumer.t.sol#L26). This test requests the square root of a number and verifies that the contract makes a call to InfinityVM and that the coprocessor submits the correct result back to the contract.
