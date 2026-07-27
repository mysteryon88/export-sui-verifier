mod context;
mod render;

use crate::bytes::move_hex_literal;
use crate::curves::{CurveAdapter, CurveId};
use crate::error::{Error, Result};
use crate::model::{CurveKind, Groth16VerifierInputs, SourceFormat};
pub use context::{MovegenMode, MovegenTemplateInput};
use handlebars::Handlebars;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, create_dir_all, write};
use std::path::{Component, Path};
use tempfile::{Builder as TempDirBuilder, TempDir};

#[derive(Debug, Clone)]
pub struct GenerateMovePackageOptions<'a> {
    pub package_name: &'a str,
    pub module_name: &'a str,
    pub mode: MovegenMode,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofDataSnippet {
    pub proof_bytes: String,
    pub public_inputs_bytes: String,
}

impl ProofDataSnippet {
    pub fn render_sui_test_functions(&self) -> String {
        format!(
            r#"fun proof_bytes(): vector<u8> {{
    {}
}}

fun public_inputs_bytes(): vector<u8> {{
    {}
}}"#,
            self.proof_bytes, self.public_inputs_bytes
        )
    }
}

pub fn proof_data_snippet(
    adapter: &dyn CurveAdapter,
    inputs: &Groth16VerifierInputs,
) -> Result<ProofDataSnippet> {
    ensure_adapter_matches_inputs(adapter, inputs)?;
    let proof = inputs.proof.as_ref().ok_or_else(|| {
        Error::MissingInput("proof-data requires proof input; VK-only inputs have no proof".into())
    })?;

    Ok(ProofDataSnippet {
        proof_bytes: move_hex_literal(&adapter.serialize_proof(proof)?),
        public_inputs_bytes: move_hex_literal(&serialize_public_inputs(adapter, inputs)?),
    })
}

pub fn generate_move_package(
    out_dir: &Path,
    adapter: &dyn CurveAdapter,
    inputs: &Groth16VerifierInputs,
    options: &GenerateMovePackageOptions<'_>,
) -> Result<()> {
    ensure_adapter_matches_inputs(adapter, inputs)?;
    if options.force {
        validate_safe_force_output_dir(out_dir)?;
    }

    if out_dir.exists() && !options.force {
        return Err(Error::OutputExists(out_dir.to_path_buf()));
    }

    inputs.validate()?;
    let mut reg = Handlebars::new();
    register_templates(&mut reg)?;

    let raw_verifying_key = adapter.serialize_verifying_key(&inputs.verifying_key)?;
    let raw_proof = match inputs.proof.as_ref() {
        Some(proof) => adapter.serialize_proof(proof)?,
        None => Vec::new(),
    };
    let raw_public_inputs = serialize_public_inputs(adapter, inputs)?;
    let mut noncanonical_public_inputs = raw_public_inputs.clone();
    let mut noncanonical_public_inputs_plus_one = raw_public_inputs.clone();
    if !noncanonical_public_inputs.is_empty() {
        let modulus = adapter.scalar_modulus_le();
        noncanonical_public_inputs[..32].copy_from_slice(&modulus);

        let mut modulus_plus_one = modulus;
        let mut carry = 1u16;
        for byte in &mut modulus_plus_one {
            let sum = u16::from(*byte) + carry;
            *byte = sum as u8;
            carry = sum >> 8;
        }
        if carry != 0 {
            return Err(Error::Serialization(
                "scalar modulus plus one does not fit in 32 bytes".to_string(),
            ));
        }
        noncanonical_public_inputs_plus_one[..32].copy_from_slice(&modulus_plus_one);
    }
    let fingerprint = vk_fingerprint(inputs, &raw_verifying_key);

    let expected_public_inputs_bytes = inputs
        .verifying_key
        .n_public
        .checked_mul(32)
        .ok_or_else(|| Error::Serialization("public input byte length overflow".to_string()))?;
    let input = MovegenTemplateInput {
        package_name: options.package_name.to_string(),
        module_name: options.module_name.to_string(),
        curve_function: adapter.sui_curve_function().to_string(),
        verifying_key_bytes: move_hex_literal(&raw_verifying_key),
        proof_bytes: move_hex_literal(&raw_proof),
        public_inputs_bytes: move_hex_literal(&raw_public_inputs),
        noncanonical_public_inputs_bytes: move_hex_literal(&noncanonical_public_inputs),
        noncanonical_public_inputs_plus_one_bytes: move_hex_literal(
            &noncanonical_public_inputs_plus_one,
        ),
        has_public_inputs: inputs.verifying_key.n_public != 0,
        expected_proof_bytes: expected_proof_bytes(inputs),
        expected_public_inputs_bytes,
        vk_fingerprint_bytes: move_hex_literal(&fingerprint),
        include_test_vectors: inputs.has_test_vectors(),
        include_entry: options.mode.include_entry(),
    };

    let move_toml = reg
        .render("move_toml", &input)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    let verifier_source = reg
        .render("verifier", &input)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    let move_tests = input
        .include_test_vectors
        .then(|| {
            reg.render("move_tests", &input)
                .map_err(|e| Error::TemplateRender(e.to_string()))
        })
        .transpose()?;
    let generated_readme = render::readme_content(
        options.package_name,
        options.module_name,
        &input.curve_function,
        input.include_test_vectors,
        input.include_entry,
    );
    let manifest = render_manifest(inputs, &input, &fingerprint)?;

    let staging = create_staging_directory(out_dir)?;
    let staged_out = staging.path().join("output");
    create_dir_all(staged_out.join("sources")).map_err(|e| Error::Io {
        source: e,
        context: format!(
            "create staged sources dir {}",
            staged_out.join("sources").display()
        ),
    })?;
    write_generated(staged_out.join("Move.toml"), move_toml, "Move.toml")?;
    write_generated(
        staged_out.join("sources").join("verifier.move"),
        verifier_source,
        "verifier.move",
    )?;
    if let Some(tests) = move_tests {
        create_dir_all(staged_out.join("tests")).map_err(|e| Error::Io {
            source: e,
            context: format!(
                "create staged tests dir {}",
                staged_out.join("tests").display()
            ),
        })?;
        write_generated(
            staged_out.join("tests").join("verifier_tests.move"),
            tests,
            "verifier_tests.move",
        )?;
    }
    write_generated(staged_out.join("README.md"), generated_readme, "README.md")?;
    write_generated(
        staged_out.join("verifier-manifest.json"),
        format!("{manifest}\n"),
        "verifier-manifest.json",
    )?;
    publish_staged_directory(staging, &staged_out, out_dir)?;

    Ok(())
}

