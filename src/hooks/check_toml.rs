//! `check-toml` — verify that files parse as TOML.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Check that TOML files parse as valid TOML.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-toml".to_owned()).chain(args.iter().cloned()));

    let mut retval = 0;
    for filename in &args.filenames {
        let Ok(bytes) = std::fs::read(filename) else {
            continue;
        };
        match std::str::from_utf8(&bytes) {
            Ok(text) => {
                if let Err(err) = toml::from_str::<toml::Value>(text) {
                    println!("{filename}: {err}");
                    retval = 1;
                }
            }
            Err(err) => {
                println!("{filename}: {err}");
                retval = 1;
            }
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
    fn valid_toml_passes() {
        assert_eq!(run_on(b"[section]\nkey = \"value\"\nnum = 42\n"), 0);
    }

    #[test]
    fn invalid_toml_fails() {
        assert_eq!(run_on(b"key = \nbroken"), 1);
    }
}
