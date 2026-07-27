# stateful_gatekeeper

Generated Sui Move Groth16 verifier package.

## Generated API

The verifier module is `stateful_gatekeeper::verifier`.

- Curve: `bn254`
- `verifying_key_bytes()`
- `vk_fingerprint()`
- `prepare_bound()`
- `verify(proof_bytes, public_inputs_bytes)`
- `verify_with_bound_prepared(bound_prepared_verifying_key, proof_bytes, public_inputs_bytes)`


The prepared-key wrapper has a private field and can only be constructed from the embedded verification key. The verifier uses `sui::groth16` and expects Arkworks canonical compressed proof bytes plus concatenated canonical 32-byte little-endian public inputs.

## Regenerate

Run `export-sui-verifier` with root-level generation flags:

```sh
export-sui-verifier --vk ./verification_key.json --out ./generated --force
export-sui-verifier --bundle ./groth16_artifacts.json --out ./generated --force
```

Add `--proof ./proof.json` and optional `--public ./public.json` to include local proof verification and generated Move tests.

Useful flags:

- `--package-name stateful_gatekeeper`
- `--module-name verifier`
- `--mode library|entry|test`
- `--run-sui-test`
- `--skip-local-verify`

VK-only packages are generated without `tests/`. To print proof helpers for a later test file, run:

```sh
export-sui-verifier proof-data --vk ./verification_key.json --proof ./proof.json
```

## Stateful authorization flow

`gatekeeper.move` keeps the generated verifier stateless and adds an application-owned `Gatekeeper` object. Its constructor is `public(package)`, so untrusted callers cannot create fresh replay tables; the trusted package must publish and retain one canonical object for each protected operation. The object stores its domain, package identifier, chain, operation, and a `Table` of used nullifiers. `authorize` derives the sole public input from all of those values plus `verifier::vk_fingerprint()`, verifies the real BN254 proof, and only then inserts the nullifier.

The fixed `CONTEXT_MASK` is a 248-bit output encoding chosen so this example can reuse the repository's existing proof fixture while always producing a canonical scalar. The final byte is fixed and the remaining 31 bytes are `SHA-256(context) XOR mask`. A production circuit may instead prove an unmasked hash-to-field output directly.

The context encoding length-prefixes every field. Its 32-byte output is the exact public input proved by the circuit; domain and nullifier are not side arguments to an unrelated proof.

Move tests cover a first valid authorization, replay of the same nullifier, another domain, another package identifier, and a wrong VK fingerprint. Run them with:

```sh
sui move test --path examples/stateful-gatekeeper
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
