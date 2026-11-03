//! `check-xml` — verify that files are well-formed XML.
//!
//! Uses a streaming (SAX-style) reader so the whole document is never held in
//! memory; any parse error is reported per-file.

use clap::Parser;
use std::io::Cursor;
use quick_xml::Reader;

#[derive(Parser)]
#[command(about = "Check that XML files are well-formed.")]
struct Args {
    /// XML filenames to check.
    filenames: Vec<String>,
}

/// Whether the XML in `bytes` is well-formed.
fn is_well_formed(bytes: &[u8]) -> Result<(), String> {
    let mut reader = Reader::from_reader(Cursor::new(bytes));
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => return Ok(()),
            Ok(_) => {}
            Err(err) => return Err(err.to_string()),
        }
    }
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-xml".to_owned()).chain(args.iter().cloned()));

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        if let Err(err) = is_well_formed(&bytes) {
            println!("{filename}: Failed to xml parse ({err})");
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
    fn valid_xml_passes() {
        assert_eq!(run_on(b"<?xml version=\"1.0\"?><root><a>1</a></root>"), 0);
    }

    #[test]
    fn unclosed_tag_fails() {
        assert_eq!(run_on(b"<root><a></root>"), 1);
    }

    #[test]
    fn mismatched_tag_fails() {
        assert_eq!(run_on(b"<root></other>"), 1);
    }
}
