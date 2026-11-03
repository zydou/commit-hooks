//! `end-of-file-fixer` — ensure files end with exactly one newline.
//!
//! A file must be empty or end in a single line terminator (`\n`, `\r\n`, or
//! `\r`). Trailing blank lines are collapsed; a missing final newline is added.
//! The terminator style already present at the end of the file is preserved.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Ensure files are empty or end with exactly one newline.")]
struct Args {
    /// Filenames to fix.
    filenames: Vec<String>,
}

/// Compute the corrected contents of `data`.
///
/// Returns `None` when the file already conforms (so the caller avoids a
/// needless rewrite); otherwise returns the new bytes to write.
fn fixed_contents(data: &[u8]) -> Option<Vec<u8>> {
    // An empty file conforms and (like the upstream `seek(-1)` raising on an
    // empty stream) is left untouched.
    if data.is_empty() {
        return None;
    }

    let mut pos = data.len();
    let mut last = data[pos - 1];

    if last != b'\n' && last != b'\r' {
        // No trailing newline: add one.
        let mut out = data.to_vec();
        out.push(b'\n');
        return Some(out);
    }

    // Walk backwards over the trailing run of line terminators, exactly like
    // the upstream seek/read loop. `pos` is always the offset one past the
    // byte we just examined.
    loop {
        if pos == 1 {
            // The whole file is line terminators: make it empty.
            return Some(Vec::new());
        }
        last = data[pos - 2];
        pos -= 1;
        if last != b'\n' && last != b'\r' {
            break;
        }
    }

    // `pos` is one past the last non-terminator byte, so `remaining` is the
    // trailing run of terminators.
    let remaining = &data[pos..];
    for seq in [&b"\n"[..], &b"\r\n"[..], &b"\r"[..]] {
        if remaining == seq {
            // Already ends with exactly one terminator.
            return None;
        }
        if remaining.starts_with(seq) {
            // Keep a single terminator of the file's own style.
            return Some(data[..pos + seq.len()].to_vec());
        }
    }
    None
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("end-of-file-fixer".to_owned()).chain(args.iter().cloned()),
    );

    let mut retv = 0;
    for filename in &args.filenames {
        let Ok(data) = std::fs::read(filename) else {
            continue;
        };
        if let Some(new) = fixed_contents(&data) {
            if std::fs::write(filename, &new).is_ok() {
                println!("Fixing {filename}");
                retv |= 1;
            }
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::fixed_contents;

    fn fix(data: &[u8]) -> Vec<u8> {
        fixed_contents(data).unwrap_or_else(|| data.to_vec())
    }

    #[test]
    fn adds_missing_newline() {
        assert_eq!(fix(b"a"), b"a\n");
        assert_eq!(fix(b"a\nb"), b"a\nb\n");
    }

    #[test]
    fn collapses_trailing_blank_lines() {
        assert_eq!(fix(b"a\n\n\n"), b"a\n");
        assert_eq!(fix(b"a\n\n"), b"a\n");
    }

    #[test]
    fn leaves_single_newline_untouched() {
        assert_eq!(fixed_contents(b"a\n"), None);
        assert_eq!(fixed_contents(b"a\r\n"), None);
        assert_eq!(fixed_contents(b"a\r"), None);
    }

    #[test]
    fn preserves_terminator_style() {
        assert_eq!(fix(b"a\r\n\r\n"), b"a\r\n");
        assert_eq!(fix(b"a\r\r"), b"a\r");
    }

    #[test]
    fn empty_and_all_newlines() {
        assert_eq!(fixed_contents(b""), None);
        assert_eq!(fix(b"\n"), b"");
        assert_eq!(fix(b"\n\n\n"), b"");
    }
}
