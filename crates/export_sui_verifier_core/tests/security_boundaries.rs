use std::fs;
use std::path::{Path, PathBuf};

use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalSerialize;
use export_sui_verifier_core::{
    create_adapter, load_gnark_json_inputs, load_snarkjs_json_inputs, local_verify,
    parse_compact_artifact, parse_public_inputs, parse_verification_key, Error,
    Groth16VerifierInputs,
};
use serde_json::json;

fn fixture_dir(curve: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join(if curve == "bls12381" {
            "bls12_381"
        } else {
            curve
        })
}

fn valid_inputs(curve: &str) -> Groth16VerifierInputs {
    let dir = fixture_dir(curve);
    load_snarkjs_json_inputs(
        &dir.join("verification_key.json"),
        &dir.join("proof.json"),
        None,
    )
    .unwrap()
}

fn compressed_hex(value: &impl CanonicalSerialize) -> String {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes).unwrap();
    hex::encode(bytes)
}

fn write_json(path: &Path, value: &serde_json::Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

#[test]
fn local_verification_validates_shape_through_wrapper_and_adapter() {
    for curve in ["bn254", "bls12381"] {
        let adapter = create_adapter(curve).unwrap();
        let inputs = valid_inputs(curve);
        assert!(local_verify(adapter.as_ref(), &inputs).unwrap());
        assert!(adapter.local_verify(&inputs).unwrap());

        let mut trailing = inputs.clone();
        trailing.public_inputs.push("0".to_string());
        assert!(matches!(
            local_verify(adapter.as_ref(), &trailing),
            Err(Error::PublicInputCountMismatch(_))
        ));
        assert!(matches!(
            adapter.local_verify(&trailing),
            Err(Error::PublicInputCountMismatch(_))
        ));

        let mut empty_ic = inputs;
        empty_ic.verifying_key.ic.clear();
        assert!(matches!(
            local_verify(adapter.as_ref(), &empty_ic),
            Err(Error::IcLengthMismatch(_))
        ));
        assert!(matches!(
            adapter.local_verify(&empty_ic),
            Err(Error::IcLengthMismatch(_))
        ));
    }
}

#[test]
fn text_parsers_reject_oversized_files_arrays_and_scalars() {
    let dir = tempfile::tempdir().unwrap();
    let oversized = dir.path().join("oversized.json");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    assert!(matches!(
        parse_verification_key(&oversized),
        Err(Error::InputTooLarge { .. })
    ));

    let public = dir.path().join("public.json");
    write_json(&public, &json!(["9".repeat(257)]));
    assert!(matches!(
        parse_public_inputs(&public),
        Err(Error::DecimalParse(_))
    ));

    write_json(
        &public,
        &json!(["0", "0", "0", "0", "0", "0", "0", "0", "0"]),
    );
    assert!(matches!(
        parse_public_inputs(&public),
        Err(Error::PublicInputCountMismatch(_))
    ));
}

#[test]
fn gnark_json_caps_k_before_coordinate_conversion() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bn254")
        .join("verification_key_gnark.json");
    let mut vk: serde_json::Value = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
    let k = vk["G1"]["K"].as_array_mut().unwrap();
    let point = k[0].clone();
    while k.len() < 10 {
        k.push(point.clone());
    }
    vk["G1"]["Alpha"]["X"] = json!("9".repeat(257));

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gnark.json");
    write_json(&path, &vk);
    assert!(matches!(
        load_gnark_json_inputs(&path, None, None, Some("bn254")),
        Err(Error::PublicInputCountMismatch(_))
    ));

    vk["G1"]["K"].as_array_mut().unwrap().truncate(2);
    write_json(&path, &vk);
    assert!(matches!(
        load_gnark_json_inputs(&path, None, None, Some("bn254")),
        Err(Error::DecimalParse(_))
    ));
}

#[test]
fn compact_parser_rejects_empty_trailing_and_overlong_encodings() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir.path().join("artifact.json");

    for (curve, vk) in [
        ("bn254", compressed_hex(&VerifyingKey::<Bn254>::default())),
        (
            "bls12381",
            compressed_hex(&VerifyingKey::<Bls12_381>::default()),
        ),
    ] {
        write_json(&artifact, &json!({ "curve": curve, "vk": vk }));
        assert!(matches!(
            parse_compact_artifact(&artifact, None),
            Err(Error::IcLengthMismatch(_))
        ));
    }

    let mut vk = VerifyingKey::<Bn254>::default();
    vk.gamma_abc_g1.push(Default::default());
    let vk = compressed_hex(&vk);
    write_json(&artifact, &json!({ "curve": "bn254", "vk": vk }));
    let (valid_vk, proof, public) = parse_compact_artifact(&artifact, None).unwrap();
    assert_eq!(valid_vk.n_public, 0);
    assert!(proof.is_none());
    assert!(public.is_empty());

    let mut proof = Vec::new();
    Proof::<Bn254>::default()
        .serialize_compressed(&mut proof)
        .unwrap();
    proof.push(0);
    write_json(
        &artifact,
        &json!({ "curve": "bn254", "vk": vk, "proof": hex::encode(proof) }),
    );
    assert!(matches!(
        parse_compact_artifact(&artifact, None),
        Err(Error::Serialization(message)) if message.contains("trailing bytes")
    ));

    write_json(
        &artifact,
        &json!({ "curve": "bn254", "vk": "aa".repeat(65 * 1024) }),
    );
    assert!(matches!(
        parse_compact_artifact(&artifact, None),
        Err(Error::HexParse(_))
    ));

    write_json(
        &artifact,
        &json!({
            "curve": "bn254",
            "vk": vk,
            "public_input": format!("0x{}", "a".repeat(129))
        }),
    );
    assert!(matches!(
        parse_compact_artifact(&artifact, None),
        Err(Error::DecimalParse(_))
    ));
}
