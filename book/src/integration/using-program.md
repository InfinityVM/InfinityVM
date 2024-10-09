# Using a zkVM Program

To run your zkVM program in the InfinityVM coprocessor, you can request this in two ways: onchain and offchain.

## Onchain requests

This allows you to make calls to the coprocessor from a smart contract. At a high level, this is how onchain requests work:

1. You make a call to the coprocessor from your app contract, and pass in your zkVM program ID and inputs. This emits an event.
2. The InfinityVM coprocessor executes your zkVM program with the inputs.
3. You write a callback function in your app contract which accepts the coprocessor's output and uses it in any application logic.

![onchain job request](../assets/onchain-job-request.png)

You can read more in the [Onchain Jobs](./onchain.md) section.

## Offchain requests

This allows you to send requests directly to the InfinityVM coprocessor. This can be done using gRPC or the coprocessor node's REST API. The result will still be submitted onchain and can be used by a callback function in your app contract, similar to onchain requests.

![offchain job request](../assets/offchain-job-request.png)

Offchain job requests can be sent by a user, your app, or any authorized third-party. In the case that your app sends the requests, there is an interesting class of applications that run as real-time servers. 

You can read more in the [Offchain Jobs](./offchain.md) section.