fn create_staging_directory(out_dir: &Path) -> Result<TempDir> {
    let parent = out_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    create_dir_all(parent).map_err(|e| Error::Io {
        source: e,
        context: format!("create output parent {}", parent.display()),
    })?;
    TempDirBuilder::new()
        .prefix(".export-sui-verifier-")
        .tempdir_in(parent)
        .map_err(|e| Error::Io {
            source: e,
            context: format!("create staging directory in {}", parent.display()),
        })
}

fn publish_staged_directory(staging: TempDir, staged_out: &Path, out_dir: &Path) -> Result<()> {
    let backup = staging.path().join("previous-output");
    let had_existing = out_dir.exists();
    if had_existing {
        fs::rename(out_dir, &backup).map_err(|e| Error::Io {
            source: e,
            context: format!("stage existing output {}", out_dir.display()),
        })?;
    }

    if let Err(publish_error) = fs::rename(staged_out, out_dir) {
        if had_existing {
            return restore_previous_output_or_preserve(staging, &backup, out_dir, publish_error);
        }
        return Err(Error::Io {
            source: publish_error,
            context: format!("publish generated output {}", out_dir.display()),
        });
    }
    Ok(())
}

fn restore_previous_output_or_preserve(
    staging: TempDir,
    backup: &Path,
    out_dir: &Path,
    publish_error: std::io::Error,
) -> Result<()> {
    match fs::rename(backup, out_dir) {
        Ok(()) => Err(Error::Io {
            source: publish_error,
            context: format!("publish generated output {}", out_dir.display()),
        }),
        Err(rollback_error) => {
            let preserved = staging.keep().join("previous-output");
            Err(Error::Io {
                source: rollback_error,
                context: format!(
                    "restore {} after publish failed ({publish_error}); previous output preserved at {}",
                    out_dir.display(),
                    preserved.display()
                ),
            })
        }
    }
}

fn ensure_adapter_matches_inputs(
    adapter: &dyn CurveAdapter,
    inputs: &Groth16VerifierInputs,
) -> Result<()> {
    let matches = matches!(
        (adapter.id(), inputs.curve),
        (CurveId::Bn254, CurveKind::Bn254) | (CurveId::Bls12381, CurveKind::Bls12_381)
    );
    if !matches {
        return Err(Error::CurveMismatch(format!(
            "adapter {:?} does not match input curve {}",
            adapter.id(),
            inputs.curve.canonical_name()
        )));
    }
    Ok(())
}

fn write_generated(path: impl AsRef<Path>, contents: String, label: &str) -> Result<()> {
    write(path, contents).map_err(|e| Error::Io {
        source: e,
        context: format!("write {label}"),
    })
}

fn expected_proof_bytes(inputs: &Groth16VerifierInputs) -> usize {
    match inputs.curve {
        crate::model::CurveKind::Bn254 => 128,
        crate::model::CurveKind::Bls12_381 => 192,
    }
}

