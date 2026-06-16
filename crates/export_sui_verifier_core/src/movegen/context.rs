use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MovegenTemplateInput {
    pub package_name: String,
    pub module_name: String,
    pub curve_function: String,
    pub verifying_key_bytes: String,
    pub proof_bytes: String,
    pub public_inputs_bytes: String,
    pub include_test_vectors: bool,
    pub include_entry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovegenMode {
    Library,
    Entry,
    Test,
}

impl MovegenMode {
    pub fn include_entry(self) -> bool {
        matches!(self, Self::Entry | Self::Test)
    }
}
