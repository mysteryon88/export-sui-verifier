pub mod model;
pub mod parser;
pub mod validate;

pub use model::{parse_decimal, DecimalValue, Proof, SnarkJsG1, SnarkJsG2, VerificationKey};
pub use parser::{
    parse_compact_artifact, parse_proof, parse_public_inputs, parse_verification_key,
};
pub use validate::{
    validate_curve_match, validate_protocol, validate_public_counts,
    validate_verification_key_geometry,
};
