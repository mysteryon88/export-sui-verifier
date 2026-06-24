module gnark_mimc_bn254_json::verifier;

use sui::groth16;

const EInvalidProof: u64 = 1;

public fun verifying_key_bytes(): vector<u8> {
    x"0d01e5168d5162ecea80a11de7cdb735b155f30376e1aef1b95f4b73e285062e59fdaa97132c921a19820d090bb22a2cdce6bf23f3f38a34e97a4a8cab3da02d5012e0fced56e61e05d1cbb8248ab06343e3395e2fcb9bb171816ba114e49615d3cd5f1573ce7c18922214ce9c3f10014cb54f81252ae6c3ea292d39cd02a1128f679abaca73bb174adeb71f1f30eb078140e2338b430299e450ab50935f7aa7abf90b3d39e7cc4e1c4b27bd2aef9a05210e31d3b70482e1dad5f47e3974ce20982bbb991e5574419587e840d60ee4992680b9bb5b66fc46d600072346485c8c02000000000000004fc12c33d04f5e6a2a1cb7fe1ef135936b600376aa4f04b948be89c5c8ff870ad5712bce472ab3f4a31d6f51df5a99e8588891240c9dd64b1f7397dc3373bba3"
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
