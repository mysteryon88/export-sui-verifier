# Export Sui Verifier

[![dependency status](https://deps.rs/repo/github/mysteryon88/export-sui-verifier/status.svg)](https://deps.rs/repo/github/mysteryon88/export-sui-verifier)

**Export Sui Verifier** is a CLI tool and Rust library for generating **Groth16** Sui Move verifier packages from snarkjs JSON, Arkworks JSON/hex inputs, native Gnark JSON/binary artifacts, SP1 Groth16 wrapper proofs, or compact Arkworks bundle files.

It supports **BN254** and **BLS12-381**. Circuits built with **Circom** and **Noname** are supported through `snarkjs`-compatible JSON; **Gnark** is supported both through `snarkjs` compatibility and native Gnark artifacts; **SP1** is supported through its BN254 Groth16 wrapper proof format; **Arkworks** is supported through direct JSON/hex inputs or compact bundles. The curve and input format are inferred from artifact metadata and structure when exactly one supported curve validates the artifacts.

When proof data is supplied, the tool validates the artifacts, runs local Arkworks Groth16 verification, and emits Move tests with the generated package. VK-only generation is also supported.

Generated packages use `sui::groth16` and contain `Move.toml`, `sources/verifier.move`, and optional `tests/verifier_tests.move`. Generation uses root-level CLI flags; `proof-data` is the only subcommand.

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
export-sui-verifier --vk ./verification_key.json --out ./generated/my_verifier --force

# Include proof vectors for local verification and generated Move tests:
export-sui-verifier --vk ./verification_key.json --proof ./proof.json --public ./public.json --out ./generated/my_verifier --force

# If proof.json contains publicSignals, --public can be omitted:
export-sui-verifier --vk ./verification_key.json --proof ./proof.json --out ./generated/my_verifier --force

# From Arkworks JSON/hex inputs:
export-sui-verifier --vk ./arkworks_verification_key.json --proof ./arkworks_proof.json --public ./public_inputs.json --out ./generated/arkworks_verifier --force

# From a compact Arkworks bundle:
export-sui-verifier --bundle ./groth16_artifacts.json --out ./generated/ark_mimc_bn254 --force

# From native Gnark JSON:
export-sui-verifier --vk ./verification_key_gnark.json --proof ./proof_gnark.json --public ./public.json --out ./generated/gnark_json_verifier --force

# From native Gnark binary vk.WriteTo/proof.WriteTo artifacts:
export-sui-verifier --vk ./verification_key.bin --proof ./proof.bin --public ./public.json --out ./generated/gnark_bin_verifier --force

# From an SP1 Groth16 wrapper VK and serialized SP1ProofWithPublicValues:
export-sui-verifier --vk ./groth16_vk.bin --proof ./sp1_proof.bin --out ./generated/sp1_verifier --force

# Customize the generated Move package:
export-sui-verifier --vk ./verification_key.json --out ./generated/my_verifier --package-name my_verifier --module-name verifier --mode entry --force

# Generate proof helper functions for tests:
export-sui-verifier proof-data --vk ./verification_key.json --proof ./proof.json

# Generate and run sui move test:
export-sui-verifier --vk ./verification_key.json --proof ./proof.json --out ./generated/my_verifier --run-sui-test --force
```

`--package-name` is derived from `--out` by default, `--module-name` defaults to `verifier`, and `--mode` defaults to `entry`. `--mode` accepts `library`, `entry`, or `test`. Use `--skip-local-verify` only when you want to bypass local Arkworks proof verification.

## Supported Artifact Shapes

- **snarkjs JSON**: `verification_key.json`, `proof.json`, and optional `public.json`/`publicSignals`, as emitted by snarkjs-compatible tooling.
- **Arkworks JSON/hex**: direct Arkworks-style verifying key, proof, and public input objects, raw Arkworks compressed hex files, or one compact `groth16_artifacts.json` bundle. Raw hex and bundles without `curve` metadata are auto-detected when exactly one supported curve parses and verifies them; malformed compressed hex with trailing bytes is rejected.
- **native Gnark JSON**: `json.Marshal` output of Gnark Groth16 structs. The VK contains `G1.Alpha`, `G1.Beta`, `G1.Delta`, `G1.K`, `G2.Beta`, `G2.Gamma`, `G2.Delta`; the proof contains `Ar`, `Bs`, and `Krs`. `public.json` is an ordered array of decimal public field elements.
- **native Gnark binary**: Gnark `VerifyingKey.WriteTo` and `Proof.WriteTo` byte streams. The loader supports the current BN254 and BLS12-381 Groth16 layout and normalizes the bytes for `sui::groth16`.
- **SP1 Groth16**: SP1 wrapper `groth16_vk.bin` plus serialized `SP1ProofWithPublicValues`. The loader extracts the embedded Gnark BN254 proof and normalized public inputs. Older SP1 wrapper artifacts commonly expose `vkey_hash` and committed-values digest; SP1 6.x Groth16 artifacts expose five wrapper public inputs: vkey hash, committed-values digest, exit code, vk root, and proof nonce.

## References

- [Sui Groth16 documentation](https://docs.sui.io/develop/cryptography/groth16)
- [Sui `groth16` Move module](https://docs.sui.io/references/framework/sui_sui/groth16)
- Examples
  - [examples](./examples/)
- Export of proof and verification key in JSON format compatible with snarkjs
  - [gnark-to-snarkjs](https://github.com/mysteryon88/gnark-to-snarkjs)
  - [ark-snarkjs](https://github.com/mysteryon88/ark-snarkjs)
- SP1 Sui verifier reference
  - [SoundnessLabs/sp1-sui](https://github.com/SoundnessLabs/sp1-sui)
- Frameworks verified for compatibility
  - [Circom](https://docs.circom.io/)
  - [Noname](https://github.com/zksecurity/noname)
  - [Gnark](https://github.com/Consensys/gnark)
  - [SP1](https://github.com/succinctlabs/sp1)
  - [Arkworks](https://github.com/arkworks-rs)
