use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use ark_bn254::{Fq, G1Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::Field;
use export_sui_verifier_core::curves::create_adapter;
use export_sui_verifier_core::error::Error;
use export_sui_verifier_core::formats::{
    load_arkworks_bundle, load_arkworks_inputs, load_gnark_binary_inputs, load_gnark_json_inputs,
    load_snarkjs_json_inputs, load_snarkjs_json_inputs_with_optional_proof,
    load_sp1_groth16_inputs,
};
use export_sui_verifier_core::model::Groth16G1Point;
use export_sui_verifier_core::movegen::{
    generate_move_package, proof_data_snippet, GenerateMovePackageOptions, MovegenMode,
};
use export_sui_verifier_core::parser::arkworks;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn temp_output_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("export_sui_verifier_{name}_{}", std::process::id()));
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    dir
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn sui_move_test(package_dir: &Path) {
    static SUI_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = SUI_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    let output = Command::new("sui")
        .args(["move", "test"])
        .current_dir(package_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "sui move test failed for {}\nstdout:\n{}\nstderr:\n{}",
        package_dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn sui_move_build(package_dir: &Path) {
    let output = Command::new("sui")
        .args(["move", "build"])
        .current_dir(package_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "sui move build failed for {}\nstdout:\n{}\nstderr:\n{}",
        package_dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn generated_move_uses_move_2024_module_syntax() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();

    let out_dir = temp_output_dir("move_2024_module_syntax");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "move_2024_syntax_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    let verifier = normalize_newlines(
        &fs::read_to_string(out_dir.join("sources").join("verifier.move")).unwrap(),
    );
    let tests = normalize_newlines(
        &fs::read_to_string(out_dir.join("tests").join("verifier_tests.move")).unwrap(),
    );

    assert!(verifier.starts_with("module move_2024_syntax_verifier::verifier;\n"));
    assert!(!verifier.starts_with("module move_2024_syntax_verifier::verifier {\n"));
    assert!(tests.contains("\nmodule move_2024_syntax_verifier::verifier_tests;\n"));
    assert!(!tests.contains("\nmodule move_2024_syntax_verifier::verifier_tests {\n"));
}

#[test]
fn generated_verifier_only_accepts_module_bound_prepared_keys() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();
    let out_dir = temp_output_dir("bound_prepared_key");

    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "bound_prepared_key_verifier",
            module_name: "verifier",
            mode: MovegenMode::Library,
            force: true,
        },
    )
    .unwrap();

    let source = fs::read_to_string(out_dir.join("sources/verifier.move")).unwrap();
    assert!(source.contains("public struct BoundPreparedVerifyingKey"));
    assert!(source.contains("public fun verify_with_bound_prepared"));
    assert!(!source.contains("public fun verify_with_prepared("));
    assert!(!source.contains(
        "public fun verify_with_bound_prepared(\n    prepared_verifying_key: &groth16::PreparedVerifyingKey"
    ));

    fs::write(
        out_dir.join("sources/untrusted_consumer.move"),
        r#"module bound_prepared_key_verifier::untrusted_consumer;

use bound_prepared_key_verifier::verifier;
use sui::groth16::PreparedVerifyingKey;

fun forge(inner: PreparedVerifyingKey): verifier::BoundPreparedVerifyingKey {
    verifier::BoundPreparedVerifyingKey { inner }
}

fun pass_raw(
    prepared: &PreparedVerifyingKey,
    proof: vector<u8>,
    public_inputs: vector<u8>,
): bool {
    verifier::verify_with_bound_prepared(prepared, proof, public_inputs)
}
"#,
    )
    .unwrap();

    let output = Command::new("sui")
        .args(["move", "build"])
        .current_dir(&out_dir)
        .output()
        .unwrap();
    let diagnostics = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "forged bound key compiled");
    assert!(diagnostics.contains("BoundPreparedVerifyingKey"));
    assert!(diagnostics.contains("PreparedVerifyingKey"));
}

