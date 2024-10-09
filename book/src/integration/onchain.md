# Onchain Jobs

For building an app which uses onchain jobs, we recommend using the [Infinity foundry template](https://github.com/InfinityVM/infinity-foundry-template/tree/main). We have instructions in the `README` to get started (most of the content in this page is in the `README` too).

## Writing your app contract

Any app contract building with InfinityVM needs to inherit the [`Consumer`](https://github.com/InfinityVM/infinity-foundry-template/blob/main/contracts/src/coprocessor/Consumer.sol) interface. To build an app, you don't need to read how `Consumer` or any of the other contracts in [`contracts/src/coprocessor`](https://github.com/InfinityVM/infinity-foundry-template/tree/main/contracts/src/coprocessor) are implemented; you can just focus on your app.

Next, your app contract needs to do two things:

1. Call `requestJob()` with the program ID of your zkVM program along with ABI-encoded inputs that you want to pass into your program from the contract. We have instructions on how to generate the program ID in the Infinity foundry template's `README`.
2. Write a `_receiveResult()` callback function which accepts the output from the InfinityVM coprocessor running your program. You can write any app logic in this function and even call into any other functions you'd like.

#### Initiating onchain job requests

Typically, we expect an onchain request to be triggered by user interaction with your app contract. But in some designs, contract callback handling logic for a previous job result may trigger a new request event, effectively creating a continuous loop of requests without user interaction.

## Testing your app contract

In the foundry template, you can write tests for the end-to-end flow of your app similar to any other foundry tests. We have built an SDK within the foundry template which allows you to request and receive compute from InfinityVM within the foundry tests. 

One example of this is [`test_Consumer_RequestJob`](https://github.com/InfinityVM/infinity-foundry-template/blob/2d10113f1e01ac314c7b9fb96b1a40d640d53a4b/contracts/test/SquareRootConsumer.t.sol#L26). You can run the test using `forge test -vvv --ffi`.
