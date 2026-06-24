#[test_only]
module gnark_cubic_bn254_json::verifier_tests;

use gnark_cubic_bn254_json::verifier;
use std::vector;

fun proof_bytes(): vector<u8> {
    x"94ca5231d53c6443653714c9f70ce521974c13fd1a40a810c034f6881dabb9a2c7250f51a243bee4038947ba57aaf5977a6af957a8a9b0122f654dc0e685352fbee72507ae149f1494bff7f685658d07bba9083d18286704a1c370b62b801e231b0d2e06da743f6c5c689b37ecab0985fbd7a33b9d053e0ab220532507c7a203"
}

fun public_inputs_bytes(): vector<u8> {
    x"2300000000000000000000000000000000000000000000000000000000000000"
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