#[test]
fn force_generation_validates_before_replacing_existing_output() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let mut inputs = load_snarkjs_json_inputs_with_optional_proof(
        &artifact_dir.join("verification_key.json"),
        None,
        None,
        Some("bn254"),
    )
    .unwrap();
    inputs.verifying_key.vk_alpha_1 = Groth16G1Point {
        x: "1".to_string(),
        y: "1".to_string(),
        z: "1".to_string(),
    };
    let out_dir = temp_output_dir("validate_before_replace");
    fs::create_dir_all(&out_dir).unwrap();
    let sentinel = out_dir.join("keep.txt");
    fs::write(&sentinel, "existing output").unwrap();

    let result = generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "validate_before_replace",
            module_name: "verifier",
            mode: MovegenMode::Library,
            force: true,
        },
    );

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(sentinel).unwrap(), "existing output");
}

#[test]
fn canonical_vk_fingerprint_is_format_and_projective_invariant() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();
    let arkworks =
        load_arkworks_bundle(&artifact_dir.join("groth16_artifacts.json"), Some("bn254")).unwrap();
    let mut projective = inputs.clone();
    let alpha = &mut projective.verifying_key.vk_alpha_1;
    let z = Fq::from(2u64);
    alpha.x = (Fq::from_str(&alpha.x).unwrap() * z.square()).to_string();
    alpha.y = (Fq::from_str(&alpha.y).unwrap() * z.square() * z).to_string();
    alpha.z = z.to_string();

    let original_dir = temp_output_dir("fingerprint_original");
    let arkworks_dir = temp_output_dir("fingerprint_arkworks");
    let projective_dir = temp_output_dir("fingerprint_projective");
    for (out_dir, candidate) in [
        (&original_dir, &inputs),
        (&arkworks_dir, &arkworks),
        (&projective_dir, &projective),
    ] {
        generate_move_package(
            out_dir,
            create_adapter("bn254").unwrap().as_ref(),
            candidate,
            &GenerateMovePackageOptions {
                package_name: "fingerprint_verifier",
                module_name: "verifier",
                mode: MovegenMode::Library,
                force: true,
            },
        )
        .unwrap();
    }

    let read_fingerprint = |dir: &Path| {
        let manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("verifier-manifest.json")).unwrap())
                .unwrap();
        manifest["vk_sha256"].as_str().unwrap().to_string()
    };
    let fingerprint = read_fingerprint(&original_dir);
    assert_eq!(fingerprint, read_fingerprint(&arkworks_dir));
    assert_eq!(fingerprint, read_fingerprint(&projective_dir));
    let source = fs::read_to_string(original_dir.join("sources/verifier.move")).unwrap();
    assert!(source.contains(&format!("x\"{fingerprint}\"")));

    let mut different = inputs;
    let replacement = G1Affine::generator().mul_bigint([2u64]).into_affine();
    different.verifying_key.vk_alpha_1 = Groth16G1Point {
        x: replacement.x.to_string(),
        y: replacement.y.to_string(),
        z: "1".to_string(),
    };
    let different_dir = temp_output_dir("fingerprint_different");
    generate_move_package(
        &different_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &different,
        &GenerateMovePackageOptions {
            package_name: "fingerprint_verifier",
            module_name: "verifier",
            mode: MovegenMode::Library,
            force: true,
        },
    )
    .unwrap();
    assert_ne!(fingerprint, read_fingerprint(&different_dir));
}

#[test]
fn generated_readme_documents_entry_mode_api() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs_with_optional_proof(
        &artifact_dir.join("verification_key.json"),
        None,
        None,
        Some("bn254"),
    )
    .unwrap();

    let entry_out = temp_output_dir("readme_entry_mode");
    generate_move_package(
        &entry_out,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "readme_entry_mode",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    let entry_readme = fs::read_to_string(entry_out.join("README.md")).unwrap();
    assert!(entry_readme.contains("verify_entry(proof_bytes, public_inputs_bytes)"));

    let library_out = temp_output_dir("readme_library_mode");
    generate_move_package(
        &library_out,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "readme_library_mode",
            module_name: "verifier",
            mode: MovegenMode::Library,
            force: true,
        },
    )
    .unwrap();

    let library_readme = fs::read_to_string(library_out.join("README.md")).unwrap();
    assert!(!library_readme.contains("verify_entry(proof_bytes, public_inputs_bytes)"));
}

#[test]
fn inputs_with_proof_require_exact_public_input_count() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();

    let err = export_sui_verifier_core::model::Groth16VerifierInputs::from_parts(
        inputs.curve,
        inputs.verifying_key,
        inputs.proof,
        Vec::new(),
        inputs.source_format,
    )
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("verification key expects 1 public inputs, got 0"));
}

