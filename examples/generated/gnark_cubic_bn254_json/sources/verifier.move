module gnark_cubic_bn254_json::verifier;

use sui::groth16;

const EInvalidProof: u64 = 1;

public fun verifying_key_bytes(): vector<u8> {
    x"d8fb0000d816c759a0f0ead21892e9f0aa0ba8f958ed1e20a1c2fa2903fafa19df37f258e0917cd178e528a793fc9898bc53a8f5cd0cc991f0322c93cd54aa10dd4a7c478428a2603ff3d05b79c50768bca50689d7f541406538ec469570289ebeab762a5ef08ab36eab9e2d013e6f3ee48563317badfd408ede033a157ebe2ba0165014791b20c8c9804444108ffe071357a02c81e503a79289ccb0850e790fba94a183d1a271b2e1bf440d263ccfc740e2d864452743f97fc479278448042eff38ad60d41828aea413ec7d38eab467aecee405c5ff9027a566bf9c9a7640900200000000000000b6c02890a630f83cb44161bce0354fc1b262e84754b7c4f2fefc0e9d447d36102067cdf2145b8ca790c82705e7e9635b1cf6ce7fea6aaeecc5b15f9e770ba881"
}

public fun prepare(): groth16::PreparedVerifyingKey {
    let curve = groth16::bn254();
    let vk = verifying_key_bytes();
    groth16::prepare_verifying_key(&curve, &vk)
}

public fun verify(proof_bytes: vector<u8>, public_inputs_bytes: vector<u8>): bool {
    let curve = groth16::bn254();
    let pvk = prepare();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);
    let public_inputs = groth16::public_proof_inputs_from_bytes(public_inputs_bytes);

    groth16::verify_groth16_proof(
        &curve,
        &pvk,
        &public_inputs,
        &proof_points,
    )
}

public fun verify_with_prepared(
    prepared_verifying_key: &groth16::PreparedVerifyingKey,
    proof_bytes: vector<u8>,
    public_inputs_bytes: vector<u8>,
): bool {
    let curve = groth16::bn254();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);
    let public_inputs = groth16::public_proof_inputs_from_bytes(public_inputs_bytes);

    groth16::verify_groth16_proof(
        &curve,
        prepared_verifying_key,
        &public_inputs,
        &proof_points,
    )
}

entry fun verify_entry(proof_bytes: vector<u8>, public_inputs_bytes: vector<u8>) {
    assert!(verify(proof_bytes, public_inputs_bytes), EInvalidProof);
}
