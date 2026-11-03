//! `python-shebang-coding` — add a shebang and a `coding` declaration to Python
//! files that lack one.
//!
//! The hook inspects the top of each file and, when needed, prepends a shebang
//! line (customisable with `--shebang`, defaulting to `#!/usr/bin/env python`)
//! and a coding declaration (`# -*- coding: utf-8 -*-`). Files that already have
//! both are left untouched. A file that consists only of a lone shebang or a
//! lone coding declaration (no other content) is emptied, since such a stub is
//! not useful on its own.

use clap::Parser;
use regex::Regex;
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};

const DEFAULT_SHEBANG: &str = "#!/usr/bin/env python";
const DEFAULT_CODING: &str = "# -*- coding: utf-8 -*-";

/// The desired top-of-file contents after analysis.
struct ExpectedContents {
    shebang: String,
    coding: String,
    rest: String,
    shebang_status: bool,
    coding_status: bool,
    ending: &'static str,
}

/// Read one `readline`-style line (including its trailing `\n`, or up to EOF)
/// from `bytes` starting at `*pos`, returning the line and advancing `*pos`.
fn read_line(bytes: &[u8], pos: &mut usize) -> String {
    if *pos >= bytes.len() {
        return String::new();
    }
    let start = *pos;
    let end = match bytes[start..].iter().position(|&b| b == b'\n') {
        Some(i) => start + i + 1,
        None => bytes.len(),
    };
    *pos = end;
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// Remove a single trailing line terminator (`\r\n` or `\n`), leaving any other
/// content (including trailing whitespace) intact.
fn strip_terminator(line: &str) -> &str {
    if let Some(rest) = line.strip_suffix("\r\n") {
        rest
    } else {
        line.strip_suffix('\n').unwrap_or(line)
    }
}

/// Decide what the top of the file should look like.
///
/// `shebang` is the shebang to add when the file lacks one. `first_line`,
/// `second_line` and `rest` are the newline-translated first line, second line
/// and remaining content of the file.
fn get_expected_contents(
    shebang: String,
    first_line: &str,
    second_line: &str,
    mut rest: String,
) -> ExpectedContents {
    let ending = if first_line.ends_with("\r\n") { "\r\n" } else { "\n" };

    let (shebang, shebang_status, potential_coding) = if first_line.starts_with("#!") {
        // Already has a shebang: a coding declaration would be on the second line.
        (strip_terminator(first_line).to_owned(), true, second_line.to_owned())
    } else {
        // No shebang: the first line is the potential coding line, and the second
        // line is folded back into the rest of the file.
        rest = format!("{second_line}{rest}");
        (shebang, false, first_line.to_owned())
    };

    // A coding declaration looks like `# -*- coding: utf-8 -*-` or `# coding=utf8`.
    let coding_re = Regex::new("^[ \t\x0c]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)").unwrap();
    let coding_status = coding_re.is_match(strip_terminator(&potential_coding));
    let (coding, coding_status) = if coding_status {
        (strip_terminator(&potential_coding).to_owned(), true)
    } else {
        // No coding declaration: fold the potential coding line back into the rest.
        rest = format!("{potential_coding}{rest}");
        (DEFAULT_CODING.to_owned(), false)
    };

    ExpectedContents {
        shebang,
        coding,
        rest,
        shebang_status,
        coding_status,
        ending,
    }
}

/// Add a shebang/coding declaration to `filename` if missing. `shebang` is the
/// shebang to add when the file lacks one. Returns nonzero if the file changed.
fn fix_shebang_coding(filename: &str, shebang: String) -> i32 {
    let Ok(mut file) = OpenOptions::new().read(true).write(true).open(filename) else {
        return 0;
    };
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return 0;
    }
    // Read with universal-newline semantics: `\r\n` and `\r` both become `\n`.
    let translated = String::from_utf8_lossy(&bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    let mut pos = 0;
    let first_line = read_line(translated.as_bytes(), &mut pos);
    let second_line = read_line(translated.as_bytes(), &mut pos);
    let rest = translated[pos..].to_owned();

    let expected = get_expected_contents(shebang, &first_line, &second_line, rest);

    // Already has both a shebang and a coding declaration: nothing to do.
    if expected.shebang_status && expected.coding_status {
        return 0;
    }

    // Special case: the remaining content is only whitespace.
    if expected.rest.trim().is_empty() {
        // A lone shebang or a lone coding declaration is not useful; remove it.
        if expected.shebang_status ^ expected.coding_status {
            let _ = file.set_len(0);
            let _ = file.seek(std::io::SeekFrom::Start(0));
            return 1;
        }
        return 0;
    }

    // Write out the new contents: shebang, then coding, then the rest.
    let _ = file.set_len(0);
    let _ = file.seek(std::io::SeekFrom::Start(0));
    let _ = write!(file, "{}{}", expected.shebang, expected.ending);
    let _ = write!(file, "{}{}", expected.coding, expected.ending);
    let _ = file.write_all(expected.rest.as_bytes());
    1
}

#[derive(Parser)]
#[command(
    name = "python-shebang-coding",
    about = "Add shebang and coding to python files"
)]
struct Args {
    /// The shebang to add to files that lack one.
    #[arg(long, default_value = DEFAULT_SHEBANG)]
    shebang: String,