fn vk_fingerprint(inputs: &Groth16VerifierInputs, verifying_key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"export-sui-verifier:groth16-vk:v1\0");
    hash_component(&mut hasher, inputs.curve.canonical_name().as_bytes());
    hash_component(
        &mut hasher,
        &(inputs.verifying_key.n_public as u64).to_be_bytes(),
    );
    hash_component(&mut hasher, verifying_key);
    hasher.finalize().into()
}

fn hash_component(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn source_format_name(source_format: SourceFormat) -> &'static str {
    match source_format {
        SourceFormat::SnarkjsJson => "snarkjs-json",
        SourceFormat::Arkworks => "arkworks",
        SourceFormat::GnarkJson => "gnark-json",
        SourceFormat::GnarkBin => "gnark-binary",
        SourceFormat::Sp1 => "sp1-groth16",
    }
}

fn render_manifest(
    inputs: &Groth16VerifierInputs,
    template: &MovegenTemplateInput,
    fingerprint: &[u8; 32],
) -> Result<String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": "groth16-verifier-manifest-v1",
        "generator": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "protocol": "groth16",
        "curve": inputs.curve.canonical_name(),
        "public_inputs": inputs.verifying_key.n_public,
        "vk_sha256": hex::encode(fingerprint),
        "circuit_sha256": serde_json::Value::Null,
        "source_format": source_format_name(inputs.source_format),
        "serialization_format": "sui-arkworks-canonical-compressed-v1",
        "package": template.package_name,
        "module": template.module_name,
        "dependencies": {
            "sui-framework": "provided-by-sui-cli",
            "arkworks": "0.6",
        },
    }))
    .map_err(|e| Error::TemplateRender(format!("failed to render verifier manifest: {e}")))
}

fn serialize_public_inputs(
    adapter: &dyn CurveAdapter,
    inputs: &Groth16VerifierInputs,
) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(inputs.public_inputs.len() * 32);
    for value in &inputs.public_inputs {
        out.extend_from_slice(&adapter.serialize_fr_public_input(value)?);
    }
    Ok(out)
}

fn validate_safe_force_output_dir(out_dir: &Path) -> Result<()> {
    if out_dir
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(Error::UnsafeOutputDirectory(out_dir.to_path_buf()));
    }

    if !out_dir.exists() {
        return Ok(());
    }

    let target = out_dir.canonicalize().map_err(|e| Error::Io {
        source: e,
        context: format!("canonicalize output dir {}", out_dir.display()),
    })?;
    if target.parent().is_none() {
        return Err(Error::UnsafeOutputDirectory(target));
    }

    let cwd = env::current_dir().map_err(|e| Error::Io {
        source: e,
        context: "get current working directory".to_string(),
    })?;
    let cwd = cwd.canonicalize().map_err(|e| Error::Io {
        source: e,
        context: format!("canonicalize current working directory {}", cwd.display()),
    })?;
    if target == cwd || cwd.starts_with(&target) {
        return Err(Error::UnsafeOutputDirectory(target));
    }

    Ok(())
}

fn register_templates(handlebars: &mut Handlebars) -> Result<()> {
    let move_toml = include_str!("../../templates/Move.toml.hbs");
    let verifier = include_str!("../../templates/verifier.move.hbs");
    let tests = include_str!("../../templates/tests.move.hbs");

    handlebars
        .register_template_string("move_toml", move_toml)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    handlebars
        .register_template_string("verifier", verifier)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    handlebars
        .register_template_string("move_tests", tests)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_staging_directory, publish_staged_directory};
    use std::fs;

    #[test]
    fn failed_publish_restores_existing_output() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("generated");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("keep.txt"), "existing output").unwrap();
        let staging = create_staging_directory(&out).unwrap();
        let missing_staged_output = staging.path().join("missing");

        assert!(publish_staged_directory(staging, &missing_staged_output, &out).is_err());
        assert_eq!(
            fs::read_to_string(out.join("keep.txt")).unwrap(),
            "existing output"
        );
    }

    #[test]
    fn failed_rollback_preserves_backup_on_disk() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("generated");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("occupied.txt"), "concurrent output").unwrap();
        let staging = create_staging_directory(&out).unwrap();
        let staging_path = staging.path().to_path_buf();
        let backup = staging.path().join("previous-output");
        fs::create_dir(&backup).unwrap();
        fs::write(backup.join("keep.txt"), "existing output").unwrap();

        let err = super::restore_previous_output_or_preserve(
            staging,
            &backup,
            &out,
            std::io::Error::other("publish failed"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("previous output preserved"));
        assert_eq!(
            fs::read_to_string(staging_path.join("previous-output/keep.txt")).unwrap(),
            "existing output"
        );
        fs::remove_dir_all(staging_path).unwrap();
    }
}
