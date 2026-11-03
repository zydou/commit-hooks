//! `check-json` — verify that files parse as JSON.
//!
//! On top of plain syntax checking, duplicate object keys are rejected (the
//! upstream hook uses a `object_pairs_hook` for this). serde_json silently
//! keeps the *last* duplicate, so we drive a custom seed visitor that walks
//! the whole document and fails on any repeated key.

use clap::Parser;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use std::collections::HashSet;

#[derive(Parser)]
#[command(about = "Check that JSON files parse as valid JSON (rejecting duplicate keys).")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

/// Seed that recursively validates a JSON document for duplicate keys.
struct DupSeed;

impl<'de> DeserializeSeed<'de> for DupSeed {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(DupVisitor)
    }
}

struct DupVisitor;

impl<'de> Visitor<'de> for DupVisitor {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(A::Error::custom(format!("Duplicate key: {key}")));
            }
            map.next_value_seed(DupSeed)?;
        }
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        while let Some(()) = seq.next_element_seed(DupSeed)? {}
        Ok(())
    }

    // JSON scalars carry no keys; accept them unchanged. serde_json routes
    // numbers to i64/u64/f64 and strings to the borrowed/owned `visit_*_str`.
    fn visit_bool<E: serde::de::Error>(self, _v: bool) -> Result<Self::Value, E> { Ok(()) }
    fn visit_i64<E: serde::de::Error>(self, _v: i64) -> Result<Self::Value, E> { Ok(()) }
    fn visit_u64<E: serde::de::Error>(self, _v: u64) -> Result<Self::Value, E> { Ok(()) }
    fn visit_f64<E: serde::de::Error>(self, _v: f64) -> Result<Self::Value, E> { Ok(()) }
    fn visit_str<E: serde::de::Error>(self, _v: &str) -> Result<Self::Value, E> { Ok(()) }
    fn visit_borrowed_str<E: serde::de::Error>(self, _v: &str) -> Result<Self::Value, E> { Ok(()) }
    fn visit_string<E: serde::de::Error>(self, _v: String) -> Result<Self::Value, E> { Ok(()) }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(()) }
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-json".to_owned()).chain(args.iter().cloned()));

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        if let Err(err) = DupSeed.deserialize(&mut de) {
            println!("{filename}: Failed to json decode ({err})");
            retval = 1;
        }
    }
    retval
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(contents: &[u8]) -> i32 {
        let path = crate::util::write_temp_file(contents);
        run(&[path.to_string_lossy().into_owned()])
    }

    #[test]
    fn valid_json_passes() {
        assert_eq!(run_on(b"{\"a\": 1, \"b\": [1, 2, 3]}"), 0);
    }

    #[test]
    fn invalid_syntax_fails() {
        assert_eq!(run_on(b"{\"a\": 1,,}"), 1);
    }

    #[test]
    fn duplicate_top_level_key_fails() {
        assert_eq!(run_on(b"{\"a\": 1, \"a\": 2}"), 1);
    }

    #[test]
    fn duplicate_nested_key_fails() {
        assert_eq!(run_on(b"{\"a\": {\"b\": 1, \"b\": 2}}"), 1);
    }

    #[test]
    fn duplicate_key_in_array_object_fails() {
        assert_eq!(run_on(b"[{\"x\": 1, \"x\": 2}]"), 1);
    }
}
