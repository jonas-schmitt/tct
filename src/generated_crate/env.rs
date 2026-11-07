//! Environment helpers for Cargo commands that build generated crates.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::GENERATED_CARGO_RUSTFLAGS;

const CARGO_ENCODED_RUSTFLAGS_SEPARATOR: &str = "\x1f";

pub fn generated_rustup_toolchain(tct_path: &Path) -> Option<String> {
    std::env::var("RUSTUP_TOOLCHAIN")
        .ok()
        .filter(|toolchain| !toolchain.trim().is_empty())
        .or_else(|| rust_toolchain_channel(tct_path))
}

pub fn configure_generated_cargo_command<'a>(
    command: &'a mut Command,
    tct_path: &Path,
) -> &'a mut Command {
    configure_generated_cargo_target_dir(
        command,
        tct_path,
        std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR").as_deref(),
        std::env::var_os("CARGO_TARGET_DIR").as_deref(),
    );
    configure_generated_rustflags(
        command,
        std::env::var_os("RUSTFLAGS").as_deref(),
        std::env::var_os("CARGO_ENCODED_RUSTFLAGS").as_deref(),
    );
    if let Some(toolchain) = generated_rustup_toolchain(tct_path) {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }
    command
}

/// Copy the workspace lockfile into a generated crate when it is available.
///
/// Generated crates are built outside the workspace, so Cargo will otherwise
/// resolve path-dependency transitive dependencies from the network even when
/// the workspace has a current lockfile.
pub fn copy_workspace_cargo_lock(tct_path: &Path, project_dir: &Path) -> std::io::Result<bool> {
    let cargo_lock = tct_path.join("Cargo.lock");
    if cargo_lock.is_file() {
        fs::copy(cargo_lock, project_dir.join("Cargo.lock"))?;
        return Ok(true);
    }
    Ok(false)
}

