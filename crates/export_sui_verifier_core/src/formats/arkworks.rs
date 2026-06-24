use crate::curves::create_adapter;
use crate::error::{Error, Result};
use crate::model::Groth16VerifierInputs;
use crate::parser::arkworks;
use std::path::Path;

pub fn load_arkworks_inputs(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
    curve_hint: Option<&str>,
) -> Result<Groth16VerifierInputs> {
    arkworks::load_arkworks_inputs(vk_path, proof_path, public_path, curve_hint)
}

pub fn load_arkworks_bundle(
    path: &Path,
    curve_hint: Option<&str>,
) -> Result<Groth16VerifierInputs> {
    arkworks::load_arkworks_bundle(path, curve_hint)
}

pub fn load_arkworks_inputs_auto(
    vk_path: &Path,
    proof_path: Option<&Path>,
    public_path: Option<&Path>,
) -> Result<Groth16VerifierInputs> {
    infer_arkworks_curve("arkworks inputs", |curve| {
        arkworks::load_arkworks_inputs(vk_path, proof_path, public_path, Some(curve))
    })
}

pub fn load_arkworks_bundle_auto(path: &Path) -> Result<Groth16VerifierInputs> {
    infer_arkworks_curve("arkworks bundle", |curve| {
        arkworks::load_arkworks_bundle(path, Some(curve))
    })
}

fn infer_arkworks_curve(
    label: &str,
    mut load: impl FnMut(&str) -> Result<Groth16VerifierInputs>,
) -> Result<Groth16VerifierInputs> {
    let mut matches = Vec::new();
    let mut errors = Vec::new();

    for curve in ["bn254", "bls12381"] {
        match load(curve).and_then(validate_candidate) {
            Ok(inputs) => {
                if !matches
                    .iter()
                    .any(|existing: &Groth16VerifierInputs| existing.curve == inputs.curve)
                {
                    matches.push(inputs);
                }
            }
            Err(err) => errors.push(format!("{curve}: {err}")),
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Error::MissingInput(format!(
            "could not auto-detect {label} curve: {}",
            errors.join("; ")
        ))),
        _ => Err(Error::CurveMismatch(format!(
            "{label} are valid for more than one supported curve"
        ))),
    }
}

fn validate_candidate(inputs: Groth16VerifierInputs) -> Result<Groth16VerifierInputs> {
    let adapter = create_adapter(inputs.curve.canonical_name())?;
    if inputs.has_test_vectors() {
        match adapter.local_verify(&inputs) {
            Ok(true) => Ok(inputs),
            Ok(false) => Err(Error::LocalProofVerificationFailed(
                "local verification returned false".to_string(),
            )),
            Err(err) => Err(err),
        }
    } else {
        adapter.serialize_verifying_key(&inputs.verifying_key)?;
        Ok(inputs)
    }
}
