//! `detect-private-key` — detect the presence of private keys.
//!
//! Scans each file for well-known private-key markers (RSA/DSA/EC/OpenSSH/PGP
//! blocks, PuTTY keys, …). Any match fails the hook.

use clap::Parser;

/// Byte markers that indicate a private key is present.
const BLACKLIST: &[&[u8]] = &[
    b"BEGIN RSA PRIVATE KEY",
    b"BEGIN DSA PRIVATE KEY",
    b"BEGIN EC PRIVATE KEY",
    b"BEGIN OPENSSH PRIVATE KEY",
    b"BEGIN PRIVATE KEY",
    b"PuTTY-User-Key-File-2",
    b"BEGIN SSH2 ENCRYPTED PRIVATE KEY",
    b"BEGIN PGP PRIVATE KEY BLOCK",
    b"BEGIN ENCRYPTED PRIVATE KEY",
    b"BEGIN OpenVPN Static key V1",
];

#[derive(Parser)]
#[command(about = "Detect the presence of private keys.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

/// Whether `haystack` contains `needle` as a byte subsequence.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("detect-private-key".to_owned()).chain(args.iter().cloned()),
    );

    let mut private_key_files = Vec::new();
    for filename in &args.filenames {
        let Ok(content) = std::fs::read(filename) else {
            continue;
        };
        if BLACKLIST.iter().any(|marker| contains(&content, marker)) {
            private_key_files.push(filename.clone());
        }
    }

    if private_key_files.is_empty() {
        0
    } else {
        for file in &private_key_files {
            println!("Private key found: {file}");
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(contents: &[u8]) -> i32 {
        let dir = std::env::temp_dir().join("detect-private-key-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f");
        std::fs::write(&path, contents).unwrap();
        run(&[path.to_string_lossy().into_owned()])
    }

    #[test]
    fn clean_file_passes() {
        assert_eq!(run_on(b"just some text\n"), 0);
    }

    #[test]
    fn detects_rsa_key() {
        assert_eq!(run_on(b"-----BEGIN RSA PRIVATE KEY-----\nabc\n"), 1);
    }

    #[test]
    fn detects_putty_key() {
        assert_eq!(run_on(b"PuTTY-User-Key-File-2: ssh-rsa\n"), 1);
    }

    #[test]
    fn detects_openvpn_key() {
        assert_eq!(run_on(b"-----BEGIN OpenVPN Static key V1-----\n"), 1);
    }

    #[test]
    fn contains_helper() {
        assert!(contains(b"abc", b""));
        assert!(contains(b"abcdef", b"cde"));
        assert!(!contains(b"abc", b"d"));
    }
}
