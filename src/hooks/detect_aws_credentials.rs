//! `detect-aws-credentials` — detect AWS secrets in the given files.
//!
//! Collects candidate secret access keys from the AWS CLI config files (and a
//! few environment variables), then scans each input file for those keys. Any
//! hit fails the hook.

use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Detect AWS credentials in the given files.")]
struct Args {
    /// Filenames to run.
    #[arg(required = true)]
    filenames: Vec<String>,

    /// Additional AWS credential file to read secrets from (repeatable).
    #[arg(long = "credentials-file")]
    credentials_file: Vec<String>,

    /// Allow the hook to pass when no credentials are detected.
    #[arg(long = "allow-missing-credentials")]
    allow_missing_credentials: bool,
}

/// Extra credential files named by environment variables.
fn cred_files_from_env() -> Vec<String> {
    ["AWS_CONFIG_FILE", "AWS_CREDENTIAL_FILE", "AWS_SHARED_CREDENTIALS_FILE", "BOTO_CONFIG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .collect()
}

/// AWS secrets exported as environment variables.
fn secrets_from_env() -> HashSet<String> {
    ["AWS_SECRET_ACCESS_KEY", "AWS_SECURITY_TOKEN", "AWS_SESSION_TOKEN"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Minimal INI parse: return all values whose key is one of the AWS secret keys.
///
/// Only the small subset `configparser` needs for these files is handled:
/// `[section]` headers, `key = value` / `key: value` pairs, and `#`/`;` comments.
fn secrets_from_file(path: &str) -> HashSet<String> {
    let expanded = expanduser(path);
    let Ok(text) = std::fs::read_to_string(&expanded) else {
        return HashSet::new();
    };
    let wanted = ["aws_secret_access_key", "aws_security_token", "aws_session_token"];
    let mut keys = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once(['=', ':']) else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if wanted.contains(&key.as_str()) {
            let value = value.trim();
            if !value.is_empty() {
                keys.insert(value.to_owned());
            }
        }
    }
    keys
}

/// Expand a leading `~` to the user's home directory.
fn expanduser(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn check_file_for_aws_keys(filenames: &[String], keys: &HashSet<String>) -> Vec<(String, String)> {
    let mut bad = Vec::new();
    for filename in filenames {
        let Ok(content) = std::fs::read(filename) else {
            continue;
        };
        for key in keys {
            if content.windows(key.len()).any(|w| w == key.as_bytes()) {
                // Obfuscate all but the first four characters for debugging.
                let prefix: String = key.chars().take(4).collect();
                let hidden = format!("{prefix}{}", "*".repeat(28 - prefix.len()));
                bad.push((filename.clone(), hidden));
            }
        }
    }
    bad
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(
        std::iter::once("detect-aws-credentials".to_owned()).chain(args.iter().cloned()),
    );

    let mut credential_files: HashSet<String> = [
        "~/.aws/config".to_owned(),
        "~/.aws/credentials".to_owned(),
        "/etc/boto.cfg".to_owned(),
        "~/.boto".to_owned(),
    ]
    .into();
    credential_files.extend(args.credentials_file.iter().cloned());
    credential_files.extend(cred_files_from_env());

    let mut keys: HashSet<String> = HashSet::new();
    for file in &credential_files {
        keys.extend(secrets_from_file(file));
    }
    keys.extend(secrets_from_env());

    if keys.is_empty() && args.allow_missing_credentials {
        return 0;
    }
    if keys.is_empty() {
        println!(
            "No AWS keys were found in the configured credential files and \
             environment variables.\nPlease ensure you have the correct setting \
             for --credentials-file"
        );
        return 2;
    }

    let bad = check_file_for_aws_keys(&args.filenames, &keys);
    if bad.is_empty() {
        0
    } else {
        for (filename, hidden) in &bad {
            println!("AWS secret found in {filename}: {hidden}");
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secret_in_file() {
        // Provide the secret through the environment so no real AWS files are read.
        let secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        // SAFETY: single-threaded test; setting a unique env var for the check.
        unsafe { std::env::set_var("AWS_SECRET_ACCESS_KEY", secret) };
        let path = crate::util::write_temp_file(secret.as_bytes());
        let bad = check_file_for_aws_keys(&[path.to_string_lossy().into_owned()], &std::iter::once(secret.to_owned()).collect());
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].1, "wJal************************");
        unsafe { std::env::remove_var("AWS_SECRET_ACCESS_KEY") };
    }

    #[test]
    fn clean_file_passes() {
        let secret = "someSecretValue123456";
        let path = crate::util::write_temp_file(b"nothing to see here");
        let bad = check_file_for_aws_keys(&[path.to_string_lossy().into_owned()], &std::iter::once(secret.to_owned()).collect());
        assert!(bad.is_empty());
    }

    #[test]
    fn expands_home() {
        // When HOME is unset, expanduser returns the literal path.
        unsafe { std::env::remove_var("HOME") };
        assert_eq!(expanduser("~/x").to_string_lossy(), "~/x");
    }
}
