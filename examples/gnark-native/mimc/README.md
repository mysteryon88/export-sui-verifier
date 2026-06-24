# Gnark Native MiMC Example

This example contains native Gnark Groth16 artifacts for proving knowledge of a MiMC preimage whose digest equals a public hash.

- Private input: `PreImage = 500304`
- Public input: `Hash`
- Curves: BN254 and BLS12-381

Artifacts were generated with Gnark v0.15.0 for both curves.

Regenerate native Gnark JSON and binary artifacts from the repository root:

```sh
go run ./examples/gnark-native/mimc
```

Regenerate one curve only:

```sh
go run ./examples/gnark-native/mimc -curve bn254
```

Generate and test a Sui verifier from native Gnark JSON:

```sh
cargo run -- \
  --vk examples/gnark-native/mimc/artifacts/bn254/verification_key_gnark.json \
  --proof examples/gnark-native/mimc/artifacts/bn254/proof_gnark.json \
  --public examples/gnark-native/mimc/artifacts/bn254/public.json \
  --out examples/generated/gnark_mimc_bn254_json \
  --force \
  --run-sui-test
```

Generate and test a Sui verifier from native Gnark binary artifacts:

```sh
cargo run -- \
  --vk examples/gnark-native/mimc/artifacts/bls12381/verification_key.bin \
  --proof examples/gnark-native/mimc/artifacts/bls12381/proof.bin \
  --public examples/gnark-native/mimc/artifacts/bls12381/public.json \
  --out examples/generated/gnark_mimc_bls12381_bin \
  --force \
  --run-sui-test
```
