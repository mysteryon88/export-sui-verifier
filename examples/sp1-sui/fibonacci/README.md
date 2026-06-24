# SP1 Sui Fibonacci Example

Artifacts copied from `SoundnessLabs/sp1-sui` revision `15d84fd54f8127c4a5c5fac6fad75eb888d46fa2`:

- `artifacts/fibonacci_proof.bin`: serialized Groth16 `SP1ProofWithPublicValues` from `proofs/fibonacci_proof.bin`.
- `artifacts/groth16_vk_v5.bin`: SP1 Groth16 wrapper verifying key from `verifier/vk/v5.0.0/groth16_vk.bin`.

Generate and test a Sui verifier:

```sh
cargo run -- \
  --vk examples/sp1-sui/fibonacci/artifacts/groth16_vk_v5.bin \
  --proof examples/sp1-sui/fibonacci/artifacts/fibonacci_proof.bin \
  --out examples/generated/sp1_sui_fibonacci \
  --force \
  --run-sui-test
```

The loader converts the SP1/Gnark BN254 wrapper proof into the normalized Arkworks-compatible Groth16 inputs expected by `sui::groth16`.

The `sp1-sui` repository also contains `proofs/proof_jwt_verify_email_domain.bin`, but the checked-in artifact is a default SP1 core proof, not a Groth16 wrapper proof. Regenerate that example with `.groth16().run()` before feeding it to this generator.
