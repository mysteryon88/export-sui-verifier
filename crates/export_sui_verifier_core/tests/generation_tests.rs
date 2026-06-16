use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use export_sui_verifier_core::curves::create_adapter;
use export_sui_verifier_core::formats::{
    load_arkworks_bundle, load_arkworks_inputs, load_snarkjs_json_inputs,
    load_snarkjs_json_inputs_with_optional_proof,
};
use export_sui_verifier_core::movegen::{
    generate_move_package, GenerateMovePackageOptions, MovegenMode,
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

    let verifier = fs::read_to_string(out_dir.join("sources").join("verifier.move")).unwrap();
    let tests = fs::read_to_string(out_dir.join("tests").join("verifier_tests.move")).unwrap();

    assert!(verifier.starts_with("module move_2024_syntax_verifier::verifier;\n"));
    assert!(!verifier.starts_with("module move_2024_syntax_verifier::verifier {\n"));
    assert!(tests.contains("\nmodule move_2024_syntax_verifier::verifier_tests;\n"));
    assert!(!tests.contains("\nmodule move_2024_syntax_verifier::verifier_tests {\n"));
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
    sui_move_build(&out_dir);
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
