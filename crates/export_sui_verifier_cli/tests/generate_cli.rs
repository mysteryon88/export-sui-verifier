use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn temp_output_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "export_sui_verifier_cli_{name}_{}",
        std::process::id()
    ));
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    dir
}

#[test]
fn help_omits_format_curve_and_output_flags() {
    let assert = Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--out"));

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.contains("--format"));
    assert!(!stdout.contains("--curve"));
    assert!(!stdout.contains("--output"));
}

#[test]
fn snarkjs_vk_only_command_infers_curve_from_metadata() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("snarkjs_auto_curve_vk_only").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bn254_snarkjs_auto_curve",
            "--module-name",
            "verifier",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("sources").join("verifier.move").exists());
    assert!(!output.join("tests").exists());
}

#[test]
fn generation_uses_simple_defaults_when_names_are_omitted() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("default_names").join("my-verifier");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--out"])
        .arg(&output)
        .args(["--force"])
        .assert()
        .success();

    let move_toml = fs::read_to_string(output.join("Move.toml")).unwrap();
    let verifier = fs::read_to_string(output.join("sources").join("verifier.move")).unwrap();
    assert!(move_toml.contains("name = \"my_verifier\""));
    assert!(verifier.contains("module my_verifier::verifier"));
}

#[test]
fn generation_accepts_test_mode_from_cli() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("test_mode").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--out"])
        .arg(&output)
        .args(["--mode", "test"])
        .assert()
        .success();

    let verifier = fs::read_to_string(output.join("sources").join("verifier.move")).unwrap();
    assert!(verifier.contains("entry fun verify_entry"));
}

#[test]
fn groth16_subcommand_is_no_longer_supported() {
    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["groth16", "--help"])
        .assert()
        .failure();
}

#[test]
fn format_curve_and_output_flags_are_rejected() {
    for flag in ["--format", "--curve", "--output"] {
        Command::cargo_bin("export-sui-verifier")
            .unwrap()
            .args([flag, "auto"])
            .assert()
            .failure();
    }
}

#[test]
fn proof_data_infers_curve_from_metadata() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["proof-data", "--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof.json"))
        .assert()
        .success()
        .stdout(predicates::str::contains("fun proof_bytes(): vector<u8>"));
}

#[test]
fn arkworks_vk_only_command_generates_package() {
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

    let temp = temp_output_dir("arkworks_vk_only");
    fs::create_dir_all(&temp).unwrap();
    let vk_path = temp.join("arkworks_verification_key.json");
    let output = temp.join("generated");
    fs::write(&vk_path, serde_json::to_string_pretty(&vk_only).unwrap()).unwrap();

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(&vk_path)
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bn254_arkworks_verifier",
            "--module-name",
            "verifier",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("sources").join("verifier.move").exists());
    assert!(!output.join("tests").exists());
}

#[test]
fn gnark_json_command_is_auto_detected_without_format_or_curve() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("gnark_json_autodetect").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key_gnark.json"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof_gnark.json"))
        .args(["--public"])
        .arg(artifact_dir.join("public.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bn254_gnark_json_auto",
            "--module-name",
            "verifier",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("tests").join("verifier_tests.move").exists());
}

#[test]
fn gnark_binary_command_is_auto_detected_without_format_or_curve() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("gnark-native")
        .join("cubic")
        .join("artifacts")
        .join("bls12381");
    let output = temp_output_dir("gnark_binary_autodetect").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.bin"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof.bin"))
        .args(["--public"])
        .arg(artifact_dir.join("public.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bls12381_gnark_bin_auto",
            "--module-name",
            "verifier",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("tests").join("verifier_tests.move").exists());
}

#[test]
fn snarkjs_command_can_run_sui_test() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("MulCircuit")
        .join("artifacts")
        .join("bls12_381");
    let output = temp_output_dir("snarkjs_run_sui_test").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof.json"))
        .args(["--public"])
        .arg(artifact_dir.join("public.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "mul_circuit_bls12381_verifier",
            "--module-name",
            "verifier",
            "--run-sui-test",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("tests").join("verifier_tests.move").exists());
}

#[test]
fn snarkjs_vk_only_command_generates_package_without_tests() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("snarkjs_vk_only").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bn254_snarkjs_vk_only",
            "--module-name",
            "verifier",
        ])
        .assert()
        .success();

    assert!(output.join("Move.toml").exists());
    assert!(output.join("sources").join("verifier.move").exists());
    assert!(!output.join("tests").exists());
}

#[test]
fn proof_data_prints_snippets_matching_generated_sui_tests() {
    let artifact_dir = repo_root()
        .join("examples")
        .join("ark-mimc")
        .join("artifacts")
        .join("bn254");
    let output = temp_output_dir("proof_data_matches_tests").join("generated");

    Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof.json"))
        .args(["--out"])
        .arg(&output)
        .args([
            "--package-name",
            "groth16_bn254_snarkjs_proof_data",
            "--module-name",
            "verifier",
            "--force",
        ])
        .assert()
        .success();

    let generated_tests =
        fs::read_to_string(output.join("tests").join("verifier_tests.move")).unwrap();

    let assert = Command::cargo_bin("export-sui-verifier")
        .unwrap()
        .args(["proof-data", "--vk"])
        .arg(artifact_dir.join("verification_key.json"))
        .args(["--proof"])
        .arg(artifact_dir.join("proof.json"))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("fun proof_bytes(): vector<u8>"));
    assert!(stdout.contains("fun public_inputs_bytes(): vector<u8>"));
    let generated_tests = generated_tests.replace("\r\n", "\n");
    let stdout = stdout.replace("\r\n", "\n");
    assert!(generated_tests.contains(stdout.trim()));
}
