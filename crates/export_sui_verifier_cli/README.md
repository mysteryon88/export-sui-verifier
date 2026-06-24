# export-sui-verifier

CLI for generating Sui Move Groth16 verifier packages.

Generation uses root-level flags:

```sh
export-sui-verifier \
  --vk ./verification_key.json \
  --proof ./proof.json \
  --public ./public.json \
  --out ./generated/my_verifier \
  --force
```

Compact Arkworks bundle mode:

```sh
export-sui-verifier \
  --bundle ./groth16_artifacts.json \
  --out ./generated/arkworks_verifier \
  --force
```

Common options:

- `--package-name <name>`: defaults to the sanitized `--out` directory name
- `--module-name <name>`: defaults to `verifier`
- `--mode library|entry|test`: defaults to `entry`
- `--run-sui-test`: runs `sui move test` inside the generated package
- `--skip-local-verify`: skips local Arkworks proof verification
- `--force`: overwrites the output directory

`--proof` is optional. Supplying proof data enables local verification and generated Move tests. `--public` is optional when `proof.json` already contains `publicSignals`.

`proof-data` is the only subcommand:

```sh
export-sui-verifier proof-data \
  --vk ./verification_key.json \
  --proof ./proof.json
```

It prints Move helper functions for `proof_bytes()` and `public_inputs_bytes()` using the same serialization as generated tests.

Supported inputs:

- snarkjs-compatible JSON
- Arkworks VK/proof JSON or raw compressed hex inputs
- compact Arkworks bundles
- native Gnark JSON
- native Gnark `WriteTo` binary artifacts
- SP1 BN254 Groth16 wrapper VK/proof artifacts

Arkworks raw hex and bundles without `curve` metadata are auto-detected when
exactly one supported curve parses and verifies the supplied artifacts.

Supported curves:

- BN254
- BLS12-381

MIT.