    /// Filenames to add.
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("python-shebang-coding".to_owned()).chain(args.iter().cloned()),
    );

    let mut retv = 0;
    for filename in &args.filenames {
        let file_ret = fix_shebang_coding(filename, args.shebang.clone());
        retv |= file_ret;
        if file_ret != 0 {
            println!("Added `{}` to {filename}", args.shebang);
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A unique temp path per test, so parallel tests never share a file.
    fn temp_path(tag: &str) -> String {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join("commit-hooks-tests");
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("shebang-{tag}-{}-{n}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn run_case(tag: &str, input: &str) -> (i32, String) {
        let path = temp_path(tag);
        fs::write(&path, input).unwrap();
        let rc = run(&[path.clone()]);
        let out = fs::read_to_string(&path).unwrap_or_default();
        let _ = fs::remove_file(&path);
        (rc, out)
    }

    #[test]
    fn adds_shebang_and_coding_to_plain_file() {
        let (rc, out) = run_case("plain", "x = 1\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "#!/usr/bin/env python\n# -*- coding: utf-8 -*-\nx = 1\n");
    }

    #[test]
    fn already_has_both_is_noop() {
        let input = "#!/usr/bin/env python\n# -*- coding: utf-8 -*-\nx = 1\n";
        let (rc, out) = run_case("both", input);
        assert_eq!(rc, 0);
        assert_eq!(out, input);
    }

    #[test]
    fn coding_only_gets_shebang() {
        let (rc, out) = run_case("codingonly", "# coding=utf-8\nx = 1\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "#!/usr/bin/env python\n# coding=utf-8\nx = 1\n");
    }

    #[test]
    fn shebang_only_gets_coding() {
        let (rc, out) = run_case("shebangonly", "#!/usr/bin/env python\nx = 1\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "#!/usr/bin/env python\n# -*- coding: utf-8 -*-\nx = 1\n");
    }

    #[test]
    fn lone_shebang_is_removed() {
        let (rc, out) = run_case("loneshebang", "#!/usr/bin/env python\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "");
    }

    #[test]
    fn lone_coding_is_removed() {
        let (rc, out) = run_case("lonecoding", "# -*- coding: utf-8 -*-\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "");
    }

    #[test]
    fn empty_file_is_noop() {
        let (rc, out) = run_case("empty", "");
        assert_eq!(rc, 0);
        assert_eq!(out, "");
    }

    #[test]
    fn whitespace_only_file_is_noop() {
        let (rc, out) = run_case("ws", "   \n  \n");
        assert_eq!(rc, 0);
        assert_eq!(out, "   \n  \n");
    }

    #[test]
    fn no_trailing_newline_is_preserved() {
        let (rc, out) = run_case("notrailnl", "x = 1");
        assert_eq!(rc, 1);
        assert_eq!(out, "#!/usr/bin/env python\n# -*- coding: utf-8 -*-\nx = 1");
    }

    #[test]
    fn universal_newlines_normalise_crlf() {
        // `\r\n` is translated to `\n` on read, so the result uses `\n`.
        let (rc, out) = run_case("crlf", "x = 1\r\ny = 2\r\n");
        assert_eq!(rc, 1);
        assert_eq!(out, "#!/usr/bin/env python\n# -*- coding: utf-8 -*-\nx = 1\ny = 2\n");
    }

    #[test]
    fn custom_shebang_is_written() {
        let path = temp_path("customshebang");
        fs::write(&path, "x = 1\n").unwrap();
        let rc = run(&[path.clone(), "--shebang".to_owned(), "#!/usr/bin/env python3".to_owned()]);
        assert_eq!(rc, 1);
        let out = fs::read_to_string(&path).unwrap();
        assert_eq!(out, "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\nx = 1\n");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn custom_coding_line_is_kept_verbatim() {
        let (rc, out) = run_case("customcoding", "#!/usr/bin/env python3\n# coding: latin-1\nx=1\n");
        assert_eq!(rc, 0);
        assert_eq!(out, "#!/usr/bin/env python3\n# coding: latin-1\nx=1\n");
    }
}
