//! `name-tests-test` — verify that test files are named correctly.
//!
//! Defaults to the pytest `.*_test.py` convention; `--pytest-test-first` and
//! `--django`/`--unittest` select the alternatives. `__init__.py` and
//! `conftest.py` are always allowed.

use clap::Parser;
use regex::Regex;
use std::path::Path;

#[derive(Parser)]
#[command(about = "Verify that test files are named correctly.")]
struct Args {
    /// Ensure tests match `.*_test.py` (the default).
    #[arg(long, conflicts_with_all = ["pytest_test_first", "django"])]
    pytest: bool,

    /// Ensure tests match `test_.*.py`.
    #[arg(long = "pytest-test-first", conflicts_with_all = ["pytest", "django"])]
    pytest_test_first: bool,

    /// Ensure tests match `test.*.py`.
    #[arg(long = "django", visible_alias = "unittest", conflicts_with_all = ["pytest", "pytest_test_first"])]
    django: bool,

    /// Filenames to check.
    filenames: Vec<String>,
}

fn pattern_for(args: &Args) -> &'static str {
    if args.pytest_test_first {
        r"test_.*\.py"
    } else if args.django {
        r"test.*\.py"
    } else {
        r".*_test\.py"
    }
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("name-tests-test".to_owned()).chain(args.iter().cloned()));
    let pattern = pattern_for(&args);
    let reg = Regex::new(pattern).expect("valid pattern");

    let mut retcode = 0;
    for filename in &args.filenames {
        let base = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        // Python `re.fullmatch`: the pattern must consume the whole basename.
        let full_match = reg
            .find(base)
            .is_some_and(|m| m.start() == 0 && m.end() == base.len());
        let allowed = full_match || base == "__init__.py" || base == "conftest.py";
        if !allowed {
            retcode = 1;
            println!("{filename} does not match pattern \"{pattern}\"");
        }
    }
    retcode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytest_default_accepts_and_rejects() {
        assert_eq!(run(&["foo_test.py".to_owned()]), 0);
        assert_eq!(run(&["foo.py".to_owned()]), 1);
        assert_eq!(run(&["__init__.py".to_owned()]), 0);
        assert_eq!(run(&["conftest.py".to_owned()]), 0);
    }

    #[test]
    fn test_first_convention() {
        assert_eq!(run(&["--pytest-test-first".to_owned(), "test_foo.py".to_owned()]), 0);
        assert_eq!(run(&["--pytest-test-first".to_owned(), "foo_test.py".to_owned()]), 1);
    }

    #[test]
    fn django_convention() {
        assert_eq!(run(&["--django".to_owned(), "testfoo.py".to_owned()]), 0);
        assert_eq!(run(&["--django".to_owned(), "foo_test.py".to_owned()]), 1);
    }
}
