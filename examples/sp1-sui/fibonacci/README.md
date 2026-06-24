# SP1 Sui Fibonacci Example

This directory contains the Fibonacci SP1 example source adapted from
`SoundnessLabs/sp1-sui/examples/fibonacci` at revision
`15d84fd54f8127c4a5c5fac6fad75eb888d46fa2`.

SP1 is a zkVM/proving stack, not a blockchain. The guest program computes the
Fibonacci pair for private stdin `n` and commits Solidity ABI-encoded public
values:

```text
PublicValuesStruct { n: uint32, a: uint32, b: uint32 }
```

The original copied upstream fixture is still available:

- `artifacts/fibonacci_proof.bin`: serialized Groth16
  `SP1ProofWithPublicValues` from `proofs/fibonacci_proof.bin`.
- `artifacts/groth16_vk_v5.bin`: SP1 Groth16 wrapper verifying key from
  `verifier/vk/v5.0.0/groth16_vk.bin`.

Fresh local SP1 6.x artifacts are written to separate filenames so the v5
fixture is not overwritten.

## Commands

Run commands from this directory:

```sh
cd examples/sp1-sui/fibonacci
```

If `protoc` is not installed system-wide:

```sh
export PROTOC="$(cargo run -q -p protoc-path)"
```

Run the shared Rust logic test:

```sh
cargo test -p fibonacci-lib
```

Execute the SP1 guest without proving:

```sh
cargo run --release -p fibonacci-script -- --execute --n 20
```

Generate a local SP1 Groth16 proof:

```sh
env -u SP1_PROVER all_proxy= ALL_PROXY= \
  cargo run --release -p fibonacci-script -- --prove --n 20
```

Successful proving writes:

- `artifacts/fibonacci_sp1_6.elf`
- `artifacts/fibonacci_sp1_6_proof.bin`
- `artifacts/fibonacci_sp1_6_public_values.bin`
- `artifacts/fibonacci_sp1_6_program_vkey.txt`
- `artifacts/sp1_groth16_vk.bin`

Generate and test the Sui Move verifier package from the new SP1 6.x proof:

```sh
cd ../../..

cargo run -- \
  --vk examples/sp1-sui/fibonacci/artifacts/sp1_groth16_vk.bin \
  --proof examples/sp1-sui/fibonacci/artifacts/fibonacci_sp1_6_proof.bin \
  --out examples/generated/sp1_sui_fibonacci_sp1_6 \
  --force \
  --run-sui-test
```
