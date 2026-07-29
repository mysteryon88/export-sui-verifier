pub mod arkworks;

use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::{Error, Result};

pub(crate) const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DECODED_ARTIFACT_BYTES: usize = 64 * 1024;

pub(crate) fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|source| Error::Io {
        source,
        context: format!("failed to inspect file {}", path.display()),
    })?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(Error::InputTooLarge {
            path: path.to_path_buf(),
            size: metadata.len(),
            max: MAX_INPUT_BYTES,
        });
    }

    let file = File::open(path).map_err(|source| Error::Io {
        source,
        context: format!("failed to open file {}", path.display()),
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| Error::Io {
            source,
            context: format!("failed to read file {}", path.display()),
        })?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(Error::InputTooLarge {
            path: path.to_path_buf(),
            size: bytes.len() as u64,
            max: MAX_INPUT_BYTES,
        });
    }
    Ok(bytes)
}

pub(crate) fn read_text_bounded(path: &Path) -> Result<String> {
    String::from_utf8(read_bounded(path)?).map_err(|source| Error::Io {
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        context: format!("{} is not valid UTF-8", path.display()),
    })
}

pub(crate) fn decode_hex_bounded(raw: &str, field: &str) -> Result<Vec<u8>> {
    let value = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
    if value.is_empty() {
        return Err(Error::HexParse(format!("{field} must not be empty")));
    }
    if value.len() > MAX_DECODED_ARTIFACT_BYTES * 2 {
        return Err(Error::HexParse(format!(
            "{field} exceeds the {MAX_DECODED_ARTIFACT_BYTES}-byte decoded artifact limit"
        )));
    }
    if !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::HexParse(format!("{field} must be a hex string")));
    }
    if !value.len().is_multiple_of(2) {
        return Err(Error::HexParse(format!("{field} has odd hex length")));
    }
    hex::decode(value).map_err(|source| Error::HexParse(format!("{field}: {source}")))
}

pub(crate) fn ensure_no_trailing_bytes(cursor: &Cursor<Vec<u8>>, field: &str) -> Result<()> {
    let position: usize = cursor
        .position()
        .try_into()
        .map_err(|_| Error::Serialization(format!("{field} cursor position overflow")))?;
    let trailing = cursor.get_ref().len().saturating_sub(position);
    if trailing == 0 {
        return Ok(());
    }
    Err(Error::Serialization(format!(
        "{field} has {trailing} trailing bytes after Arkworks compressed artifact"
    )))
}