#[test]
fn snarkjs_bn254_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();

    let out_dir = temp_output_dir("snarkjs_bn254");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_snarkjs_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    let tests = fs::read_to_string(out_dir.join("tests").join("verifier_tests.move")).unwrap();
    assert!(tests.contains("fun reject_invalid_public_input()"));

    sui_move_test(&out_dir);
}

#[test]
fn snarkjs_bls12381_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bls12_381");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();

    let out_dir = temp_output_dir("snarkjs_bls12381");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_snarkjs_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn snarkjs_vk_only_generates_buildable_package_without_tests() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs_with_optional_proof(
        &artifact_dir.join("verification_key.json"),
        None,
        None,
        Some("bn254"),
    )
    .unwrap();
    assert!(!inputs.has_test_vectors());

    let out_dir = temp_output_dir("snarkjs_vk_only");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_snarkjs_vk_only",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    assert!(!out_dir.join("tests").exists());
    let verifier = fs::read_to_string(out_dir.join("sources/verifier.move")).unwrap();
    assert!(verifier.contains("const EXPECTED_PUBLIC_INPUTS_BYTES: u64 = 32;"));

    let proof_inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();
    let snippet =
        proof_data_snippet(create_adapter("bn254").unwrap().as_ref(), &proof_inputs).unwrap();
    fs::create_dir(out_dir.join("tests")).unwrap();
    fs::write(
        out_dir.join("tests/later_proof_test.move"),
        format!(
            r#"#[test_only]
module groth16_bn254_snarkjs_vk_only::later_proof_test;

use groth16_bn254_snarkjs_vk_only::verifier;

{}

#[test]
fun accepts_proof_supplied_after_vk_only_generation() {{
    assert!(verifier::verify(proof_bytes(), public_inputs_bytes()));
}}
"#,
            snippet.render_sui_test_functions()
        ),
    )
    .unwrap();
    sui_move_test(&out_dir);
}

#[test]
fn public_generation_apis_reject_curve_confused_adapters() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        None,
    )
    .unwrap();
    let wrong_adapter = create_adapter("bls12381").unwrap();
    let out_dir = temp_output_dir("curve_confused_adapter");

    let err = generate_move_package(
        &out_dir,
        wrong_adapter.as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "curve_confused_adapter",
            module_name: "verifier",
            mode: MovegenMode::Library,
            force: true,
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::CurveMismatch(_)));
    assert!(!out_dir.exists());

    let err = proof_data_snippet(wrong_adapter.as_ref(), &inputs).unwrap_err();
    assert!(matches!(err, Error::CurveMismatch(_)));
}

#[test]
fn snarkjs_bls12381_vk_only_generates_buildable_package_without_tests() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bls12_381");
    let inputs = load_snarkjs_json_inputs_with_optional_proof(
        &artifact_dir.join("verification_key.json"),
        None,
        None,
        Some("bls12381"),
    )
    .unwrap();
    assert!(!inputs.has_test_vectors());

    let out_dir = temp_output_dir("snarkjs_bls12381_vk_only");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_snarkjs_vk_only",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    assert!(!out_dir.join("tests").exists());
    sui_move_build(&out_dir);
}

