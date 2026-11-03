//! `requirements-txt-fixer` — sort entries in requirements.txt.
//!
//! Requirements are grouped with their trailing comments and sorted by package
//! name (a leading `#` comment at the top of the file stays on top). The broken
//! Debian `pkg-resources==0.0.0` entry is removed and duplicates collapsed.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Sort entries in requirements.txt and constraints.txt.")]
struct Args {
    /// Filenames to fix.
    filenames: Vec<String>,
}

const PASS: i32 = 0;
const FAIL: i32 = 1;

/// A single requirement plus any attached comment lines.
#[derive(Debug, Clone)]
struct Requirement {
    value: Option<Vec<u8>>,
    comments: Vec<Vec<u8>>,
}

/// The package name used for sorting (lower-cased, comparisons stripped).
impl Requirement {
    fn name(&self) -> Vec<u8> {
        let value = self.value.as_ref().expect("name called before value set");
        let lower: Vec<u8> = value.iter().map(|b| b.to_ascii_lowercase()).collect();
        for egg in [&b"#egg="[..], &b"&egg="[..]] {
            if let Some(pos) = find_subslice(&lower, egg) {
                return lower[pos + egg.len()..].to_vec();
            }
        }
        // Everything up to the first separator / comparison operator.
        let end = lower
            .iter()
            .position(|b| matches!(b, b'=' | b'!' | b'~' | b'<' | b'>' | b';' | b' '))
            .unwrap_or(lower.len());
        lower[..end].to_vec()
    }

    fn is_complete(&self) -> bool {
        match &self.value {
            None => false,
            Some(v) => {
                let trimmed = trim_end_newlines(v);
                !trimmed.ends_with(b"\\")
            }
        }
    }

    fn append_value(&mut self, value: &[u8]) {
        match &mut self.value {
            Some(v) => v.extend_from_slice(value),
            None => self.value = Some(value.to_vec()),
        }
    }
}

/// The upstream ordering: the top-of-file newline sorts first, otherwise by
/// name (a commented requirement sorts before an uncommented one on ties).
fn req_lt(a: &Requirement, b: &Requirement) -> bool {
    let av = a.value.as_ref().unwrap();
    let bv = b.value.as_ref().unwrap();
    if av.as_slice() == b"\n" {
        return true;
    }
    if bv.as_slice() == b"\n" {
        return false;
    }
    if a.name() == b.name() {
        return (!a.comments.is_empty()) > (!b.comments.is_empty());
    }
    a.name() < b.name()
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn trim_end_newlines(v: &[u8]) -> &[u8] {
    let mut end = v.len();
    while end > 0 && (v[end - 1] == b'\n' || v[end - 1] == b'\r') {
        end -= 1;
    }
    &v[..end]
}

fn fix_requirements(before_lines: &[Vec<u8>]) -> (Vec<u8>, i32) {
    let mut before = before_lines.to_vec();
    let before_string: Vec<u8> = before.iter().flatten().copied().collect();

    // Ensure the last line ends with a newline so a change is well-formed.
    if let Some(last) = before.last_mut() {
        if !last.ends_with(b"\n") {
            last.push(b'\n');
        }
    }

    // Empty (whitespace-only) file: nothing to do.
    if before_string.iter().all(|b| b.is_ascii_whitespace()) {
        return (before_string, PASS);
    }

    let mut requirements: Vec<Requirement> = Vec::new();
    for line in &before {
        if requirements.is_empty() || requirements.last().unwrap().is_complete() {
            requirements.push(Requirement { value: None, comments: Vec::new() });
        }
        let is_first = requirements.len() == 1;
        let last = requirements.last_mut().unwrap();

        let stripped = line.iter().filter(|b| !b.is_ascii_whitespace()).count();
        if is_first && stripped == 0 {
            // A newline before any requirement is a top-of-file comment.
            if !last.comments.is_empty() && last.comments[0].starts_with(b"#") {
                last.value = Some(b"\n".to_vec());
            } else {
                last.comments.push(line.clone());
            }
        } else if line.trim_ascii_start().starts_with(b"#") || stripped == 0 {
            last.comments.push(line.clone());
        } else {
            last.append_value(line);
        }
    }

    // A trailing comment (no value) is preserved at the end.
    let rest = if requirements.last().is_some_and(|r| r.value.is_none()) {
        requirements.pop().unwrap().comments
    } else {
        Vec::new()
    };

    // Drop the broken Debian pkg-resources entry.
    requirements.retain(|r| {
        r.value.as_deref() != Some(&b"pkg-resources==0.0.0\n"[..])
            && r.value.as_deref() != Some(&b"pkg_resources==0.0.0\n"[..])
    });

    requirements.sort_by(|a, b| if req_lt(a, b) { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater });

    let mut after: Vec<u8> = Vec::new();
    let mut prev: Option<Vec<u8>> = None;
    for req in &requirements {
        for comment in &req.comments {
            after.extend_from_slice(comment);
        }
        let value = req.value.as_ref().unwrap();
        if prev.as_deref() != Some(value.as_slice()) {
            after.extend_from_slice(value);
            prev = Some(value.clone());
        }
    }
    for line in &rest {
        after.extend_from_slice(line);
    }

    if before_string == after {
        (before_string, PASS)
    } else {
        (after, FAIL)
    }
}

fn split_lines(data: &[u8]) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for i in 0..data.len() {
        if data[i] == b'\n' {
            lines.push(data[start..=i].to_vec());
            start = i + 1;
        }
    }
    if start < data.len() {
        lines.push(data[start..].to_vec());
    }
    lines
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("requirements-txt-fixer".to_owned()).chain(args.iter().cloned()),
    );

    let mut retv = PASS;
    for filename in &args.filenames {
        let Ok(before) = std::fs::read(filename) else {
            continue;
        };
        let (after, ret) = fix_requirements(&split_lines(&before));
        if ret != PASS {
            if std::fs::write(filename, &after).is_ok() {
                println!("Sorting {filename}");
            }
        }
        retv |= ret;
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::{fix_requirements, split_lines};

    fn fix(input: &[u8]) -> (Vec<u8>, i32) {
        // Build the line vector the way split_lines would, preserving a final
        // newline so the last (possibly empty) element is handled identically.
        let lines = split_lines(input);
        fix_requirements(&lines)
    }

    #[test]
    fn sorts_alphabetically() {
        let (out, ret) = fix(b"zlib==1.0\nflake8==3.0\n");
        assert_eq!(ret, 1);
        assert_eq!(out, b"flake8==3.0\nzlib==1.0\n");
    }

    #[test]
    fn keeps_top_comment() {
        // The leading comment stays attached to its requirement (zlib), so the
        // alphabetically-first flake8 is emitted first — matches upstream.
        let (out, ret) = fix(b"# top comment\nzlib==1.0\nflake8==3.0\n");
        assert_eq!(ret, 1);
        assert_eq!(out, b"flake8==3.0\n# top comment\nzlib==1.0\n");
    }

    #[test]
    fn removes_pkg_resources() {
        let (out, _) = fix(b"zlib==1.0\npkg-resources==0.0.0\n");
        assert_eq!(out, b"zlib==1.0\n");
    }

    #[test]
    fn already_sorted_passes() {
        let (_, ret) = fix(b"flake8==3.0\nzlib==1.0\n");
        assert_eq!(ret, 0);
    }
}
