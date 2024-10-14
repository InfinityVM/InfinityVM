# zkVM programs

This folder contains sample programs for running in the zkVM executor.

We currently only have a risc0 program. (Previously we had an equivalent sp1 program, but that was removed since it's not currently in use.)

We intend to [bring back support of sp1][1] and investigate adding support for [jolt][2].

Open questions:

- How should programs handle errors?

[1]: https://github.com/InfinityVM/InfinityVM/issues/120
[2]: https://github.com/a16z/jolt