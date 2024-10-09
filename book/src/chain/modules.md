# Key modules

Infinity follows a service-based architecture, with code separated into different modules and a service that can be run for each module.

A list of key modules is provided below:

- `execution`: The execution engine, which is essentially a shim around the execution client (i.e. Reth). It communicates with the execution client using the Engine API.
- `consensus`: Single-slot finality protocol and handlers for events like proposing and finalizing block proposals, etc.
- `beacon`: Logic and state related to the beacon chain.
- `payload`: Code to request building payloads on the execution client and retrieving execution payloads.
- `state-transition`: Code to run the [beacon chain state transition function](https://eth2book.info/capella/part3/transition/). This includes processing slots and blocks, which involves verifying validity conditions and processing payloads, staking deposits and withdrawals, etc.
