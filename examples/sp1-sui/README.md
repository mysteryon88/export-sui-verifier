# SP1 Sui Examples

This directory contains SP1-to-Sui examples: copied artifacts from
`SoundnessLabs/sp1-sui` plus a local `simple-sum/` project that regenerates an
SP1 6.x Groth16 proof from source.

Checked upstream revision:

```text
15d84fd54f8127c4a5c5fac6fad75eb888d46fa2
```

The copied upstream example is `fibonacci/`, built from:

- `proofs/fibonacci_proof.bin`
- `verifier/vk/v5.0.0/groth16_vk.bin`

The `simple-sum/` example is a tiny local SP1 project that shows the full flow
from guest source, to ELF, to Groth16 proof `.bin`, to generated Sui Move
verifier package. It is better for learning the process than the copied
Fibonacci artifact, but local Groth16 proving can still be memory-heavy on WSL.

The `simple-sum/` proof was generated with SP1 6.x and uses the newer Groth16
wrapper layout with five public inputs: program vkey hash, committed-values
digest, exit code, vk root, and proof nonce.

The upstream repository also contains `proofs/proof_jwt_verify_email_domain.bin`, but the checked-in JWT script currently saves a default SP1 core proof with `client.prove(...).run()` rather than a Groth16 wrapper proof with `.groth16().run()`. That file starts as SP1 proof variant `0`, while the Sui Groth16 path requires variant `3`.

To use the JWT example with this generator, regenerate that proof as an SP1 Groth16 proof and pass it with the v5 wrapper VK:

```sh
export-sui-verifier \
  --vk examples/sp1-sui/fibonacci/artifacts/groth16_vk_v5.bin \
  --proof path/to/jwt_groth16_proof.bin \
  --out examples/generated/sp1_sui_jwt_email_domain \
  --force \
  --run-sui-test
```
