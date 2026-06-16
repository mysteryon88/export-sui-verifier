# export-sui-verifier-core

Library crate for loading Groth16 artifacts and rendering Sui Move verifier packages.

## Capabilities

- loads snarkjs-compatible JSON inputs
- loads Arkworks VK/proof JSON or raw hex inputs
- loads compact Arkworks bundle JSON
- infers the curve and input format from artifact metadata
- supports BN254 and BLS12-381
- validates protocol, curve, subgroup membership, input counts, and field bounds
- serializes verification keys, proofs, and public inputs for `sui::groth16`
- performs local Arkworks Groth16 verification when proof vectors are supplied
- renders Sui Move packages with `Move.toml`, `sources/verifier.move`, optional proof/public-input tests, and generated package README

## Generated Move API

Generated modules expose:

- `verifying_key_bytes()`
- `prepare()`
- `verify(proof_bytes, public_inputs_bytes): bool`
- `verify_with_prepared(prepared_verifying_key, proof_bytes, public_inputs_bytes): bool`
- `verify_entry(proof_bytes, public_inputs_bytes)` when generated in `entry` or `test` mode

The verifier expects Arkworks canonical compressed proof bytes plus concatenated 32-byte little-endian public inputs.

## Main Modules

- `formats`: high-level loaders for snarkjs JSON and Arkworks inputs
- `parser::arkworks`: direct Arkworks VK/proof/public input parser
- `snarkjs`: strict snarkjs-compatible JSON parsing
- `model`: normalized Groth16 IR
- `curves`: curve-specific adapters for BN254 and BLS12-381
- `movegen`: Sui Move package rendering and proof-data snippets
- `verifier`: local Arkworks verification helpers

## Rust Usage

Use the crate directly when embedding generation in another Rust tool. Most users should use the `export-sui-verifier` CLI.

```rust
use export_sui_verifier_core::curves::create_adapter;
use export_sui_verifier_core::formats::load_arkworks_bundle;
use export_sui_verifier_core::movegen::{
    generate_move_package, GenerateMovePackageOptions, MovegenMode,
};

# fn main() -> export_sui_verifier_core::Result<()> {
let inputs = load_arkworks_bundle("groth16_artifacts.json".as_ref(), None)?;
let adapter = create_adapter(inputs.curve.canonical_name())?;

generate_move_package(
    "generated".as_ref(),
    adapter.as_ref(),
    &inputs,
    &GenerateMovePackageOptions {
        package_name: "generated",
        module_name: "verifier",
        mode: MovegenMode::Entry,
        force: true,
    },
)?;
# Ok(())
# }
```

## Crate Docs

- docs.rs: `https://docs.rs/export-sui-verifier-core`
- Rust import path: `export_sui_verifier_core`
