//! Temporary helper-project assembly for instance encoding.

use crate::generated_crate::{configure_generated_cargo_command, copy_workspace_cargo_lock};
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static ENCODER_PROJECT_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(super) struct BuiltGeneratedHelper {
    pub(super) project_dir: PathBuf,
    pub(super) encoder_bin: PathBuf,
}

pub(super) fn build_generated_helper(
    tokenizer_code: &str,
    root_type_name: &str,
    allocation_metrics_enabled: bool,
) -> Result<BuiltGeneratedHelper, Box<dyn Error>> {
    let tct_path = crate::generated_crate::tct_source_dir()?;

    let temp_dir = std::env::temp_dir();
    let project_id = unique_encoder_project_id();
    let project_dir = temp_dir.join(format!("tct_encoder_{}", project_id.replace('-', "_")));
    let helper_name = format!("tct-encoder-{project_id}");

    super::create_encoder_project(
        &project_dir,
        &helper_name,
        tokenizer_code,
        root_type_name,
        allocation_metrics_enabled,
    )?;
    copy_workspace_cargo_lock(&tct_path, &project_dir)?;

    let mut build_command = Command::new("cargo");
    configure_generated_cargo_command(&mut build_command, &tct_path);
    build_command
        .arg("build")
        .arg("--release")
        .arg("--quiet")
        .current_dir(&project_dir);
    let cargo_target_dir = command_env_os(&build_command, "CARGO_TARGET_DIR")
        .or_else(|| std::env::var_os("CARGO_TARGET_DIR"));
    let build_output = build_command.output()?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        return Err(generated_helper_build_failure_message(&project_dir, &stderr).into());
    }

    let encoder_bin = encoder_binary_path(&project_dir, &helper_name, cargo_target_dir);

    Ok(BuiltGeneratedHelper {
        project_dir,
        encoder_bin,
    })
}

fn generated_helper_build_failure_message(project_dir: &Path, stderr: &str) -> String {
    format!(
        "Failed to compile TCT tokenizer. Generated helper project preserved for inspection at {}. Cargo stderr:\n{}",
        project_dir.display(),
        stderr
    )
}

fn unique_encoder_project_id() -> String {
    let counter = ENCODER_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{counter}-{timestamp_nanos}", std::process::id())
}

fn command_env_os(command: &Command, name: &str) -> Option<OsString> {
    command
        .get_envs()
        .find_map(|(key, value)| (key == name).then(|| value.map(OsString::from)).flatten())
}

fn encoder_binary_path(
    project_dir: &Path,
    helper_name: &str,
    cargo_target_dir: Option<OsString>,
) -> PathBuf {
    let binary_name = format!("{}{}", helper_name, std::env::consts::EXE_SUFFIX);
    let target_dir = cargo_target_dir
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                project_dir.join(path)
            }
        })
        .unwrap_or_else(|| project_dir.join("target"));

    target_dir.join("release").join(binary_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_encoder_project_ids_do_not_repeat_in_process() {
        let first = unique_encoder_project_id();
        let second = unique_encoder_project_id();

        assert_ne!(first, second);
        assert!(first.starts_with(&format!("{}-", std::process::id())));
        assert!(second.starts_with(&format!("{}-", std::process::id())));
    }

    #[test]
    fn command_env_os_reads_configured_command_environment() {
        let mut command = Command::new("cargo");
        command.env("CARGO_TARGET_DIR", "/tmp/shared-target");

        assert_eq!(
            command_env_os(&command, "CARGO_TARGET_DIR"),
            Some(OsString::from("/tmp/shared-target"))
        );
        assert_eq!(command_env_os(&command, "RUSTUP_TOOLCHAIN"), None);
    }

    #[test]
    fn encoder_binary_path_honors_cargo_target_dir() {
        let project_dir = Path::new("/tmp/generated-project");
        let helper_name = "tct-encoder-123";
        let binary_name = format!("{}{}", helper_name, std::env::consts::EXE_SUFFIX);

        assert_eq!(
            encoder_binary_path(project_dir, helper_name, None),
            project_dir
                .join("target")
                .join("release")
                .join(&binary_name)
        );
        assert_eq!(
            encoder_binary_path(project_dir, helper_name, Some("/tmp/shared-target".into())),
            Path::new("/tmp/shared-target")
                .join("release")
                .join(&binary_name)
        );
        assert_eq!(
            encoder_binary_path(project_dir, helper_name, Some("relative-target".into())),
            project_dir
                .join("relative-target")
                .join("release")
                .join(&binary_name)
        );
    }

    #[test]
    fn build_failure_message_preserves_project_path_context() {
        let project_dir = tempfile::tempdir().expect("tempdir should be available");
        let project_path = project_dir.path().to_path_buf();

        let message =
            generated_helper_build_failure_message(&project_path, "expected compile error");

        assert!(message.contains("Failed to compile TCT tokenizer"));
        assert!(message.contains("Generated helper project preserved for inspection"));
        assert!(message.contains(&project_path.display().to_string()));
        assert!(message.contains("expected compile error"));
        assert!(
            project_path.exists(),
            "failure message creation must not remove the generated helper project"
        );
    }
}
