#[test_only]
module gnark_mimc_bn254_bin::verifier_tests;

use gnark_mimc_bn254_bin::verifier;
use std::vector;

fun proof_bytes(): vector<u8> {
    x"d76ff90ccb4e340932132cf2d3f53d68a2d23f2c834f087ef3cc7d97132adc186bf2f28fad46ecd22ac249eebd85f5a5d1c29bf204e60b25ef0d1bdbccc47f0741e75e239b6a7508d0d6da88f7448e923cbe525a44b076d2f1946c67b5b78993f45197158f4d28b601e0fec48b9df2f5e03b80864680a5efab5454ee2b21e311"
}

fun public_inputs_bytes(): vector<u8> {
    x"1dd1cd20f8b5918eddb13a298588631705408847e7a780c72a88e4157692630e"
}

#[test]
fun verify_valid_proof() {
    assert!(verifier::verify(proof_bytes(), public_inputs_bytes()));
}

#[test]
fun reject_invalid_proof() {
    let mut proof = proof_bytes();
    let _last = proof.pop_back();
    proof.push_back(0);
    assert!(!verifier::verify(proof, public_inputs_bytes()));
}

#[test]
fun reject_invalid_public_input() {
    let mut public_inputs = public_inputs_bytes();
    if (vector::is_empty(&public_inputs)) {
        let mut invalid_proof = proof_bytes();
        let last = invalid_proof.pop_back();
        if (last == 0) {
            invalid_proof.push_back(1);
        } else {
            invalid_proof.push_back(0);
        };
        assert!(!verifier::verify(invalid_proof, public_inputs));
    } else {
        let last = public_inputs.pop_back();
        if (last == 0) {
            public_inputs.push_back(1);
        } else {
            public_inputs.push_back(0);
        };
        assert!(!verifier::verify(proof_bytes(), public_inputs));
    };
}
