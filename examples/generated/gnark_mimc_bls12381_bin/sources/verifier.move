module gnark_mimc_bls12381_bin::verifier;

use sui::groth16;

const EInvalidProof: u64 = 1;

public fun verifying_key_bytes(): vector<u8> {
    x"a0ca5ae832e8c13a000047952340472e5f8bd15ffbf3e37ea3b61589c16e75d47874a2c708b55b08773bd62ed8ec1b0183870500e3fb1bb922aec5e8fdb99b64d1b56011243520b9ff52c90395270a0b4f707888cdb6d3cc0b703b208d4175120b4a31dbaa498c67524e36888b7448da2a0215ec09327ed3eb0b0a581f30e4d9dcff55a220a0136098fb3ca3bf5a45e698a6ee5dbbaf8edcb18574770626deaf8eb85b7204fd4f28d8d71c4a5fb69125bc35fe4b5b89142c5d838e9ddfb6f37c0c094eb2947c92c3fd132e06800021de15ff26cad4ed23c3162afccb0b002af5ef00150bfb0fa30fd9e5e5903589d6e084ffe10105d3172179e3d2e08b603b1b738db43b79d83ff21cf45de5a59f992d7ea4be079a32de932f61aedf10157fc708e9b026bcf26ce40307e80dc25c423edb251ff1b232c23155a0139a7a1187833ffb787e4b820c3b9bb243452febdd480200000000000000a2fd6547b3fd7b27f4f3d05ad0a1b0f781671b530077e0df10ff37af684c2fd9f6a7b71a454ba1d9b59e390abdfb1e6fa386845a02b7a9b6250b4698d2d5a9e85b27e37bdf1fc71c7d26292a8abda5d3be901eb0b0a979ba8946077870182fc2"
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
