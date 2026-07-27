# Export Sui Verifier

[![dependency status](https://deps.rs/repo/github/mysteryon88/export-sui-verifier/status.svg)](https://deps.rs/repo/github/mysteryon88/export-sui-verifier)

**Export Sui Verifier** is a CLI tool and Rust library for generating **Groth16** Sui Move verifier packages.

It supports **BN254** and **BLS12-381** verification artifacts from **snarkjs**, **Gnark**, **SP1**, and **Arkworks**. Supported inputs include JSON, native Gnark binary files, SP1 Groth16 wrapper proofs, Arkworks JSON/hex files, and compact Arkworks bundles. The curve and input format are auto-detected.

When proof data is supplied, the tool verifies it locally and generates Move tests with the package. VK-only generation is also supported.

## Installation

```bash
cargo install export-sui-verifier

# Help
export-sui-verifier --help
```

## Import as a library

```bash
cargo add export-sui-verifier-core
```

```rust
use export_sui_verifier_core::{
    curves::create_adapter,
    formats::{
        load_arkworks_bundle, load_gnark_binary_inputs_auto, load_gnark_json_inputs,
        load_snarkjs_json_inputs_with_optional_proof, load_sp1_groth16_inputs,
    },
    movegen::{generate_move_package, GenerateMovePackageOptions, MovegenMode},
};
```

Most users only need the CLI. Use the core crate when embedding verifier generation into another Rust tool.

## Usage CLI

```sh
# From snarkjs-compatible verification_key.json:
export-sui-verifier --vk ./verification_key.json --out ./generated/verifier --force

# Include proof data for local verification and generated Move tests:
export-sui-verifier --vk ./verification_key.json --proof ./proof.json --public ./public.json --out ./generated/verifier --force

# From native Gnark JSON or binary artifacts:
export-sui-verifier --vk ./verification_key_gnark.json --proof ./proof_gnark.json --public ./public.json --out ./generated/gnark_verifier --force
export-sui-verifier --vk ./verification_key.bin --proof ./proof.bin --public ./public.json --out ./generated/gnark_verifier --force

# From an SP1 Groth16 wrapper proof:
export-sui-verifier --vk ./groth16_vk.bin --proof ./sp1_proof.bin --out ./generated/sp1_verifier --force

# From a compact Arkworks bundle:
export-sui-verifier --bundle ./groth16_artifacts.json --out ./generated/arkworks_verifier --force

# Customize the generated Move package:
export-sui-verifier --vk ./verification_key.json --out ./generated/verifier --package-name verifier --module-name verifier --mode entry --force

# Generate proof helper functions or run Sui Move tests:
export-sui-verifier proof-data --vk ./verification_key.json --proof ./proof.json
export-sui-verifier --vk ./verification_key.json --proof ./proof.json --out ./generated/verifier --run-sui-test --force
```

`--package-name` is derived from `--out` by default. `--module-name` defaults to `verifier`, and `--mode` defaults to `entry`. Available modes are `library`, `entry`, and `test`.

## License

MIT.

## References

- [Sui Groth16 documentation](https://docs.sui.io/develop/cryptography/groth16)
- [Examples](./examples/)
- [gnark-to-snarkjs](https://github.com/mysteryon88/gnark-to-snarkjs)
- [ark-snarkjs](https://github.com/mysteryon88/ark-snarkjs)
- [SP1 Sui verifier](https://github.com/SoundnessLabs/sp1-sui)
- [Circom](https://docs.circom.io/)
- [Noname](https://github.com/zksecurity/noname)
- [Gnark](https://github.com/Consensys/gnark)
- [SP1](https://github.com/succinctlabs/sp1)
- [Arkworks](https://github.com/arkworks-rs)
