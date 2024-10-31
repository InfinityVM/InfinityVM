# Coprocessor Node

The Coprocessor node is the actual engine to run offchain compute. Applications send requests to these nodes to:

1. Submit their `Program`
1. Execute `Jobs` that call their `Program` on a given set of inputs and commit the results onchain.
1. Verfiy `Job` execution results.
1. Generate zk fraud proofs for a given `Job`.

Details and running the coprocessor node, it's API, and it's internal architecture follow.
