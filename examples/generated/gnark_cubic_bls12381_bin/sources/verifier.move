module gnark_cubic_bls12381_bin::verifier;

use sui::groth16;

const EInvalidProof: u64 = 1;

public fun verifying_key_bytes(): vector<u8> {
    x"8ff94a03dbc401754ae5b92f4df5b589f7a51c432b911321955f52bfe5c31683d05e7b99cdd4b8d69a16d437e3b8f97d8a560c1023556fbc66e68638642a9b020612eb11d72a23f03feee5323e4f3ad1dc82ef2cc42504ddf0d248ccbe237084043dc769762d8f8f1e881f2a68433680117ad31d5e7d5cc3371c07a0cd7eaa5b3f6776399c39caa0ef1ee077c647f2dfa74f0bea3b77eaa49bea35c3d5e1da85a4142031d1835c55d68eee5e313950fc224a6a126432851cb7198ad85787ca9204c30cc2320760c1fa6923710fb932b891402cfb2d68b63c7a65ecfbdf2f9acea8f301d5b3048689d317380fb7b7e47a96057a51496af1b6725b3142695216c39342e5f8dc2b8086eddae07efc0f54d6c3e0067f6afb036dacddeda568cf2e2017d2497d77d23dc629bef81861a6dafdb19f95065a7e2fd84ea2f42b6d80dd681b8159ca8488e4372f355fc0fa7a90180200000000000000b30ac0b397eb78a4e87e410b96c4242c8f89707bf450a859d45c1b48767f92f1021e3463a24b1696fe4236daaab872d8aac0a3033c7c6f146a92043a47870860185ce308143a6f470a1b1e3df4cca98790486307c9583dc59ab1391bb74da1da"
}

public fun prepare(): groth16::PreparedVerifyingKey {
    let curve = groth16::bls12381();
    let vk = verifying_key_bytes();
    groth16::prepare_verifying_key(&curve, &vk)
}

public fun verify(proof_bytes: vector<u8>, public_inputs_bytes: vector<u8>): bool {
    let curve = groth16::bls12381();
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
    let curve = groth16::bls12381();
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
