pub mod bytes;
pub mod curves;
pub mod error;
pub mod formats;
pub mod model;
pub mod movegen;
pub mod parser;
pub mod snarkjs;
pub mod verifier;

pub use crate::curves::{create_adapter, CurveAdapter, CurveId, PointFormat};
pub use crate::error::{Error, Result};
pub use crate::formats::{
    load_arkworks_bundle, load_arkworks_inputs, load_compact_bundle, load_gnark_binary_inputs,
    load_gnark_binary_inputs_auto, load_gnark_json_inputs, load_snarkjs_json_inputs,
    load_snarkjs_json_inputs_with_curve_hint, load_snarkjs_json_inputs_with_optional_proof,
};
pub use crate::model::{
    CurveKind, DecimalValue, Groth16G1Point, Groth16G2Point, Groth16Proof, Groth16VerificationKey,
    Groth16VerifierInputs, SourceFormat,
};
pub use crate::movegen::{
    generate_move_package, proof_data_snippet, GenerateMovePackageOptions, MovegenMode,
    ProofDataSnippet,
};
pub use crate::snarkjs::{
    parse_compact_artifact, parse_proof, parse_public_inputs, parse_verification_key, Proof,
    VerificationKey,
};
pub use crate::verifier::local_verify;
