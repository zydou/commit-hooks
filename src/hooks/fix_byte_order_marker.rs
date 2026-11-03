//! `fix-byte-order-marker` — remove the UTF-8 byte order marker (BOM).
//!
//! A leading `EF BB BF` is stripped from each file and the remainder is
//! rewritten verbatim.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Remove the UTF-8 byte order marker from files.")]
struct Args {
    /// Filenames to fix.
    filenames: Vec<String>,
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("fix-byte-order-marker".to_owned()).chain(args.iter().cloned()),
    );

    let mut retv = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        if bytes.starts_with(BOM) {
            if std::fs::write(filename, &bytes[BOM.len()..]).is_ok() {
                println!("{filename}: removed byte-order marker");
                retv = 1;
            }
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(contents: &[u8]) -> (i32, Vec<u8>) {
        let dir = std::env::temp_dir().join("fix-bom-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f");
        std::fs::write(&path, contents).unwrap();
        let code = run(&[path.to_string_lossy().into_owned()]);
        let after = std::fs::read(&path).unwrap();
        (code, after)
    }

    #[test]
    fn removes_bom() {
        let (code, after) = run_on(b"\xEF\xBB\xBFhello");
        assert_eq!(code, 1);
        assert_eq!(after, b"hello");
    }

    #[test]
    fn leaves_clean_file_alone() {
        let (code, after) = run_on(b"hello");
        assert_eq!(code, 0);
        assert_eq!(after, b"hello");
    }
}
