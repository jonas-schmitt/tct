//! Locating the TCT source tree that generated helper projects depend on.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::env::non_empty_os_str;

/// Directory of the TCT source tree that generated helper projects depend on.
///
/// Helper projects link the `tct` crate by path and reuse its `Cargo.lock` and
/// `rust-toolchain.toml`, so this has to be a real checkout. The first match wins:
///
/// 1. `TCT_SOURCE_DIR`, which must name a TCT checkout;
/// 2. the current directory when it is a TCT checkout, which is what the tools did
///    before this lookup existed, so the repository root and worktrees behave as
///    they always did;
/// 3. the directory this binary was built from, so the data tools also work from any
///    other working directory.
pub fn tct_source_dir() -> io::Result<PathBuf> {
    tct_source_dir_from(
        std::env::var_os("TCT_SOURCE_DIR").as_deref(),
        std::env::current_dir().ok().as_deref(),
        Some(Path::new(env!("CARGO_MANIFEST_DIR"))),
    )
}

fn tct_source_dir_from(
    explicit: Option<&OsStr>,
    current_dir: Option<&Path>,
    built_from: Option<&Path>,
) -> io::Result<PathBuf> {
    if let Some(dir) = non_empty_os_str(explicit) {
        let dir = Path::new(dir);
        return if is_tct_source_dir(dir) {
            Ok(dir.to_path_buf())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "TCT_SOURCE_DIR={} is not a TCT source directory: its Cargo.toml must \
                     declare package \"tct\"",
                    dir.display()
                ),
            ))
        };
    }
    [current_dir, built_from]
        .into_iter()
        .flatten()
        .find(|dir| is_tct_source_dir(dir))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "cannot find the TCT source directory that generated helper projects depend \
                 on; run from the TCT repository root or set TCT_SOURCE_DIR to it",
            )
        })
}

fn is_tct_source_dir(dir: &Path) -> bool {
    fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|manifest| manifest_declares_package(&manifest, "tct"))
}

/// True when the `[package]` table of `manifest` names `package`.
fn manifest_declares_package(manifest: &str, package: &str) -> bool {
    let mut in_package_table = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package_table = line == "[package]";
            continue;
        }
        if !in_package_table {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == "name"
        {
            let value = value.split('#').next().unwrap_or_default();
            return value.trim().trim_matches('"') == package;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_manifest(dir: &Path, package: &str) {
        fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn tct_source_dir_prefers_a_valid_explicit_override() {
        let explicit = tempdir().unwrap();
        let current = tempdir().unwrap();
        write_manifest(explicit.path(), "tct");
        write_manifest(current.path(), "tct");

        let found = tct_source_dir_from(
            Some(explicit.path().as_os_str()),
            Some(current.path()),
            Some(current.path()),
        )
        .unwrap();

        assert_eq!(found, explicit.path());
    }

    #[test]
    fn tct_source_dir_rejects_an_override_that_is_not_a_checkout() {
        let explicit = tempdir().unwrap();
        let current = tempdir().unwrap();
        write_manifest(explicit.path(), "tct-models");
        write_manifest(current.path(), "tct");

        let error = tct_source_dir_from(
            Some(explicit.path().as_os_str()),
            Some(current.path()),
            None,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("TCT_SOURCE_DIR"), "{error}");
    }

    #[test]
    fn tct_source_dir_keeps_using_the_current_directory_of_a_checkout() {
        let current = tempdir().unwrap();
        let built = tempdir().unwrap();
        write_manifest(current.path(), "tct");
        write_manifest(built.path(), "tct");

        let found = tct_source_dir_from(None, Some(current.path()), Some(built.path())).unwrap();

        assert_eq!(found, current.path());
    }

    #[test]
    fn tct_source_dir_falls_back_to_the_build_directory_elsewhere() {
        let elsewhere = tempdir().unwrap();
        let built = tempdir().unwrap();
        write_manifest(built.path(), "tct");

        let found = tct_source_dir_from(None, Some(elsewhere.path()), Some(built.path())).unwrap();

        assert_eq!(found, built.path());
    }

    #[test]
    fn tct_source_dir_ignores_an_empty_override() {
        let current = tempdir().unwrap();
        write_manifest(current.path(), "tct");

        let found = tct_source_dir_from(Some(OsStr::new("")), Some(current.path()), None).unwrap();

        assert_eq!(found, current.path());
    }

    #[test]
    fn tct_source_dir_error_says_what_to_do() {
        let elsewhere = tempdir().unwrap();

        let error = tct_source_dir_from(None, Some(elsewhere.path()), None).unwrap_err();

        let message = error.to_string();
        assert!(
            message.contains("repository root") && message.contains("TCT_SOURCE_DIR"),
            "{message}"
        );
    }

    #[test]
    fn manifest_declares_package_reads_only_the_package_table() {
        assert!(manifest_declares_package(
            "[package]\nname = \"tct\"\n",
            "tct"
        ));
        assert!(manifest_declares_package(
            "[package]\nversion = \"1\"\nname = \"tct\" # trailing comment\n",
            "tct"
        ));
        assert!(!manifest_declares_package(
            "[package]\nname = \"tct-models\"\n",
            "tct"
        ));
        assert!(!manifest_declares_package(
            "[package]\nversion = \"1\"\n[dependencies]\nname = \"tct\"\n",
            "tct"
        ));
        assert!(!manifest_declares_package(
            "[workspace]\nmembers = []\n",
            "tct"
        ));
    }

    #[test]
    fn this_repository_is_a_tct_source_dir() {
        // The directory a binary is built from has to pass the same check, or the
        // build-directory fallback could never succeed.
        assert!(is_tct_source_dir(Path::new(env!("CARGO_MANIFEST_DIR"))));
    }
}
