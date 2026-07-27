module stateful_gatekeeper::gatekeeper;

use stateful_gatekeeper::verifier;
use std::vector;
use sui::bcs;
use sui::object::{Self, UID};
use sui::table::{Self, Table};
use sui::tx_context::TxContext;

const EInvalidProof: u64 = 0;
const EReplay: u64 = 1;
const EWrongDomain: u64 = 2;
const EWrongFingerprint: u64 = 3;

const CONTEXT_MASK: vector<u8> = x"367a69520e77843895b54ceacb38b1f33844f1701e81d24eeb90277697739a";

public struct Gatekeeper has key {
    id: UID,
    domain: vector<u8>,
    package_id: address,
    chain: vector<u8>,
    operation: vector<u8>,
    used_nullifiers: Table<vector<u8>, bool>,
}

public(package) fun new(
    ctx: &mut TxContext,
    domain: vector<u8>,
    package_id: address,
    chain: vector<u8>,
    operation: vector<u8>,
    expected_vk_fingerprint: vector<u8>,
): Gatekeeper {
    assert!(expected_vk_fingerprint == verifier::vk_fingerprint(), EWrongFingerprint);
    Gatekeeper {
        id: object::new(ctx),
        domain,
        package_id,
        chain,
        operation,
        used_nullifiers: table::new(ctx),
    }
}

public fun authorize(
    gate: &mut Gatekeeper,
    proof: vector<u8>,
    domain: vector<u8>,
    nullifier: vector<u8>,
) {
    assert!(domain == gate.domain, EWrongDomain);
    assert!(!table::contains(&gate.used_nullifiers, copy nullifier), EReplay);

    let public_input = context_public_input(
        &domain,
        &nullifier,
        &verifier::vk_fingerprint(),
        gate.package_id,
        &gate.chain,
        &gate.operation,
    );
    assert!(verifier::verify(proof, public_input), EInvalidProof);

    table::add(&mut gate.used_nullifiers, nullifier, true);
}

public fun is_used(gate: &Gatekeeper, nullifier: &vector<u8>): bool {
    table::contains(&gate.used_nullifiers, *nullifier)
}

public fun context_public_input(
    domain: &vector<u8>,
    nullifier: &vector<u8>,
    vk_fingerprint: &vector<u8>,
    package_id: address,
    chain: &vector<u8>,
    operation: &vector<u8>,
): vector<u8> {
    let mut encoded = vector[];
    append_field(&mut encoded, &b"groth16-gatekeeper-v1");
    append_field(&mut encoded, domain);
    append_field(&mut encoded, nullifier);
    append_field(&mut encoded, vk_fingerprint);
    append_field(&mut encoded, &bcs::to_bytes(&package_id));
    append_field(&mut encoded, chain);
    append_field(&mut encoded, operation);

    let mut digest = std::hash::sha2_256(encoded);
    xor_mask(&mut digest);
    digest
}

fun append_field(encoded: &mut vector<u8>, field: &vector<u8>) {
    assert!(vector::length(field) < 256, EInvalidProof);
    vector::push_back(encoded, vector::length(field) as u8);
    let field_copy = *field;
    vector::append(encoded, field_copy);
}

fun xor_mask(bytes: &mut vector<u8>) {
    let mask_bytes = CONTEXT_MASK;
    let mut i = 0;
    while (i < 31) {
        let mask = *vector::borrow(&mask_bytes, i);
        let byte = vector::borrow_mut(bytes, i);
        *byte = *byte ^ mask;
        i = i + 1;
    };
    let _ = vector::pop_back(bytes);
    vector::push_back(bytes, 2);
}

#[test_only]
public fun destroy_for_test(gate: Gatekeeper) {
    let Gatekeeper { id, domain: _, package_id: _, chain: _, operation: _, used_nullifiers } = gate;
    table::drop(used_nullifiers);
    object::delete(id);
}
