#[test_only]
module gnark_cubic_bls12381_bin::verifier_tests;

use gnark_cubic_bls12381_bin::verifier;
use std::vector;

fun proof_bytes(): vector<u8> {
    x"89f8f8b3d346d4a2edd71097497bcb879825cea22d118122ef6589fa3b165a8775b630202135d00ddbd2c77dac6a5b2db0b12f9ec8c97008ed5a0776580de27c90ceedb807675622c0c2ea4f05676658535e592d29e55892c9ef7179c7aa1ac80c03f8d4f9ca7f6c6ec9a2315fd2165e3b2750aab23b911ea006844fe2aa5ed39b9d8f723d8f8315f2b8d212b96ec060a344c1487174e55e35a445efff39a034ec8b6317b94b1409398817507d7c482408fa5b2c46f2a570d4af83ecf2241aa7"
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
