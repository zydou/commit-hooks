//! `trailing-whitespace` — trim trailing whitespace from lines.
//!
//! Every line has its trailing whitespace removed (the default whitespace set,
//! or a custom set via `--chars`). For Markdown files, hard linebreaks (two
//! trailing spaces) are preserved unless extra extensions opt out.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Trim trailing whitespace from the given files.")]
struct Args {
    /// Accepted for backwards compatibility; has no effect.
    #[arg(long = "no-markdown-linebreak-ext")]
    no_markdown_linebreak_ext: bool,

    /// Markdown extensions (or `*`) whose hard linebreak spaces are preserved.
    /// May be given multiple times and/or as a comma-separated list.
    #[arg(long = "markdown-linebreak-ext")]
    markdown_linebreak_ext: Vec<String>,

    /// The set of characters to strip from line ends (defaults to whitespace).
    #[arg(long)]
    chars: Option<String>,

    /// Filenames to fix.
    filenames: Vec<String>,
}

/// The whitespace bytes Python's `bytes.rstrip()` treats as whitespace.
fn py_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `bytes.isspace()`: non-empty and every byte is whitespace.
fn py_bytes_isspace(line: &[u8]) -> bool {
    !line.is_empty() && line.iter().all(|&b| py_isspace(b))
}

/// `line.rstrip(chars)`: strip from the right the bytes in `chars`, or all
/// whitespace when `chars` is `None`.
fn rstrip<'a>(line: &'a [u8], chars: Option<&[u8]>) -> &'a [u8] {
    let is_strip = |b: u8| match chars {
        None => py_isspace(b),
        Some(set) => set.contains(&b),
    };
    let mut end = line.len();
    while end > 0 && is_strip(line[end - 1]) {
        end -= 1;
    }
    &line[..end]
}

/// Split `data` into lines on `\n`, keeping the `\n` (binary `readlines()`).
fn split_lines_keepends(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for i in 0..data.len() {
        if data[i] == b'\n' {
            lines.push(&data[start..=i]);
            start = i + 1;
        }
    }
    if start < data.len() {
        lines.push(&data[start..]);
    }
    lines
}

/// Trim one line, preserving its own line terminator.
fn process_line(line: &[u8], is_markdown: bool, chars: Option<&[u8]>) -> Vec<u8> {
    let (content, eol): (&[u8], &[u8]) = if line.ends_with(b"\r\n") {
        (&line[..line.len() - 2], b"\r\n")
    } else if line.ends_with(b"\n") {
        (&line[..line.len() - 1], b"\n")
    } else {
        (line, b"")
    };

    // Preserve Markdown hard linebreaks (two trailing spaces) on non-blank
    // lines, normalising to exactly two.
    if is_markdown && !py_bytes_isspace(content) && content.ends_with(b"  ") {
        let mut out = rstrip(&content[..content.len() - 2], chars).to_vec();
        out.extend_from_slice(b"  ");
        out.extend_from_slice(eol);
        out
    } else {
        let mut out = rstrip(content, chars).to_vec();
        out.extend_from_slice(eol);
        out
    }
}

/// Mimic `os.path.splitext` on POSIX: the extension is the trailing `.…` of the
/// basename, unless the basename starts with a dot (a hidden file).
fn splitext(path: &str) -> (&str, &str) {
    let base_start = path.rfind('/').map_or(0, |i| i + 1);
    let base = &path[base_start..];
    match base.rfind('.') {
        Some(0) | None => (path, ""),
        Some(i) => (&path[..base_start + i], &path[base_start + i..]),
    }
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("trailing-whitespace".to_owned()).chain(args.iter().cloned()),
    );

    if args.no_markdown_linebreak_ext {
        println!("--no-markdown-linebreak-ext now does nothing!");
    }

    let md_args = &args.markdown_linebreak_ext;
    if md_args.iter().any(|s| s.is_empty()) {
        eprintln!("error: --markdown-linebreak-ext requires a non-empty argument");
        return 2;
    }
    let all_markdown = md_args.iter().any(|s| s == "*");
    // Normalise extensions: split on ',', lowercase, force one leading '.'.
    let md_exts: Vec<String> = md_args
        .join(",")
        .split(',')
        .map(|x| format!(".{}", x.to_lowercase().trim_start_matches('.')))
        .collect();
    for ext in &md_exts {
        let rest = &ext[1..];
        if rest.chars().any(|c| matches!(c, '.' | '/' | '\\' | ':')) {
            eprintln!(
                "error: bad --markdown-linebreak-ext extension {ext:?} (has . / \\ :)\n  \
                 (probably filename; use '--markdown-linebreak-ext=EXT')"
            );
            return 2;
        }
    }

    let chars = args.chars.as_ref().map(|s| s.as_bytes().to_vec());
    let chars = chars.as_deref();

    let mut return_code = 0;
    for filename in &args.filenames {
        let Ok(data) = std::fs::read(filename) else {
            continue;
        };
        let lower = filename.to_lowercase();
        let (_, extension) = splitext(&lower);
        let md = all_markdown || md_exts.iter().any(|e| e == extension);

        let lines = split_lines_keepends(&data);
        let new: Vec<u8> = lines.iter().flat_map(|l| process_line(l, md, chars)).collect();
        if new != data {
            if std::fs::write(filename, &new).is_ok() {
                println!("Fixing {filename}");
                return_code = 1;
            }
        }
    }
    return_code
}

#[cfg(test)]
mod tests {
    use super::{process_line, splitext};

    #[test]
    fn trims_trailing_whitespace() {
        assert_eq!(process_line(b"foo  \n", false, None), b"foo\n");
        assert_eq!(process_line(b"foo\t\n", false, None), b"foo\n");
        assert_eq!(process_line(b"foo", false, None), b"foo");
    }

    #[test]
    fn preserves_line_ending() {
        assert_eq!(process_line(b"foo  \r\n", false, None), b"foo\r\n");
        assert_eq!(process_line(b"foo  \n", false, None), b"foo\n");
    }

    #[test]
    fn markdown_preserves_hard_linebreak() {
        assert_eq!(process_line(b"foo  \n", true, None), b"foo  \n");
        // More than two spaces still normalise to exactly two.
        assert_eq!(process_line(b"foo    \n", true, None), b"foo  \n");
        // Blank lines are still trimmed in markdown.
        assert_eq!(process_line(b"   \n", true, None), b"\n");
    }

    #[test]
    fn custom_chars() {
        let chars = b"x".as_slice();
        // Only trailing 'x' bytes are stripped; the space is kept.
        assert_eq!(process_line(b"foox x\n", false, Some(chars)), b"foox \n");
        assert_eq!(process_line(b"foox\n", false, Some(chars)), b"foo\n");
        // Spaces are not stripped when a custom set is given.
        assert_eq!(process_line(b"foo \n", false, Some(chars)), b"foo \n");
    }

    #[test]
    fn splitext_matches_python() {
        assert_eq!(splitext("foo.md").1, ".md");
        assert_eq!(splitext("foo").1, "");
        assert_eq!(splitext(".gitignore").1, "");
        assert_eq!(splitext("a/b.c.d").1, ".d");
        assert_eq!(splitext("dir.d/file").1, "");
    }
}
