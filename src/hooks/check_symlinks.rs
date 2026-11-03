//! `check-symlinks` — detect symlinks that do not point to anything.

use clap::Parser;

#[derive(Parser)]
#[command(about = "Check for broken symlinks.")]
struct Args {
    /// Filenames to check.
    filenames: Vec<String>,
}

pub fn run(args: &[String]) -> i32 {
    let args = Args::parse_from(std::iter::once("check-symlinks".to_owned()).chain(args.iter().cloned()));

    let mut retv = 0;
    for filename in &args.filenames {
        // `symlink_metadata` does not follow the link; `metadata` does.
        let is_link = std::fs::symlink_metadata(filename)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        let target_exists = std::fs::metadata(filename).is_ok();
        if is_link && !target_exists {
            println!("{filename}: Broken symlink");
            retv = 1;
        }
    }
    retv
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn broken_symlink_is_detected() {
        let path = crate::util::write_temp_file(b"x"); // real file
        let link = path.with_extension("link");
        let _ = std::fs::remove_file(&link);
        symlink(path.with_extension("does-not-exist"), &link).unwrap();
        assert_eq!(run(&[link.to_string_lossy().into_owned()]), 1);
        let _ = std::fs::remove_file(&link);
    }

    #[test]
    fn valid_symlink_passes() {
        let target = crate::util::write_temp_file(b"x");
        let link = target.with_extension("goodlink");
        let _ = std::fs::remove_file(&link);
        symlink(&target, &link).unwrap();
        assert_eq!(run(&[link.to_string_lossy().into_owned()]), 0);
        let _ = std::fs::remove_file(&link);
    }

    #[test]
    fn regular_file_passes() {
        let path = crate::util::write_temp_file(b"x");
        assert_eq!(run(&[path.to_string_lossy().into_owned()]), 0);
    }
}
