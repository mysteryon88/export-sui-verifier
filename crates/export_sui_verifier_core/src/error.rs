use serde_json::Error as SerdeJsonError;
use std::path::PathBuf;
use thiserror::Error;

/// Shared error type for strict parsing and generation.
#[derive(Debug, Error)]
pub enum Error {
    #[error("ERR_IO: {context}: {source}")]
    Io {
        #[source]
        source: std::io::Error,
        context: String,
    },
    #[error("ERR_JSON_PARSE: {context}: {source}")]
    JsonParse {
        #[source]
        source: SerdeJsonError,
        context: String,
    },
    #[error("ERR_UNSUPPORTED_CURVE: {0}")]
    UnsupportedCurve(String),
    #[error("ERR_CURVE_MISMATCH: {0}")]
    CurveMismatch(String),
    #[error("ERR_UNSUPPORTED_PROTOCOL: {0}")]
    UnsupportedProtocol(String),
    #[error("ERR_MALFORMED_G1: {0}")]
    MalformedG1(String),
    #[error("ERR_MALFORMED_G2: {0}")]
    MalformedG2(String),
    #[error("ERR_DECIMAL_PARSE: {0}")]
    DecimalParse(String),
    #[error("ERR_FIELD_OVERFLOW: {0}")]
    FieldOverflow(String),
    #[error("ERR_PUBLIC_INPUT_COUNT_MISMATCH: {0}")]
    PublicInputCountMismatch(String),
    #[error("ERR_IC_LENGTH: {0}")]
    IcLengthMismatch(String),
    #[error("ERR_POINT_NOT_ON_CURVE: {0}")]
    PointNotOnCurve(String),
    #[error("ERR_POINT_NOT_IN_SUBGROUP: {0}")]
    PointNotInSubgroup(String),
    #[error("ERR_MISSING_INPUT: {0}")]
    MissingInput(String),
    #[error("ERR_HEX_PARSE: {0}")]
    HexParse(String),
    #[error("ERR_LOCAL_VERIFICATION_FAILED: {0}")]
    LocalProofVerificationFailed(String),
    #[error("ERR_SERIALIZATION: {0}")]
    Serialization(String),
    #[error("ERR_TEMPLATE_RENDER: {0}")]
    TemplateRender(String),
    #[error("ERR_OUTPUT_EXISTS: {0}")]
    OutputExists(PathBuf),
    #[error("ERR_UNSAFE_OUTPUT_DIRECTORY: refusing to remove unsafe output directory {0}")]
    UnsafeOutputDirectory(PathBuf),
    #[error("ERR_SUI_TEST_FAILED: {0}")]
    SuiTestFailed(String),
    #[error("ERR_INVALID_MODULE_NAME: {0}")]
    InvalidModuleName(String),
    #[error("ERR_INVALID_PACKAGE_NAME: {0}")]
    InvalidPackageName(String),
    #[error("ERR_INVALID_ACCOUNT_ADDRESS: {0}")]
    InvalidAccountAddress(String),
    #[error("ERR_PREPARED_NOT_IMPLEMENTED: prepared mode is not implemented yet")]
    PreparedNotImplemented,
    #[error("ERR_TEMPLATE_MISSING: {0}")]
    MissingTemplate(String),
}

pub type Result<T> = std::result::Result<T, Error>;
