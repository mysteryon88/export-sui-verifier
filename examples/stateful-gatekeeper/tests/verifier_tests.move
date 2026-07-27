#[test_only]
module stateful_gatekeeper::verifier_tests;

use stateful_gatekeeper::verifier;
use std::vector;

fun proof_bytes(): vector<u8> {
    x"4af94d64eb4c8a384c07b00c2744ecdbfeeb5d2d51283739ab4f279beefcdb949f98c5c87fd280bf525c57cbf3148bce69507627300622a9c4fd046b88aa9716eb19a5f79b77aa3252dc57bc487c8c59f4decab20be64a24e7845a07e094c310572546ee5e79efc990bb697e0f1b3026d9298f7d5475d4270698f872f5e5f208"
}

fun public_inputs_bytes(): vector<u8> {
    x"2615248c0a010455af186e8fc226c299562d254ad30f15216aa10bed71861702"
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

    let mut extra = public_inputs_bytes();
    extra.push_back(0);
    assert!(!verifier::verify(proof_bytes(), extra));
}

#[test]
fun reject_noncanonical_public_input_at_modulus() {
    assert!(!verifier::verify(
        proof_bytes(),
        x"010000f093f5e1439170b97948e833285d588181b64550b829a031e1724e6430",
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
