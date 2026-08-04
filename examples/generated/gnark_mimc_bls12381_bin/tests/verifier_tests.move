#[test_only]
module gnark_mimc_bls12381_bin::verifier_tests;

use gnark_mimc_bls12381_bin::verifier;
use std::vector;

fun proof_bytes(): vector<u8> {
    x"b611683d8e80e715712d3ba11fc3f21db68bd75c722aab3a022274abd20ee68b73acf803722bd376c272fce61b23d612a594c1176d57215609a881c2f8772bdebcf9f8f6d68950e1d23d7aa4f0a11cf293fbbfbda9283203c723331ecc58ed6d033613d2789c4a0728c2c90f032492b0d63cbdee37116f2a4a81a91b200b5d30c18a1b5c0cbf24db19946685c3e409be8515c0f2a4c9f4b8bfbc0628a2f3c994773fb59e2f225169380b3288786853444e61fd4c09570ef8e5d8e6e5fc1dd582"
}

fun public_inputs_bytes(): vector<u8> {
    x"10ddc071b4a783f847900d545cdee7bb32c820feee6eda3f5da7729674754408"
}

#[test]
fun verify_valid_proof() {
    assert!(verifier::verify(proof_bytes(), public_inputs_bytes()));
}

#[test]
fun reject_invalid_proof() {
    let mut proof = proof_bytes();
    let last = proof.pop_back();
    if (last == 0) {
        proof.push_back(1);
    } else {
        proof.push_back(0);
    };
    assert!(!verifier::verify(proof, public_inputs_bytes()));
}

#[test]
fun verify_valid_proof_with_bound_prepared_key() {
    let prepared = verifier::prepare_bound();
    assert!(verifier::verify_with_bound_prepared(
        &prepared,
        proof_bytes(),
        public_inputs_bytes(),
    ));
}

#[test]
fun reject_wrong_public_input_lengths() {
    let mut truncated = public_inputs_bytes();
    if (vector::is_empty(&truncated)) {
        truncated.push_back(0);
    } else {
        let _last = truncated.pop_back();
    };
    assert!(!verifier::verify(proof_bytes(), truncated));

    let mut extra_scalar = public_inputs_bytes();
    let mut i = 0;
    while (i < 32) {
        extra_scalar.push_back(0);
        i = i + 1;
    };
    assert!(!verifier::verify(proof_bytes(), extra_scalar));
}

#[test]
fun reject_noncanonical_public_input_at_modulus() {
    assert!(!verifier::verify(
        proof_bytes(),
        x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
    ));
}

#[test]
fun reject_noncanonical_public_input_at_modulus_plus_one() {
    assert!(!verifier::verify(
        proof_bytes(),
        x"02000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73",
    ));
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
