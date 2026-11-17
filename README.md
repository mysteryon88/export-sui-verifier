# Export Sui Verifier

[![dependency status](https://deps.rs/repo/github/mysteryon88/export-sui-verifier/status.svg)](https://deps.rs/repo/github/mysteryon88/export-sui-verifier)

**Export Sui Verifier** is a CLI tool and Rust library for generating **Groth16** Sui Move verifier packages.

It supports **BN254** and **BLS12-381** verification artifacts from **snarkjs**, **Gnark**, **SP1**, and **Arkworks**. Supported inputs include JSON, native Gnark binary files, SP1 Groth16 wrapper proofs, Arkworks JSON/hex files, and compact Arkworks bundles. The curve and input format are auto-detected.

By default, when proof data is supplied, the CLI verifies it locally and generates Move tests with the package. `--skip-local-verify` disables that check. Core-library callers must invoke `local_verify` explicitly. VK-only generation is also supported.

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

## Security

Generated verifiers are stateless mathematical verifiers. `verify`, `verify_with_bound_prepared`, and `verify_entry` (when generated in entry or test mode) do not authorize callers or prevent replay; `verify_entry` only aborts when verification fails.

For a protected operation, bind a domain separator, package ID, Sui network or chain identifier, operation, canonical nullifier, and expected VK or circuit identity into the circuit's public inputs. Recompute the exact same context in Move from trusted application state, reject nullifiers already in canonical persistent replay state, and record a nullifier only after verification succeeds. See the [stateful gatekeeper example](./examples/stateful-gatekeeper/).

Both verification functions use the embedded fixed verification key through `BoundPreparedVerifyingKey`, whose inner key is private. Public inputs are limited to eight canonical scalars, but canonical encoding does not bind their application meaning or provide replay protection.

The manifest's `vk_sha256` and module's `vk_fingerprint()` identify the generated VK bytes and support integrity checks only when compared with an independently trusted expected value. They do not authenticate artifact provenance, independently identify the intended circuit, or prove that a trusted setup was honest. Treat the VK, proof, public inputs, circuit or program, and generated package as security-critical: authenticate and review them, inspect `verifier-manifest.json` and generated Move, keep local verification enabled, and run Move tests. VK-only generation and `--skip-local-verify` do not establish proof correctness, circuit intent, or artifact authenticity.

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


## Output directory safety

Generation rejects `..` before creating any parent and refuses filesystem roots, the current directory and its ancestors. Every existing path component must be a real directory, without symlinks or Windows reparse points. Windows output currently requires a local drive path; UNC/device paths are unsupported.

Output is written through retained directory handles into exclusive, private staging (Linux mode `0700`; Windows protected, inheritable owner/SYSTEM DACL). Files are created exclusively. Publication, force backup, rollback and cleanup use native operations relative to retained handles. Linux requires `renameat2(RENAME_NOREPLACE)`; Windows uses a held source handle and `FILE_RENAME_INFORMATION` with `ReplaceIfExists = FALSE`. Existing destinations, including raced empty directories, files and symlinks, are never replaced without force. An unsupported filesystem fails safely instead of falling back to a replacing rename. WSL DrvFS does not provide the required rename operation on the tested host; use a native Linux filesystem such as ext4 or run the Windows generator. Parsing remains available on other platforms, where generation returns `Unsupported`.

Force compares the target and actual backup against held cwd/ancestor directory identities, including alternate path spellings and aliases. It validates the actual directory moved to backup. A failed commit restores it only if the destination is still absent. If a concurrent destination prevents rollback, both it and the backup survive and the error reports the recovery location. Failed transactions retain staging for inspection; paths in errors are the last known locations and can become stale if an ancestor was renamed. Successful cleanup remains handle-relative and refuses unexpected symlinks or uncertain directory identities.

## Public scalar encodings

Arkworks and compact public inputs use the same grammar in JSON strings and raw scalar files: unprefixed digits are decimal at every length, `0x`/`0X` is a numeric big-endian hexadecimal integer, and `le:` followed by exactly 64 hex characters is one serialized 32-byte little-endian scalar. Bare 64-character hex containing `a-f` remains accepted for legacy byte artifacts; use `le:` for new serialized inputs so digit-only bytes cannot change meaning. Values must still fit the selected curve's scalar field.

The Arkworks MiMC producer writes the explicit `le:` marker in both `groth16_artifacts.json` and `public_input.hex`; VK/proof `.hex` files remain ordinary byte hex. JSON collection and aggregate structural limits are checked before allocating artifact trees, so oversized collections now fail at the JSON boundary.

On Linux, each traversed parent must be owned by the effective UID or root and must have no group/other write permission unless sticky. Newly created directories must be effective-UID-owned and private. Untrusted owners and writable nonsticky traversal parents are rejected with `Unsupported`; this can reject WSL DrvFS before publication is attempted. This boundary excludes same-UID and privileged attackers. Parent trust is checked before opening each next component and before every directory creation; an unexpected newly opened object is preserved, without chmod/chown or deletion.
