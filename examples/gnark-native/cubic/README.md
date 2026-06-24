# Gnark Native Cubic Example

This example contains native Gnark Groth16 artifacts for the cubic circuit:

```text
x^3 + x + 5 == y
```

The checked assignment uses private `x = 3` and public `y = 35`.

Artifacts were generated with Gnark v0.15.0 for both BN254 and BLS12-381:

- `main.go`: the Gnark circuit and artifact exporter.
- `artifacts/*/verification_key_gnark.json`: `json.Marshal` output of the native Gnark verifying key.
- `artifacts/*/proof_gnark.json`: `json.Marshal` output of the native Gnark proof.
- `artifacts/*/verification_key.bin`: native Gnark `vk.WriteTo` binary output.
- `artifacts/*/proof.bin`: native Gnark `proof.WriteTo` binary output.
- `artifacts/*/public.json`: ordered public inputs as decimal field elements.

Regenerate native Gnark artifacts from the repository root:

```sh
go run ./examples/gnark-native/cubic
```

Regenerate one curve only:

```sh
go run ./examples/gnark-native/cubic -curve bn254
```

Generate and test a Sui verifier from native Gnark JSON:

```sh
cargo run -- \
  --vk examples/gnark-native/cubic/artifacts/bn254/verification_key_gnark.json \
  --proof examples/gnark-native/cubic/artifacts/bn254/proof_gnark.json \
  --public examples/gnark-native/cubic/artifacts/bn254/public.json \
  --out examples/generated/gnark_cubic_bn254_json \
  --force \
  --run-sui-test
```

Generate and test a Sui verifier from native Gnark binary artifacts:

```sh
cargo run -- \
  --vk examples/gnark-native/cubic/artifacts/bls12381/verification_key.bin \
  --proof examples/gnark-native/cubic/artifacts/bls12381/proof.bin \
  --public examples/gnark-native/cubic/artifacts/bls12381/public.json \
  --out examples/generated/gnark_cubic_bls12381_bin \
  --force \
  --run-sui-test
```

No `--format` or `--curve` flag is required. The loader autodetects native Gnark JSON from the `G1/G2` and `Ar/Bs/Krs` shape, and native Gnark binary by trying the supported Gnark Groth16 binary layouts for BN254 and BLS12-381.
