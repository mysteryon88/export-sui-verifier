# export-sui-verifier examples

Fixtures and generated Sui Move packages for `export-sui-verifier`.

Run commands in this file from the `export-sui-verifier` directory.

## What Is Here

- `ark-mimc`: Rust arkworks example that exports BN254 and BLS12-381 snarkjs JSON plus compact `groth16_artifacts.json` bundles.
- `MulCircuit`: Rust BLS12-381 multiplication circuit that exports snarkjs-style `verification_key.json`, `proof.json`, and `public.json`.
- `gnark-native/cubic`: native Gnark cubic JSON and binary Groth16 artifacts for BN254 and BLS12-381.
- `gnark-native/mimc`: native Gnark MiMC preimage JSON and binary Groth16 artifacts for BN254 and BLS12-381.
- `generated`: Sui Move packages generated from the checked artifacts.

Proof-based generated packages include `tests/verifier_tests.move`. VK-only packages are generated without tests and are checked by build.

## 1. Regenerate Source Artifacts

This is optional if the checked-in artifacts are already current. Run it when you changed the example circuits.

```sh
cd ./examples/ark-mimc
cargo run -- export bn254 artifacts
cargo run -- export bls12_381 artifacts
cd ../..

cd ./examples/MulCircuit
cargo run
cd ../..

go run ./examples/gnark-native/cubic
go run ./examples/gnark-native/mimc
```

## 2. Generate Proof Packages

These commands regenerate the proof-based Sui Move packages under `examples/generated`. They also run local Rust Groth16 verification before writing the Move package.

```sh
cargo run -- --bundle examples/ark-mimc/artifacts/bn254/groth16_artifacts.json --out examples/generated/ark_mimc_bn254_arkworks --force

cargo run -- --bundle examples/ark-mimc/artifacts/bls12_381/groth16_artifacts.json --out examples/generated/ark_mimc_bls12381_arkworks --force

cargo run -- --vk examples/ark-mimc/artifacts/bn254/verification_key.json --proof examples/ark-mimc/artifacts/bn254/proof.json --out examples/generated/ark_mimc_bn254_snarkjs --force

cargo run -- --vk examples/ark-mimc/artifacts/bls12_381/verification_key.json --proof examples/ark-mimc/artifacts/bls12_381/proof.json --out examples/generated/ark_mimc_bls12381_snarkjs --force

cargo run -- --vk examples/MulCircuit/artifacts/bls12_381/verification_key.json --proof examples/MulCircuit/artifacts/bls12_381/proof.json --public examples/MulCircuit/artifacts/bls12_381/public.json --out examples/generated/mul_circuit_bls12381_snarkjs --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bn254/verification_key_gnark.json --proof examples/gnark-native/cubic/artifacts/bn254/proof_gnark.json --public examples/gnark-native/cubic/artifacts/bn254/public.json --out examples/generated/gnark_cubic_bn254_json --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bls12381/verification_key_gnark.json --proof examples/gnark-native/cubic/artifacts/bls12381/proof_gnark.json --public examples/gnark-native/cubic/artifacts/bls12381/public.json --out examples/generated/gnark_cubic_bls12381_json --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bn254/verification_key.bin --proof examples/gnark-native/cubic/artifacts/bn254/proof.bin --public examples/gnark-native/cubic/artifacts/bn254/public.json --out examples/generated/gnark_cubic_bn254_bin --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bls12381/verification_key.bin --proof examples/gnark-native/cubic/artifacts/bls12381/proof.bin --public examples/gnark-native/cubic/artifacts/bls12381/public.json --out examples/generated/gnark_cubic_bls12381_bin --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bn254/verification_key_gnark.json --proof examples/gnark-native/mimc/artifacts/bn254/proof_gnark.json --public examples/gnark-native/mimc/artifacts/bn254/public.json --out examples/generated/gnark_mimc_bn254_json --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bls12381/verification_key_gnark.json --proof examples/gnark-native/mimc/artifacts/bls12381/proof_gnark.json --public examples/gnark-native/mimc/artifacts/bls12381/public.json --out examples/generated/gnark_mimc_bls12381_json --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bn254/verification_key.bin --proof examples/gnark-native/mimc/artifacts/bn254/proof.bin --public examples/gnark-native/mimc/artifacts/bn254/public.json --out examples/generated/gnark_mimc_bn254_bin --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bls12381/verification_key.bin --proof examples/gnark-native/mimc/artifacts/bls12381/proof.bin --public examples/gnark-native/mimc/artifacts/bls12381/public.json --out examples/generated/gnark_mimc_bls12381_bin --force
```

