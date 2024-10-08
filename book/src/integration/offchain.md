# Offchain jobs

Offchain job requests are are triggered by sending a request directly to a coprocessor nodes' submit job endpoint. The result will be submitted onchain and can also be queried directly from the coprocessor node from the `GetResult` endpoint.

Offchain job requests can either be user initiated or service initiated. In the latter, there is a class of applications that run as real time servers with core state transition function (STFs) packaged up in infinityVM programs. The servers will process user requests real time and have a background task that regularly batches STF inputs and submits them to the coprocessor node as a Job request. The results and are then submitted onchain and immediately usable by all other applications.

![offchain job request](../assets/offchain-job-request.png)
