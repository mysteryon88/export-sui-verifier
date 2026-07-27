module stateful_gatekeeper::verifier;

use sui::groth16;
use std::vector;

const EXPECTED_PROOF_BYTES: u64 = 128;
const EXPECTED_PUBLIC_INPUTS_BYTES: u64 = 32;

public struct BoundPreparedVerifyingKey has drop {
    inner: groth16::PreparedVerifyingKey,
}

public fun verifying_key_bytes(): vector<u8> {
    x"435094ed34976dedcbcf4b39dc10f7a1c5c058693c8ef62b7b12b52062442c12a698531519eeff82ca9b5b7d5dcc2358d92aed2c05531d9fd74ea99cab9831256241fc2a44d8717d3005e1b756a793e0e091b519628e789217fbbde7b3fc7280cd08e9da7202efde62b8d5d8c0454c0db62da36df3120800128ae18223193d15fabdc8f4b924b6b656564ed5e36939b86f6bddd37d66d41dfdf35c622c3dd907c03660b18e18b77e6df70025922fc4e1964cdc2dd8bb5a249efa1d22d925f40ab477f203a5099bb6f989f04a24630feef0af8c6da8266b61290db43833171e820200000000000000ca70cc44ee7a5b2ace75016674c300f4072725aab20215c4bde2f9ffebdcb6af6a48279ba3c8ea4bcd84162655495c66c592b66f15dc0011d1df759c3b5319a8"
}

public fun vk_fingerprint(): vector<u8> {
    x"f38c48c808c9a2913e3db02b3f7cefe08500c972ddfdfa04312ce7171bcefc35"
}

fun prepare_embedded(): groth16::PreparedVerifyingKey {
    let curve = groth16::bn254();
    let vk = verifying_key_bytes();
    groth16::prepare_verifying_key(&curve, &vk)
}

public fun prepare_bound(): BoundPreparedVerifyingKey {
    BoundPreparedVerifyingKey { inner: prepare_embedded() }
}

public fun verify(proof_bytes: vector<u8>, public_inputs_bytes: vector<u8>): bool {
    let prepared = prepare_bound();
    verify_with_bound_prepared(&prepared, proof_bytes, public_inputs_bytes)
}

public fun verify_with_bound_prepared(
    prepared_verifying_key: &BoundPreparedVerifyingKey,
    proof_bytes: vector<u8>,
    public_inputs_bytes: vector<u8>,
): bool {
    if (
        vector::length(&proof_bytes) != EXPECTED_PROOF_BYTES
        || vector::length(&public_inputs_bytes) != EXPECTED_PUBLIC_INPUTS_BYTES
    ) {
        return false
    };
    let curve = groth16::bn254();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);
    let public_inputs = groth16::public_proof_inputs_from_bytes(public_inputs_bytes);

    groth16::verify_groth16_proof(
        &curve,
        &prepared_verifying_key.inner,
        &public_inputs,
        &proof_points,
    )
}