Add `--run-sui-test` to any command above to run `sui move test` inside the generated package immediately after generation.

## 3. Run Sui Move Tests

Run these after generation to verify the generated Move packages on Sui.

```sh
(cd examples/generated/ark_mimc_bn254_arkworks && sui move test)
(cd examples/generated/ark_mimc_bls12381_arkworks && sui move test)
(cd examples/generated/ark_mimc_bn254_snarkjs && sui move test)
(cd examples/generated/ark_mimc_bls12381_snarkjs && sui move test)
(cd examples/generated/mul_circuit_bls12381_snarkjs && sui move test)
(cd examples/generated/gnark_cubic_bn254_json && sui move test)
(cd examples/generated/gnark_cubic_bls12381_json && sui move test)
(cd examples/generated/gnark_cubic_bn254_bin && sui move test)
(cd examples/generated/gnark_cubic_bls12381_bin && sui move test)
(cd examples/generated/gnark_mimc_bn254_json && sui move test)
(cd examples/generated/gnark_mimc_bls12381_json && sui move test)
(cd examples/generated/gnark_mimc_bn254_bin && sui move test)
(cd examples/generated/gnark_mimc_bls12381_bin && sui move test)
```

## 4. Generate VK-Only Packages

VK-only packages prove that the verifier can be generated from a verification key alone. They do not contain `tests/`.

For snarkjs JSON, omit `--proof`:

```sh
cargo run -- --vk examples/ark-mimc/artifacts/bn254/verification_key.json --out examples/generated/ark_mimc_bn254_snarkjs_vk_only --force

cargo run -- --vk examples/ark-mimc/artifacts/bls12_381/verification_key.json --out examples/generated/ark_mimc_bls12381_snarkjs_vk_only --force

cargo run -- --vk examples/MulCircuit/artifacts/bls12_381/verification_key.json --out examples/generated/mul_circuit_bls12381_snarkjs_vk_only --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bn254/verification_key_gnark.json --out examples/generated/gnark_cubic_bn254_json_vk_only --force

cargo run -- --vk examples/gnark-native/cubic/artifacts/bls12381/verification_key.bin --out examples/generated/gnark_cubic_bls12381_bin_vk_only --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bn254/verification_key_gnark.json --out examples/generated/gnark_mimc_bn254_json_vk_only --force

cargo run -- --vk examples/gnark-native/mimc/artifacts/bls12381/verification_key.bin --out examples/generated/gnark_mimc_bls12381_bin_vk_only --force
```

For Arkworks VK-only packages, create temporary VK-only JSON files from the full bundles, then pass them through `--vk`:

```sh
mkdir -p target/tmp-vk-only

jq -c '{ curve, verification_key: .vk }' \
  examples/ark-mimc/artifacts/bn254/groth16_artifacts.json \
  > target/tmp-vk-only/ark_mimc_bn254_vk_only.json

jq -c '{ curve, verification_key: .vk }' \
  examples/ark-mimc/artifacts/bls12_381/groth16_artifacts.json \
  > target/tmp-vk-only/ark_mimc_bls12381_vk_only.json

cargo run -- --vk target/tmp-vk-only/ark_mimc_bn254_vk_only.json --out examples/generated/ark_mimc_bn254_arkworks_vk_only --force

cargo run -- --vk target/tmp-vk-only/ark_mimc_bls12381_vk_only.json --out examples/generated/ark_mimc_bls12381_arkworks_vk_only --force
```

## 5. Check VK-Only Packages

Run build checks because VK-only packages do not have generated tests.

```sh
(cd examples/generated/ark_mimc_bn254_snarkjs_vk_only && sui move build)
(cd examples/generated/ark_mimc_bls12381_snarkjs_vk_only && sui move build)
(cd examples/generated/mul_circuit_bls12381_snarkjs_vk_only && sui move build)
(cd examples/generated/gnark_cubic_bn254_json_vk_only && sui move build)
(cd examples/generated/gnark_cubic_bls12381_bin_vk_only && sui move build)
(cd examples/generated/gnark_mimc_bn254_json_vk_only && sui move build)
(cd examples/generated/gnark_mimc_bls12381_bin_vk_only && sui move build)
(cd examples/generated/ark_mimc_bn254_arkworks_vk_only && sui move build)
(cd examples/generated/ark_mimc_bls12381_arkworks_vk_only && sui move build)
```

## Proof Data Helpers

Use `proof-data` when you have a VK-only package and want to print Move helper functions for a later test file.

```sh
cargo run -- proof-data --vk examples/ark-mimc/artifacts/bn254/verification_key.json --proof examples/ark-mimc/artifacts/bn254/proof.json
```