#[test]
fn mul_circuit_bls12381_snarkjs_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("MulCircuit")
        .join("artifacts")
        .join("bls12_381");
    let inputs = load_snarkjs_json_inputs(
        &artifact_dir.join("verification_key.json"),
        &artifact_dir.join("proof.json"),
        Some(&artifact_dir.join("public.json")),
    )
    .unwrap();

    let out_dir = temp_output_dir("mul_circuit_bls12381");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "mul_circuit_bls12381_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn gnark_native_json_bn254_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bn254");
    let inputs = load_gnark_json_inputs(
        &artifact_dir.join("verification_key_gnark.json"),
        Some(&artifact_dir.join("proof_gnark.json")),
        Some(&artifact_dir.join("public.json")),
        Some("bn254"),
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::GnarkJson
    );

    let out_dir = temp_output_dir("gnark_json_bn254");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_gnark_json_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn gnark_native_json_bls12381_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bls12381");
    let inputs = load_gnark_json_inputs(
        &artifact_dir.join("verification_key_gnark.json"),
        Some(&artifact_dir.join("proof_gnark.json")),
        Some(&artifact_dir.join("public.json")),
        Some("bls12381"),
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::GnarkJson
    );

    let out_dir = temp_output_dir("gnark_json_bls12381");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_gnark_json_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn gnark_native_binary_bn254_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bn254");
    let inputs = load_gnark_binary_inputs(
        &artifact_dir.join("verification_key.bin"),
        Some(&artifact_dir.join("proof.bin")),
        Some(&artifact_dir.join("public.json")),
        "bn254",
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::GnarkBin
    );

    let out_dir = temp_output_dir("gnark_bin_bn254");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_gnark_bin_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn gnark_native_binary_bls12381_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bls12381");
    let inputs = load_gnark_binary_inputs(
        &artifact_dir.join("verification_key.bin"),
        Some(&artifact_dir.join("proof.bin")),
        Some(&artifact_dir.join("public.json")),
        "bls12381",
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::GnarkBin
    );

    let out_dir = temp_output_dir("gnark_bin_bls12381");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_gnark_bin_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn sp1_sui_fibonacci_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("sp1-sui")
        .join("fibonacci")
        .join("artifacts");
    let inputs = load_sp1_groth16_inputs(
        &artifact_dir.join("groth16_vk_v5.bin"),
        &artifact_dir.join("fibonacci_proof.bin"),
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::Sp1
    );

    let out_dir = temp_output_dir("sp1_sui_fibonacci");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "sp1_sui_fibonacci_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
#[ignore = "requires locally generated SP1 simple-sum artifacts; tracked Fibonacci v6 covers this path"]
fn sp1_sui_simple_sum_v6_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("sp1-sui")
        .join("simple-sum")
        .join("artifacts");
    let inputs = load_sp1_groth16_inputs(
        &artifact_dir.join("sp1_groth16_vk.bin"),
        &artifact_dir.join("simple_sum_proof.bin"),
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::Sp1
    );
    assert_eq!(inputs.verifying_key.n_public, 5);
    assert_eq!(inputs.public_inputs.len(), 5);

    let out_dir = temp_output_dir("sp1_sui_simple_sum_v6");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "sp1_sui_simple_sum_v6_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn sp1_sui_fibonacci_v6_inputs_generate_sui_package() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("sp1-sui")
        .join("fibonacci")
        .join("artifacts");
    let inputs = load_sp1_groth16_inputs(
        &artifact_dir.join("sp1_groth16_vk.bin"),
        &artifact_dir.join("fibonacci_sp1_6_proof.bin"),
    )
    .unwrap();

    assert_eq!(
        inputs.source_format,
        export_sui_verifier_core::model::SourceFormat::Sp1
    );
    assert_eq!(inputs.verifying_key.n_public, 5);
    assert_eq!(inputs.public_inputs.len(), 5);

    let out_dir = temp_output_dir("sp1_sui_fibonacci_v6");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "sp1_sui_fibonacci_v6_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn arkworks_bundle_inputs_generate_sui_package_without_snarkjs_parser() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254")
        .join("groth16_artifacts.json");
    let inputs = load_arkworks_bundle(&bundle, Some("bn254")).unwrap();

    let out_dir = temp_output_dir("arkworks_bn254_bundle");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_arkworks_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn arkworks_bundle_rejects_trailing_bytes_in_vk() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254")
        .join("groth16_artifacts.json");
    let bundle_json = fs::read_to_string(&bundle).unwrap();
    let mut bundle_value: serde_json::Value = serde_json::from_str(&bundle_json).unwrap();
    let vk = bundle_value
        .get("vk")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    bundle_value["vk"] = serde_json::Value::String(format!("{vk}00"));

    let temp = temp_output_dir("arkworks_bundle_trailing_vk");
    fs::create_dir_all(&temp).unwrap();
    let bundle_path = temp.join("groth16_artifacts_bad_vk.json");
    fs::write(
        &bundle_path,
        serde_json::to_string_pretty(&bundle_value).unwrap(),
    )
    .unwrap();

    let err = load_arkworks_bundle(&bundle_path, Some("bn254")).unwrap_err();
    assert!(err.to_string().contains("trailing bytes"));
}

