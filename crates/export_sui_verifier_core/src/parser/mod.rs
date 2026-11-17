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

// Validate structure without allocating Value trees, including ignored/unknown fields.
pub(crate) fn bounded_json<T: serde::de::DeserializeOwned>(content: &str) -> serde_json::Result<T> {
    check_json_budget(
        content,
        crate::model::MAX_PUBLIC_INPUTS + 1,
        crate::model::MAX_PUBLIC_INPUTS,
        (crate::model::MAX_PUBLIC_INPUTS + 1) * 16 + 1024,
    )?;
    serde_json::from_str(content)
}

fn check_json_budget(
    content: &str,
    collection: usize,
    public_limit: usize,
    total: usize,
) -> serde_json::Result<()> {
    use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
    struct Budget {
        collection: usize,
        public_limit: usize,
        remaining: usize,
    }
    struct Key;
    impl<'de> DeserializeSeed<'de> for Key {
        type Value = bool;
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            d: D,
        ) -> std::result::Result<bool, D::Error> {
            d.deserialize_str(self)
        }
    }
    impl<'de> Visitor<'de> for Key {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("JSON object key")
        }
        fn visit_str<E: serde::de::Error>(self, key: &str) -> std::result::Result<bool, E> {
            Ok(matches!(
                key,
                "public_input" | "public_inputs" | "publicSignals" | "public_signals"
            ))
        }
    }
    struct Node<'a> {
        budget: &'a mut Budget,
        index: usize,
    }
    impl<'de> DeserializeSeed<'de> for Node<'_> {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> std::result::Result<(), D::Error> {
            if self.index >= self.budget.collection {
                return Err(serde::de::Error::custom(
                    "JSON collection exceeds maximum item count",
                ));
            }
            if self.budget.remaining == 0 {
                return Err(serde::de::Error::custom("JSON structural budget exceeded"));
            }
            self.budget.remaining -= 1;
            deserializer.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Node<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded JSON")
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
            let mut index = 0;
            while seq
                .next_element_seed(Node {
                    budget: self.budget,
                    index,
                })?
                .is_some()
            {
                index += 1;
            }
            Ok(())
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
            let mut index = 0;
            while let Some(is_public) = map.next_key_seed(Key)? {
                // Compact metadata maps can exceed the curve's public-input array ceiling.
                if index >= self.budget.collection.max(32) {
                    return Err(serde::de::Error::custom(
                        "JSON collection exceeds maximum map entry count",
                    ));
                }
                let collection = self.budget.collection;
                if is_public {
                    self.budget.collection = self.budget.public_limit;
                }
                map.next_value_seed(Node {
                    budget: self.budget,
                    index: 0,
                })?;
                self.budget.collection = collection;
                index += 1;
            }
            Ok(())
        }
    }
    let mut deserializer = serde_json::Deserializer::from_str(content);
    Node {
        budget: &mut Budget {
            collection: if content.trim_start().starts_with('[') {
                public_limit
            } else {
                collection
            },
            public_limit,
            remaining: total,
        },
        index: 0,
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn stops_before_over_budget_value_is_parsed() {
        let err =
            check_json_budget("[null,null,{malformed expensive value]", 2, 2, 100).unwrap_err();
        assert!(err.to_string().contains("collection"), "{err}");
        let err = check_json_budget("[[null],[null],{malformed]", 10, 10, 5).unwrap_err();
        assert!(err.to_string().contains("budget"), "{err}");
        assert!(check_json_budget("{\"a\":[1,2],\"b\":true}", 2, 2, 6).is_ok());
    }
    #[test]
    fn public_array_ceiling_preserves_extra_ic_point() {
        assert!(check_json_budget("{\"IC\":[0,0,0]}", 3, 2, 100).is_ok());
        for key in [
            "public_input",
            "public_inputs",
            "publicSignals",
            "public_signals",
            "\\u0070ublicSignals",
        ] {
            let text = format!("{{\"{key}\":[0,0,{{malformed]}}");
            let err = check_json_budget(&text, 3, 2, 100).unwrap_err();
            assert!(err.to_string().contains("collection"), "{err}");
        }
    }
}
