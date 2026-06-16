pub mod endian;
pub mod hex;

pub use endian::to_le_padded_bytes;
pub use hex::to_hex as to_move_hex;

/// Convert bytes to lowercase Move hex literal without prefix and without escaping.
pub fn move_hex_literal(data: &[u8]) -> String {
    format!("x\"{}\"", hex::to_hex(data))
}