#[test]
fn arkworks_bundle_rejects_trailing_bytes_in_proof() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254")
        .join("groth16_artifacts.json");
    let bundle_json = fs::read_to_string(&bundle).unwrap();
    let mut bundle_value: serde_json::Value = serde_json::from_str(&bundle_json).unwrap();
    let proof = bundle_value
        .get("proof")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    bundle_value["proof"] = serde_json::Value::String(format!("{proof}00"));

    let temp = temp_output_dir("arkworks_bundle_trailing_proof");
    fs::create_dir_all(&temp).unwrap();
    let bundle_path = temp.join("groth16_artifacts_bad_proof.json");
    fs::write(
        &bundle_path,
        serde_json::to_string_pretty(&bundle_value).unwrap(),
    )
    .unwrap();

    let err = load_arkworks_bundle(&bundle_path, Some("bn254")).unwrap_err();
    assert!(err.to_string().contains("trailing bytes"));
}

#[test]
fn arkworks_bls12381_bundle_inputs_generate_sui_package_without_snarkjs_parser() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bls12_381")
        .join("groth16_artifacts.json");
    let inputs = load_arkworks_bundle(&bundle, Some("bls12381")).unwrap();

    let out_dir = temp_output_dir("arkworks_bls12381_bundle");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_arkworks_verifier",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    sui_move_test(&out_dir);
}

#[test]
fn arkworks_vk_only_generates_buildable_package_without_tests() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254")
        .join("groth16_artifacts.json");
    let bundle_json = fs::read_to_string(&bundle).unwrap();
    let bundle_value: serde_json::Value = serde_json::from_str(&bundle_json).unwrap();
    let vk_only = serde_json::json!({
        "curve": "bn254",
        "verification_key": bundle_value.get("vk").unwrap(),
    });

    let temp = temp_output_dir("arkworks_vk_only_input");
    fs::create_dir_all(&temp).unwrap();
    let vk_path = temp.join("arkworks_verification_key.json");
    fs::write(&vk_path, serde_json::to_string_pretty(&vk_only).unwrap()).unwrap();

    let inputs = load_arkworks_inputs(&vk_path, None, None, Some("bn254")).unwrap();
    assert!(!inputs.has_test_vectors());

    let out_dir = temp.join("generated");
    generate_move_package(
        &out_dir,
        create_adapter("bn254").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bn254_arkworks_vk_only",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    assert!(!out_dir.join("tests").exists());
    sui_move_build(&out_dir);
}

#[test]
fn arkworks_bls12381_vk_only_generates_buildable_package_without_tests() {
    let bundle = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bls12_381")
        .join("groth16_artifacts.json");
    let bundle_json = fs::read_to_string(&bundle).unwrap();
    let bundle_value: serde_json::Value = serde_json::from_str(&bundle_json).unwrap();
    let vk_only = serde_json::json!({
        "curve": "bls12381",
        "verification_key": bundle_value.get("vk").unwrap(),
    });

    let temp = temp_output_dir("arkworks_bls12381_vk_only_input");
    fs::create_dir_all(&temp).unwrap();
    let vk_path = temp.join("arkworks_verification_key.json");
    fs::write(&vk_path, serde_json::to_string_pretty(&vk_only).unwrap()).unwrap();

    let inputs = load_arkworks_inputs(&vk_path, None, None, Some("bls12381")).unwrap();
    assert!(!inputs.has_test_vectors());

    let out_dir = temp.join("generated");
    generate_move_package(
        &out_dir,
        create_adapter("bls12381").unwrap().as_ref(),
        &inputs,
        &GenerateMovePackageOptions {
            package_name: "groth16_bls12381_arkworks_vk_only",
            module_name: "verifier",
            mode: MovegenMode::Entry,
            force: true,
        },
    )
    .unwrap();

    assert!(!out_dir.join("tests").exists());
    sui_move_build(&out_dir);
}

#[test]
fn arkworks_parser_rejects_missing_curve_without_hint() {
    let temp = temp_output_dir("arkworks_missing_curve");
    fs::create_dir_all(&temp).unwrap();
    let vk_path = temp.join("vk.json");
    fs::write(&vk_path, serde_json::json!({"vk": "00"}).to_string()).unwrap();

    let err = arkworks::load_arkworks_inputs(&vk_path, None, None, None).unwrap_err();
    assert!(err.to_string().contains("requires curve metadata"));
}
