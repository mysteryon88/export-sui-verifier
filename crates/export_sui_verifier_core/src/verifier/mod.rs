use crate::curves::CurveAdapter;
use crate::error::Result;
use crate::model::Groth16VerifierInputs;
mod local_verify;

pub fn local_verify(adapter: &dyn CurveAdapter, inputs: &Groth16VerifierInputs) -> Result<bool> {
    inputs.validate()?;
    adapter.local_verify(inputs)
}
