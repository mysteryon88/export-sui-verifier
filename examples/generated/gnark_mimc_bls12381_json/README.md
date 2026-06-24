# gnark_mimc_bls12381_json

Generated Sui Move Groth16 verifier package.

## Generated API

The verifier module is `gnark_mimc_bls12381_json::verifier`.

- Curve: `bls12381`
- `verifying_key_bytes()`
- `prepare()`
- `verify(proof_bytes, public_inputs_bytes)`
- `verify_with_prepared(prepared_verifying_key, proof_bytes, public_inputs_bytes)`
- `verify_entry(proof_bytes, public_inputs_bytes)` when generated in `entry` or `test` mode

The verifier uses `sui::groth16` and expects Arkworks canonical compressed proof bytes plus concatenated 32-byte little-endian public inputs.

## Regenerate

Run `export-sui-verifier` with root-level generation flags:

```sh
export-sui-verifier --vk ./verification_key.json --out ./generated --force
export-sui-verifier --bundle ./groth16_artifacts.json --out ./generated --force
```

Add `--proof ./proof.json` and optional `--public ./public.json` to include local proof verification and generated Move tests.

Useful flags:

- `--package-name gnark_mimc_bls12381_json`
- `--module-name verifier`
- `--mode library|entry|test`
- `--run-sui-test`
- `--skip-local-verify`

VK-only packages are generated without `tests/`. To print proof helpers for a later test file, run:

```sh
export-sui-verifier proof-data --vk ./verification_key.json --proof ./proof.json
```

## Tests
This package includes Move unit tests with the proof/public inputs supplied at generation time.

Run:

```sh
sui move test
```

## Known limitations

- Supported curves: BN254 and BLS12-381.
- The curve and input format are inferred from artifact metadata.
- Generated verifier code is not audited. Review it before production use.
