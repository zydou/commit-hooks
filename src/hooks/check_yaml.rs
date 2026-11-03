//! `check-yaml` — verify that files parse as YAML.
//!
//! By default a single document is loaded; `--allow-multiple-documents`
//! accepts multi-document streams and `--unsafe` performs a syntax-only parse
//! (implying multi-document, as upstream does).

use clap::Parser;
use serde::Deserialize;

#[derive(Parser)]
#[command(about = "Check that YAML files parse as valid YAML.")]
struct Args {
    /// Accept YAML files using multi-document syntax.
    #[arg(short = 'm', long = "multi", visible_alias = "allow-multiple-documents")]
    multi: bool,

    /// Parse for syntax only, enabling extensions/unsafe constructs.
    /// Implies --allow-multiple-documents.
    #[arg(long)]
    unsafe_: bool,

    /// Filenames to check.
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-yaml".to_owned()).chain(args.iter().cloned()));
    let multi = args.multi || args.unsafe_;

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            println!("{filename}: Failed to yaml parse (invalid UTF-8)");
            retval = 1;
            continue;
        };
        let result = if multi {
            parse_multi(text)
        } else {
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).map(|_| ())
        };
        if let Err(err) = result {
            println!("{filename}: {err}");
            retval = 1;
        }
    }
    retval
}

/// Parse a (possibly multi-document) stream, discarding the values.
///
/// The `Deserializer` implements `Iterator`, yielding one deserializer per
/// document; each is consumed into a `Value` to surface any parse error.
fn parse_multi(text: &str) -> Result<(), serde_yaml_ng::Error> {
    let mut de = serde_yaml_ng::Deserializer::from_str(text);
    while let Some(doc) = de.next() {
        serde_yaml_ng::Value::deserialize(doc)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with(contents: &[u8], extra: &[&str]) -> i32 {
        let path = crate::util::write_temp_file(contents);
        let mut argv = vec![path.to_string_lossy().into_owned()];
        argv.extend(extra.iter().map(|s| s.to_string()));
        run(&argv)
    }

    #[test]
    fn valid_single_doc_passes() {
        assert_eq!(run_with(b"key: value\nnum: 42\n", &[]), 0);
    }

    #[test]
    fn invalid_yaml_fails() {
        assert_eq!(run_with(b"key: value\n  bad: : indent\n", &[]), 1);
    }

    #[test]
    fn multi_doc_requires_flag() {
        let content = b"---\na: 1\n---\nb: 2\n";
        // Without the flag, a multi-document stream is an error.
        assert_eq!(run_with(content, &[]), 1);
        // With the flag it parses.
        assert_eq!(run_with(content, &["--multi"]), 0);
    }
}
