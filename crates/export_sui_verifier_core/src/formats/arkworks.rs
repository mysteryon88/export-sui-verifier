use crate::error::Result;
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
