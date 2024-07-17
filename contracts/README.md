## Coprocessor contracts

### Overview

- `JobManager.sol`: Shared contract for all coprocessor consumers. Coprocessor job requests and result submission goes through this contract.
- `Consumer.sol`: Interface that can be inherited by coprocessor consumers
- `MockConsumer.sol`: Example of a coprocessor consumer contract which inherits `Consumer` (used in tests and CLI)

### Dependencies

You will need [foundry](https://book.getfoundry.sh/getting-started/installation) and [zap-pretty](https://github.com/maoueh/zap-pretty).
```
curl -L https://foundry.paradigm.xyz | bash
foundryup
go install github.com/maoueh/zap-pretty@latest
```

If you're facing errors when running any of the commands below, you might need to upgrade Foundry using `foundryup`.

### Run tests for contracts

```
make test-contracts
```

### Loom

Info on the anvil states and CLI commands is available in the [Loom here](https://www.loom.com/share/a4ddf4c5cccc407c8ae8c06eca4c72be?sid=2c2f3f03-e48f-4e09-984e-db9082d69ee9).

### Deploy tooling and Anvil states

To deploy the coprocessor contracts to anvil and save anvil state, you can run:
```
make deploy-coprocessor-contracts-to-anvil-and-save-state
```

To request a hard-coded job and save this in the anvil state, you can run:
```
make request-job-on-anvil-and-save-state
```

You can then use one of these commands to start Anvil from either of the saved states:
```
make start-anvil-chain-with-coprocessor-deployed
OR
make start-anvil-chain-with-coprocessor-deployed-and-job-requested
```

### CLI

All the CLI commands along with relevant comments and parameters are listed in the `Makefile`.

If you would like to use different values for any of the parameters, you can pass these in through the CLI or by changing the default values in the `Makefile`.

### Generate Go Bindings
```shell
$ ./generate_bindings.sh
```