# Offchain jobs

Offchain job requests are are triggered by sending a request directly to a coprocessor node's [`SubmitJob` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodesubmitjob). The result will be submitted onchain and can also be queried directly from the coprocessor node via the [`GetResult` endpoint](../coprocessor/api.md#coprocessor_nodev1coprocessornodegetresult).

Offchain job requests can either be user initiated or service initiated. In the latter, there is a class of applications that run as real time servers with core state transition function (STFs) packaged up in InfinityVM programs. The servers will process user requests real time and have a background task that regularly batches STF inputs and submits them to the coprocessor node as a Job request. The results and are then submitted onchain and immediately usable by all other applications. The CLOB example illustrate

![offchain job request](../assets/offchain-job-request.png)
