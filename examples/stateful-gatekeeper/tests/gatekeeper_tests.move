#[test_only]
module stateful_gatekeeper::gatekeeper_tests;

use stateful_gatekeeper::gatekeeper;
use stateful_gatekeeper::verifier;
use sui::tx_context;

const EInvalidProof: u64 = 0;
const EReplay: u64 = 1;
const EWrongDomain: u64 = 2;
const EWrongFingerprint: u64 = 3;

fun proof(): vector<u8> {
    x"4af94d64eb4c8a384c07b00c2744ecdbfeeb5d2d51283739ab4f279beefcdb949f98c5c87fd280bf525c57cbf3148bce69507627300622a9c4fd046b88aa9716eb19a5f79b77aa3252dc57bc487c8c59f4decab20be64a24e7845a07e094c310572546ee5e79efc990bb697e0f1b3026d9298f7d5475d4270698f872f5e5f208"
}

fun nullifier(): vector<u8> { x"1111111111111111111111111111111111111111111111111111111111111111" }

fun new_gate() : gatekeeper::Gatekeeper {
    let mut ctx = tx_context::dummy();
    gatekeeper::new(
        &mut ctx,
        b"gatekeeper/v1",
        @0xCAFE,
        b"sui-mainnet",
        b"mint",
        verifier::vk_fingerprint(),
    )
}

#[test]
fun first_use_with_valid_proof_succeeds() {
    let mut gate = new_gate();
    let n = nullifier();
    gatekeeper::authorize(&mut gate, proof(), b"gatekeeper/v1", copy n);
    assert!(gatekeeper::is_used(&gate, &n));
    gatekeeper::destroy_for_test(gate);
}

#[test, expected_failure(abort_code = EReplay, location = stateful_gatekeeper::gatekeeper)]
fun repeated_nullifier_is_rejected() {
    let mut gate = new_gate();
    let n = nullifier();
    gatekeeper::authorize(&mut gate, proof(), b"gatekeeper/v1", copy n);
    gatekeeper::authorize(&mut gate, proof(), b"gatekeeper/v1", n);
    gatekeeper::destroy_for_test(gate);
}

#[test, expected_failure(abort_code = EWrongDomain, location = stateful_gatekeeper::gatekeeper)]
fun wrong_domain_is_rejected() {
    let mut gate = new_gate();
    gatekeeper::authorize(&mut gate, proof(), b"other-domain", nullifier());
    gatekeeper::destroy_for_test(gate);
}

#[test, expected_failure(abort_code = EInvalidProof, location = stateful_gatekeeper::gatekeeper)]
fun different_package_cannot_reuse_proof() {
    let mut ctx = tx_context::dummy();
    let mut gate = gatekeeper::new(
        &mut ctx,
        b"gatekeeper/v1",
        @0xBEEF,
        b"sui-mainnet",
        b"mint",
        verifier::vk_fingerprint(),
    );
    gatekeeper::authorize(&mut gate, proof(), b"gatekeeper/v1", nullifier());
    gatekeeper::destroy_for_test(gate);
}

#[test, expected_failure(abort_code = EWrongFingerprint, location = stateful_gatekeeper::gatekeeper)]
fun wrong_vk_fingerprint_is_rejected() {
    let mut ctx = tx_context::dummy();
    let gate = gatekeeper::new(
        &mut ctx,
        b"gatekeeper/v1",
        @0xCAFE,
        b"sui-mainnet",
        b"mint",
        x"0000000000000000000000000000000000000000000000000000000000000000",
    );
    gatekeeper::destroy_for_test(gate);
}
