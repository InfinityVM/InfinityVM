# Data Availability

Infinity implements EIP-4844 for data availability. Most importantly, blobs are finalized in lockstep with beacon blocks, meaning that infinity DA has sub second, single slot finality. The speed and lockstep nature of Infinity DA processing is critical for native coprocessing (more details below). 

DA is gossiped around the network in short lived blobs. Validators guarantee that the blobs will be available for a certain period (normally two weeks and always longer then the fraud proof window). There is a special [blob transaction type](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-4844.md#blob-transaction) for the execution layer that contains references to blobs. By having blobs expire and thus not contribute to canonical state bloat, we can make them very cheap.

## Details 

**Block production:** when a validator is creating a block, they send [engine_forkChoiceUpdatedV3](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_forkchoiceupdatedv3) and then retrieve the payload from the execution layer using [engine_getPayloadV3](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#engine_getpayloadv3). The [response](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#response-2) includes a [blobs bundle](https://github.com/ethereum/execution-apis/blob/main/src/engine/cancun.md#BlobsBundleV1). The validator then gossips the beacon block along with the referenced sidecars.[^note1]

**Block validation:** validators approving beacon blocks will ensure that they have all blobs referenced by the block stored in a local availability store. Validators prune their availability store regularly to remove expired blobs.

![](./../assets/blob-propagation.png)
*Flow of blob propagation between the execution client and consensus client. In the future blobs will be gossiped separately.*

### Coprocessing

The target use case for DA is [offchain coprocessing jobs](../integration/offchain.md); offchain input is stored in blobs that can be used for fraud proofs and general consumption. 

When submitting the results for an offchain job, the InfinityVM coprocessor will convert the offchain input into blobs and reference the blob hashes in the job result metadata. In effect, the coprocessor is committing to not only the job inputs and outputs, but also the exact DA blobs[^note2]. The job manager contract processes the job result metadata and ensures that the blob hashes referenced in the metadata are correctly associated with the block by using the [BLOBHASH op code](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-4844.md#opcode-to-get-versioned-hashes) - guaranteeing that committed to blobs are held by all consensus layer validators. This process ensures that the correct blob data for any job is available in the consensus layer to process fraud proofs.

[^note1]: Currently both the block and sidecars are in a unified bundle, but over time we will transition to data availability sampling (DAS) to achieve higher blob throughput. The current sidecar design is meant to support a [transition to DAS](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-4844.md#consensus-layer-validation).

[^note2]: Astute readers may be asking why signing the transaction is not enough to commit to blob hashes. For flexibility and future compatibility with a decentralized coprocessing network, the transaction relayer is different from the InfinityVM coprocessor. Thus, we need the coprocessor to explicitly commit to the blobs and their ordering. This commitment takes the form of the abi encoded `OffchainResultWithMetadata`.

