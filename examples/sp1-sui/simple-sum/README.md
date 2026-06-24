# SP1 Sui Simple Sum

This is the smallest SP1-to-Sui path in this repository. SP1 is a zkVM/proving
stack, not a blockchain: the Rust guest program runs inside SP1, the host
script feeds inputs and asks SP1 to execute or prove, and `export-sui-verifier`
turns the SP1 Groth16 wrapper proof into a Sui Move verifier package.

The guest proves:

```text
given private stdin values a: u32 and b: u32,
sum = a + b without u32 overflow,
public values are committed as a, b, sum in that order
```

Only checks done inside `program/src/main.rs` are proven. Host-side checks are
sanity checks for this example.

## Prerequisites

- `cargo prove` and the SP1 toolchain.
- Docker for local Groth16 wrapping, or Succinct Prover Network credentials.
- `protoc`. If it is not installed system-wide, use this example's vendored
  helper with `PROTOC="$(cargo run -q -p protoc-path)"`.
- Enough memory for local Groth16. On WSL, local SP1 Groth16 wrapping can push
  memory to the limit even for this tiny guest. In my run, the Dockerized gnark
  step reached 15,972,262 constraints. A WSL config with `memory=30GB`,
  `swap=16GB`, and `processors=8` completed locally; `processors=20` pushed
  memory close to OOM. Use the prover network if local proving still crashes.

If your environment has a `socks5://...` `all_proxy`, unset it for the proving
command. The SP1 artifact downloader used here did not accept that proxy scheme.

## Commands

Run commands from this directory:

```sh
cd examples/sp1-sui/simple-sum
```

If you do not have `protoc` in `PATH`:

```sh
export PROTOC="$(cargo run -q -p protoc-path)"
```

Run the small Rust test for the shared sum logic:

```sh
cargo test -p simple-sum-lib
```

Compile the SP1 guest to a RISC-V ELF:

```sh
cargo prove build \
  -p simple-sum-program \
  --output-directory artifacts \
  --elf-name simple_sum.elf
```

Execute before proving. This is the cheap correctness check:

```sh
cargo run --release -p simple-sum-script -- --execute --a 17 --b 25
```

Generate a Groth16 proof locally:

```sh
env -u SP1_PROVER all_proxy= ALL_PROXY= \
  cargo run --release -p simple-sum-script -- --prove --a 17 --b 25
```

If WSL or Docker runs out of memory, use the prover network instead:

```sh
SP1_PROVER=network \
NETWORK_PRIVATE_KEY=<your_private_key> \
cargo run --release -p simple-sum-script -- --prove --a 17 --b 25
```

Successful proving writes:

- `artifacts/simple_sum_proof.bin`: serialized Groth16
  `SP1ProofWithPublicValues`.
- `artifacts/sp1_groth16_vk.bin`: SP1 Groth16 wrapper verifying key.
- `artifacts/simple_sum_public_values.bin`: raw committed public values
  `a`, `b`, `sum` as SP1-encoded bytes.
- `artifacts/simple_sum_program_vkey.txt`: program verification key hash to pin
  in downstream logic.

Generate the Sui Move verifier package:

```sh
cd ../../..

cargo run -- \
  --vk examples/sp1-sui/simple-sum/artifacts/sp1_groth16_vk.bin \
  --proof examples/sp1-sui/simple-sum/artifacts/simple_sum_proof.bin \
  --out examples/generated/sp1_sui_simple_sum \
  --force \
  --run-sui-test
```

For SP1 6.x Groth16 on Sui, the wrapper public inputs are the SP1 program vkey
hash, committed-public-values digest, exit code, vk root, and proof nonce. If
your final Sui contract needs to expose or enforce `a`, `b`, and `sum`, pass the
raw public values too and bind them to the digest expected by your verifier
logic.
