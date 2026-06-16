mod context;
mod render;

use crate::bytes::move_hex_literal;
use crate::curves::CurveAdapter;
use crate::error::{Error, Result};
use crate::model::Groth16VerifierInputs;
pub use context::{MovegenMode, MovegenTemplateInput};
use handlebars::Handlebars;
use std::env;
use std::fs::{self, create_dir_all, write};
use std::path::{Component, Path};

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
    if options.force {
        validate_safe_force_output_dir(out_dir)?;
    }

    if out_dir.exists() && !options.force {
        return Err(Error::OutputExists(out_dir.to_path_buf()));
    }

    if out_dir.exists() {
        fs::remove_dir_all(out_dir).map_err(|e| Error::Io {
            source: e,
            context: format!("failed to clear existing output dir {}", out_dir.display()),
        })?;
    }

    create_dir_all(out_dir).map_err(|e| Error::Io {
        source: e,
        context: format!("create output dir {}", out_dir.display()),
    })?;

    create_dir_all(out_dir.join("sources")).map_err(|e| Error::Io {
        source: e,
        context: format!("create sources dir {}", out_dir.join("sources").display()),
    })?;
    let mut reg = Handlebars::new();
    register_templates(&mut reg)?;

    let verifying_key_bytes =
        move_hex_literal(&adapter.serialize_verifying_key(&inputs.verifying_key)?);
    let proof_bytes = match inputs.proof.as_ref() {
        Some(proof) => move_hex_literal(&adapter.serialize_proof(proof)?),
        None => "x\"\"".to_string(),
    };
    let public_inputs_bytes = move_hex_literal(&serialize_public_inputs(adapter, inputs)?);

    let input = MovegenTemplateInput {
        package_name: options.package_name.to_string(),
        module_name: options.module_name.to_string(),
        curve_function: adapter.sui_curve_function().to_string(),
        verifying_key_bytes,
        proof_bytes,
        public_inputs_bytes,
        include_test_vectors: inputs.has_test_vectors(),
        include_entry: options.mode.include_entry(),
    };

    let move_toml = reg
        .render("move_toml", &input)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    fs::write(out_dir.join("Move.toml"), move_toml).map_err(|e| Error::Io {
        source: e,
        context: "write Move.toml".to_string(),
    })?;

    let verifier_source = reg
        .render("verifier", &input)
        .map_err(|e| Error::TemplateRender(e.to_string()))?;
    fs::write(
        out_dir.join("sources").join("verifier.move"),
        verifier_source,
    )
    .map_err(|e| Error::Io {
        source: e,
        context: "write verifier.move".to_string(),
    })?;

    if input.include_test_vectors {
        create_dir_all(out_dir.join("tests")).map_err(|e| Error::Io {
            source: e,
            context: format!("create tests dir {}", out_dir.join("tests").display()),
        })?;
        let tests = reg
            .render("move_tests", &input)
            .map_err(|e| Error::TemplateRender(e.to_string()))?;
        fs::write(out_dir.join("tests").join("verifier_tests.move"), tests).map_err(|e| {
            Error::Io {
                source: e,
                context: "write verifier_tests.move".to_string(),
            }
        })?;
    }

    write(
        out_dir.join("README.md"),
        render::readme_content(
            options.package_name,
            options.module_name,
            &input.curve_function,
            input.include_test_vectors,
            input.include_entry,
        ),
    )
    .map_err(|e| Error::Io {
        source: e,
        context: "write README.md".to_string(),
    })?;

    Ok(())
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