fn configure_generated_cargo_target_dir(
    command: &mut Command,
    tct_path: &Path,
    generated_target_dir: Option<&OsStr>,
    inherited_target_dir: Option<&OsStr>,
) {
    if let Some(target_dir) =
        generated_cargo_target_dir(tct_path, generated_target_dir, inherited_target_dir)
    {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
}

/// Resolve `CARGO_TARGET_DIR` for generated-crate Cargo/maturin invocations.
///
/// Priority: non-empty `generated_target_dir` (typically
/// `TCT_GENERATED_CARGO_TARGET_DIR`) wins; else leave ambient
/// `inherited_target_dir` alone (`None`); else default to
/// `{tct_path}/target/tct-generated-cargo`.
pub(crate) fn generated_cargo_target_dir(
    tct_path: &Path,
    generated_target_dir: Option<&OsStr>,
    inherited_target_dir: Option<&OsStr>,
) -> Option<OsString> {
    if let Some(target_dir) = non_empty_os_str(generated_target_dir) {
        return Some(target_dir.to_os_string());
    }
    if non_empty_os_str(inherited_target_dir).is_some() {
        return None;
    }
    Some(
        tct_path
            .join("target")
            .join("tct-generated-cargo")
            .into_os_string(),
    )
}

fn configure_generated_rustflags(
    command: &mut Command,
    rustflags: Option<&OsStr>,
    encoded_rustflags: Option<&OsStr>,
) {
    if let Some(encoded) = non_empty_os_str(encoded_rustflags) {
        command.env(
            "CARGO_ENCODED_RUSTFLAGS",
            append_encoded_rustflag(encoded, GENERATED_CARGO_RUSTFLAGS),
        );
        if let Some(flags) = non_empty_os_str(rustflags) {
            command.env(
                "RUSTFLAGS",
                append_space_separated_rustflag(Some(flags), GENERATED_CARGO_RUSTFLAGS),
            );
        }
    } else {
        command.env(
            "RUSTFLAGS",
            append_space_separated_rustflag(non_empty_os_str(rustflags), GENERATED_CARGO_RUSTFLAGS),
        );
    }
}

pub(super) fn non_empty_os_str(value: Option<&OsStr>) -> Option<&OsStr> {
    value.filter(|v| !v.to_string_lossy().is_empty())
}

fn append_space_separated_rustflag(existing: Option<&OsStr>, flag: &str) -> OsString {
    let Some(existing) = existing else {
        return OsString::from(flag);
    };
    let existing_text = existing.to_string_lossy();
    if existing_text.split_whitespace().any(|part| part == flag) {
        return existing.to_os_string();
    }

    let mut combined = existing.to_os_string();
    combined.push(" ");
    combined.push(flag);
    combined
}

fn append_encoded_rustflag(existing: &OsStr, flag: &str) -> OsString {
    let existing_text = existing.to_string_lossy();
    if existing_text
        .split(CARGO_ENCODED_RUSTFLAGS_SEPARATOR)
        .any(|part| part == flag)
    {
        return existing.to_os_string();
    }

    let mut combined = existing.to_os_string();
    combined.push(CARGO_ENCODED_RUSTFLAGS_SEPARATOR);
    combined.push(flag);
    combined
}

fn rust_toolchain_channel(tct_path: &Path) -> Option<String> {
    let content = fs::read_to_string(tct_path.join("rust-toolchain.toml")).ok()?;
    parse_rust_toolchain_channel(&content)
}

fn parse_rust_toolchain_channel(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "channel" {
            continue;
        }
        let channel = value.trim().trim_matches('"').trim();
        if !channel.is_empty() {
            return Some(channel.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_rust_toolchain_channel_reads_workspace_pin() {
        let content = r#"
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy"]
"#;

        assert_eq!(
            parse_rust_toolchain_channel(content),
            Some("1.95.0".to_string())
        );
        assert_eq!(parse_rust_toolchain_channel("[toolchain]"), None);
    }

    #[test]
    fn rust_toolchain_channel_reads_repo_file() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.95.0\"\n",
        )
        .unwrap();

        assert_eq!(
            rust_toolchain_channel(dir.path()),
            Some("1.95.0".to_string())
        );
    }

    #[test]
    fn configure_generated_cargo_command_sets_shared_environment() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.95.0\"\n",
        )
        .unwrap();

        let mut command = Command::new("cargo");
        configure_generated_cargo_command(&mut command, dir.path());

        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();

        assert!(
            rustflags_contain(
                envs.get("RUSTFLAGS").and_then(|value| value.as_deref()),
                GENERATED_CARGO_RUSTFLAGS
            ) || encoded_rustflags_contain(
                envs.get("CARGO_ENCODED_RUSTFLAGS")
                    .and_then(|value| value.as_deref()),
                GENERATED_CARGO_RUSTFLAGS
            ),
            "generated cargo command should apply the generated-crate lint cap"
        );
        assert!(
            envs.get("RUSTUP_TOOLCHAIN")
                .and_then(|value| value.as_deref())
                .is_some_and(|value| !value.trim().is_empty())
        );
        // Match production: same policy as configure_generated_cargo_command, so
        // the assertion stays green under plain cargo test and under quality-check
        // (which sets TCT_GENERATED_CARGO_TARGET_DIR for isolation).
        let expected_target_dir = generated_cargo_target_dir(
            dir.path(),
            std::env::var_os("TCT_GENERATED_CARGO_TARGET_DIR").as_deref(),
            std::env::var_os("CARGO_TARGET_DIR").as_deref(),
        );
        assert_eq!(
            envs.get("CARGO_TARGET_DIR")
                .and_then(|value| value.as_ref())
                .map(|value| OsString::from(value.as_str())),
            expected_target_dir
        );
    }

    #[test]
    fn copy_workspace_cargo_lock_copies_lockfile_when_available() {
        let workspace = tempdir().unwrap();
        let project = tempdir().unwrap();
        fs::write(workspace.path().join("Cargo.lock"), "# locked\n").unwrap();

        let copied = copy_workspace_cargo_lock(workspace.path(), project.path()).unwrap();

        assert!(copied);
        assert_eq!(
            fs::read_to_string(project.path().join("Cargo.lock")).unwrap(),
            "# locked\n"
        );
    }

    #[test]
    fn copy_workspace_cargo_lock_skips_missing_lockfile() {
        let workspace = tempdir().unwrap();
        let project = tempdir().unwrap();

        let copied = copy_workspace_cargo_lock(workspace.path(), project.path()).unwrap();

        assert!(!copied);
        assert!(!project.path().join("Cargo.lock").exists());
    }

    #[test]
    fn generated_cargo_target_dir_defaults_to_repo_target_cache() {
        let dir = tempdir().unwrap();

        assert_eq!(
            generated_cargo_target_dir(dir.path(), None, None),
            Some(
                dir.path()
                    .join("target")
                    .join("tct-generated-cargo")
                    .into_os_string()
            )
        );
    }

    #[test]
    fn generated_cargo_target_dir_uses_explicit_generated_cache() {
        let dir = tempdir().unwrap();

        assert_eq!(
            generated_cargo_target_dir(
                dir.path(),
                Some(OsStr::new("/tmp/generated-cache")),
                Some(OsStr::new("/tmp/inherited-cache")),
            ),
            Some(OsString::from("/tmp/generated-cache"))
        );
    }

    #[test]
    fn generated_cargo_target_dir_preserves_inherited_cache() {
        let dir = tempdir().unwrap();

        assert_eq!(
            generated_cargo_target_dir(dir.path(), None, Some(OsStr::new("/tmp/inherited-cache"))),
            None
        );
    }

    #[test]
    fn generated_rustflags_preserve_existing_space_separated_flags() {
        let combined = append_space_separated_rustflag(
            Some(OsStr::new("-Cinstrument-coverage -Clink-dead-code")),
            GENERATED_CARGO_RUSTFLAGS,
        );

        let combined = combined.to_string_lossy();
        assert!(combined.contains("-Cinstrument-coverage"));
        assert!(combined.contains("-Clink-dead-code"));
        assert!(combined.contains(GENERATED_CARGO_RUSTFLAGS));
    }

    #[test]
    fn generated_rustflags_preserve_existing_encoded_flags() {
        let combined = append_encoded_rustflag(
            OsStr::new("-Cinstrument-coverage\x1f-Clink-dead-code"),
            GENERATED_CARGO_RUSTFLAGS,
        );

        let combined = combined.to_string_lossy();
        let flags = combined
            .split(CARGO_ENCODED_RUSTFLAGS_SEPARATOR)
            .collect::<Vec<_>>();
        assert!(flags.contains(&"-Cinstrument-coverage"));
        assert!(flags.contains(&"-Clink-dead-code"));
        assert!(flags.contains(&GENERATED_CARGO_RUSTFLAGS));
    }

    #[test]
    fn configure_generated_rustflags_prefers_encoded_coverage_flags() {
        let mut command = Command::new("cargo");
        configure_generated_rustflags(
            &mut command,
            None,
            Some(OsStr::new("-Cinstrument-coverage\x1f-Clink-dead-code")),
        );
        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();

        assert!(
            !envs.contains_key("RUSTFLAGS"),
            "plain RUSTFLAGS should not be introduced when cargo-llvm-cov uses encoded flags"
        );
        let encoded = envs
            .get("CARGO_ENCODED_RUSTFLAGS")
            .and_then(|value| value.as_deref());
        assert!(encoded_rustflags_contain(encoded, "-Cinstrument-coverage"));
        assert!(encoded_rustflags_contain(encoded, "-Clink-dead-code"));
        assert!(encoded_rustflags_contain(
            encoded,
            GENERATED_CARGO_RUSTFLAGS
        ));
    }

    fn rustflags_contain(value: Option<&str>, flag: &str) -> bool {
        value
            .map(|value| value.split_whitespace().any(|part| part == flag))
            .unwrap_or(false)
    }

    fn encoded_rustflags_contain(value: Option<&str>, flag: &str) -> bool {
        value
            .map(|value| {
                value
                    .split(CARGO_ENCODED_RUSTFLAGS_SEPARATOR)
                    .any(|part| part == flag)
            })
            .unwrap_or(false)
    }
}
