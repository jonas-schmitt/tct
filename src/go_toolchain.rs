//! Unified pinned Go toolchain resolution.
//!
//! Pin version SoT: [`crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION`].
//! Algorithm contract (keep in lockstep with `scripts/testing/resolve-pinned-go.sh`):
//!
//! 1. `TCT_GO_BIN` if set — absolute operator intent; version must equal pin or fail.
//! 2. `PATH` `go` if `go version` equals pin.
//! 3. Auto-discover:
//!    - `$HOME/sdk/<pin>/bin/go` (preferred: binary named `go`, sibling `gofmt`)
//!    - `$HOME/go/bin/<pin>` (versioned binary name)
//! 4. Else fail closed.
//!
//! `gofmt`: `TCT_GOFMT_BIN` if set (must be executable and not the `go`
//! launcher), else `` `$("$go_bin" env GOROOT)/bin/gofmt` ``. Fail closed if
//! that path is missing or not executable. Version launchers
//! (`~/go/bin/go1.X.Y`) are not toolchains-with-gofmt; the real gofmt lives
//! under the launcher's GOROOT. Never treat a sibling of a versioned launcher
//! (or the launcher itself) as gofmt.
//!
//! See `docs/frontends-development.md` ("Pinned Go Toolchain Resolution").

use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Pinned Go version string (`goX.Y.Z`), re-exported for call sites.
pub const PINNED_GO_VERSION: &str = GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION;

/// How the toolchain binary was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoResolveSource {
    /// Explicit `TCT_GO_BIN`.
    Env,
    /// `PATH` `go` already reports the pin.
    Path,
    /// `$HOME/sdk/<pin>/bin/go`.
    SdkHome,
    /// `$HOME/go/bin/<pin>`.
    VersionedHome,
}

impl GoResolveSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Path => "path",
            Self::SdkHome => "sdk",
            Self::VersionedHome => "versioned_home",
        }
    }
}

impl fmt::Display for GoResolveSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resolved pin-matching Go toolchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGoToolchain {
    pub go_bin: PathBuf,
    pub gofmt_bin: PathBuf,
    pub version: String,
    pub source: GoResolveSource,
}

/// Resolution failure with actionable context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoToolchainError {
    pub message: String,
}

impl fmt::Display for GoToolchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GoToolchainError {}

impl GoToolchainError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Resolve the pinned Go toolchain (no process env mutation).
pub fn resolve() -> Result<ResolvedGoToolchain, GoToolchainError> {
    resolve_with_env(EnvSnapshot::from_process())
}

/// Ensure `TCT_GO_BIN`, `TCT_GOFMT_BIN`, and `PATH` point at the pin.
///
/// Safe to call repeatedly; applies once per process via [`OnceLock`].
/// Creates a repo-local shim under `target/tct-pinned-go/bin` when the
/// resolved binary is not named `go` (versioned home install).
pub fn ensure_process_env() -> Result<&'static ResolvedGoToolchain, GoToolchainError> {
    static CACHED: OnceLock<Result<ResolvedGoToolchain, GoToolchainError>> = OnceLock::new();
    let result = CACHED.get_or_init(|| {
        let resolved = resolve()?;
        apply_to_process_env(&resolved)?;
        Ok(resolved)
    });
    match result {
        Ok(tc) => Ok(tc),
        Err(err) => Err(err.clone()),
    }
}

/// Drop-in: resolved `go` path as [`OsString`] (runs full resolve each call
/// unless env already correct — prefer [`ensure_process_env`] for hot paths).
pub fn go_bin_os_string() -> Result<OsString, GoToolchainError> {
    Ok(resolve()?.go_bin.into_os_string())
}

/// Drop-in: resolved `gofmt` path as [`OsString`].
pub fn gofmt_bin_os_string() -> Result<OsString, GoToolchainError> {
    Ok(resolve()?.gofmt_bin.into_os_string())
}

/// Drop-in: resolved `go` path as [`String`].
pub fn go_bin_string() -> Result<String, GoToolchainError> {
    Ok(resolve()?.go_bin.display().to_string())
}

/// Parse `go version go1.24.3 linux/amd64` → `go1.24.3`.
pub fn parse_go_version_output(stdout: &str) -> Option<String> {
    let line = stdout.lines().next().unwrap_or(stdout).trim();
    // "go version go1.24.3 linux/amd64"
    let mut parts = line.split_whitespace();
    if parts.next()? != "go" {
        return None;
    }
    if parts.next()? != "version" {
        return None;
    }
    let ver = parts.next()?;
    if ver.starts_with("go") && ver.len() > 2 {
        Some(ver.to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct EnvSnapshot {
    tct_go_bin: Option<PathBuf>,
    tct_gofmt_bin: Option<PathBuf>,
    home: Option<PathBuf>,
    path: Option<OsString>,
}

impl EnvSnapshot {
    fn from_process() -> Self {
        Self {
            tct_go_bin: env::var_os("TCT_GO_BIN").map(PathBuf::from),
            tct_gofmt_bin: env::var_os("TCT_GOFMT_BIN").map(PathBuf::from),
            home: env::var_os("HOME").map(PathBuf::from),
            path: env::var_os("PATH"),
        }
    }
}

fn resolve_with_env(snap: EnvSnapshot) -> Result<ResolvedGoToolchain, GoToolchainError> {
    let pin = PINNED_GO_VERSION;
    let (go_bin, source) = select_go_bin(&snap, pin)?;
    let version = go_version_of(&go_bin)?;
    if version != pin {
        return Err(GoToolchainError::new(format!(
            "Go toolchain version mismatch: expected {pin}, got {version} from {} ({source})",
            go_bin.display()
        )));
    }
    let gofmt_bin = select_gofmt_bin(&snap, pin, &go_bin)?;
    Ok(ResolvedGoToolchain {
        go_bin,
        gofmt_bin,
        version,
        source,
    })
}

fn select_go_bin(
    snap: &EnvSnapshot,
    pin: &str,
) -> Result<(PathBuf, GoResolveSource), GoToolchainError> {
    // 1. Explicit TCT_GO_BIN
    if let Some(ref configured) = snap.tct_go_bin {
        if !is_runnable(configured) {
            return Err(GoToolchainError::new(format!(
                "TCT_GO_BIN is set to {} but is not an executable file",
                configured.display()
            )));
        }
        return Ok((configured.clone(), GoResolveSource::Env));
    }

    // 2. PATH go if pin matches
    if let Some(path_go) = which_in_path("go", snap.path.as_deref())
        && let Ok(ver) = go_version_of(&path_go)
        && ver == pin
    {
        return Ok((path_go, GoResolveSource::Path));
    }

    // 3. Auto-discover
    if let Some(ref home) = snap.home {
        let sdk = home.join("sdk").join(pin).join("bin").join("go");
        if is_runnable(&sdk)
            && let Ok(ver) = go_version_of(&sdk)
            && ver == pin
        {
            return Ok((sdk, GoResolveSource::SdkHome));
        }
        let versioned = home.join("go").join("bin").join(pin);
        if is_runnable(&versioned)
            && let Ok(ver) = go_version_of(&versioned)
            && ver == pin
        {
            return Ok((versioned, GoResolveSource::VersionedHome));
        }
    }

    Err(GoToolchainError::new(format!(
        "pinned Go {pin} not found.\n\
         Checked: TCT_GO_BIN (unset), PATH go (missing or wrong version), \
         $HOME/sdk/{pin}/bin/go, $HOME/go/bin/{pin}.\n\
         Remedy:\n\
           export PATH=\"$HOME/sdk/{pin}/bin:$PATH\"\n\
           export TCT_GO_BIN=\"$HOME/sdk/{pin}/bin/go\"\n\
           export TCT_GOFMT_BIN=\"$HOME/sdk/{pin}/bin/gofmt\"\n\
         Or: eval \"$(./scripts/testing/resolve-pinned-go.sh --export)\""
    )))
}

fn select_gofmt_bin(
    snap: &EnvSnapshot,
    pin: &str,
    go_bin: &Path,
) -> Result<PathBuf, GoToolchainError> {
    let gofmt_bin = if let Some(ref configured) = snap.tct_gofmt_bin {
        if !is_runnable(configured) {
            return Err(GoToolchainError::new(format!(
                "TCT_GOFMT_BIN is set to {} but is not an executable file",
                configured.display()
            )));
        }
        configured.clone()
    } else {
        gofmt_from_goroot(go_bin, pin)?
    };
    assert_gofmt_is_real(go_bin, &gofmt_bin)?;
    Ok(gofmt_bin)
}

/// Resolve `$("$go_bin" env GOROOT)/bin/gofmt` — the real gofmt for both
/// version launchers and ordinary toolchains.
fn gofmt_from_goroot(go_bin: &Path, pin: &str) -> Result<PathBuf, GoToolchainError> {
    let goroot = go_env_value(go_bin, "GOROOT")?;
    if goroot.is_empty() {
        return Err(GoToolchainError::new(format!(
            "gofmt not found for pinned Go {pin}: `{} env GOROOT` returned empty.\n\
             Set TCT_GOFMT_BIN to the real gofmt for this toolchain.",
            go_bin.display()
        )));
    }
    let candidate = PathBuf::from(&goroot).join("bin").join("gofmt");
    if is_runnable(&candidate) {
        return Ok(candidate);
    }
    Err(GoToolchainError::new(format!(
        "gofmt not found for pinned Go {pin}.\n\
         Resolved go: {}\n\
         Queried:     {} env GOROOT -> {goroot}\n\
         Expected:    {{GOROOT}}/bin/gofmt to be an executable file.\n\
         Set TCT_GOFMT_BIN to the real gofmt for this toolchain, or install the \
         full SDK so GOROOT/bin/gofmt exists (version launchers alone are not enough).",
        go_bin.display(),
        go_bin.display()
    )))
}

/// Reject a gofmt path that is the go launcher or does not print gofmt usage.
fn assert_gofmt_is_real(go_bin: &Path, gofmt_bin: &Path) -> Result<(), GoToolchainError> {
    if !is_runnable(gofmt_bin) {
        return Err(GoToolchainError::new(format!(
            "gofmt is not an executable file: {}\n\
             Resolved go: {}\n\
             Expected real gofmt at \"$(go env GOROOT)/bin/gofmt\".",
            gofmt_bin.display(),
            go_bin.display()
        )));
    }

    if let (Ok(go_real), Ok(gofmt_real)) = (fs::canonicalize(go_bin), fs::canonicalize(gofmt_bin))
        && go_real == gofmt_real
    {
        return Err(GoToolchainError::new(format!(
            "resolved gofmt is the same binary as go (launcher mislink).\n\
             go:    {} -> {}\n\
             gofmt: {} -> {}\n\
             Use $GOROOT/bin/gofmt for this toolchain, not the version launcher.",
            go_bin.display(),
            go_real.display(),
            gofmt_bin.display(),
            gofmt_real.display()
        )));
    }

    // Behavioral smoke: real gofmt prints "usage: gofmt ..." on -h; the go
    // launcher rejects -h/-l with "flag provided but not defined".
    let output = Command::new(gofmt_bin).arg("-h").output().map_err(|err| {
        GoToolchainError::new(format!(
            "failed to run `{} -h` while validating gofmt: {err}",
            gofmt_bin.display()
        ))
    })?;
    let help = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !help.to_ascii_lowercase().contains("gofmt") {
        return Err(GoToolchainError::new(format!(
            "resolved gofmt does not behave as gofmt: {}\n\
             `gofmt -h` output did not mention gofmt:\n{}",
            gofmt_bin.display(),
            help.lines()
                .take(8)
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )));
    }
    Ok(())
}

fn go_env_value(go_bin: &Path, key: &str) -> Result<String, GoToolchainError> {
    let output = Command::new(go_bin)
        .args(["env", key])
        .output()
        .map_err(|err| {
            GoToolchainError::new(format!(
                "failed to run `{} env {key}`: {err}",
                go_bin.display()
            ))
        })?;
    if !output.status.success() {
        return Err(GoToolchainError::new(format!(
            "`{} env {key}` failed (status {}): {}",
            go_bin.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn go_version_of(go_bin: &Path) -> Result<String, GoToolchainError> {
    let output = Command::new(go_bin)
        .arg("version")
        .output()
        .map_err(|err| {
            GoToolchainError::new(format!(
                "failed to run `{} version`: {err}",
                go_bin.display()
            ))
        })?;
    if !output.status.success() {
        return Err(GoToolchainError::new(format!(
            "`{} version` failed (status {}): {}",
            go_bin.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_go_version_output(&stdout).ok_or_else(|| {
        GoToolchainError::new(format!(
            "could not parse go version from `{} version` output: {stdout}",
            go_bin.display()
        ))
    })
}

fn is_runnable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn which_in_path(name: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let path_var = path_var?;
    for dir in env::split_paths(path_var) {
        let candidate = dir.join(name);
        if is_runnable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn apply_to_process_env(resolved: &ResolvedGoToolchain) -> Result<(), GoToolchainError> {
    let (export_go, export_gofmt, path_prefix) = prepare_export_paths(resolved, None)?;
    let prefix = path_prefix
        .or_else(|| export_go.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| GoToolchainError::new("resolved go binary has no parent directory"))?;
    // SAFETY: called once from OnceLock init during controlled toolchain setup;
    // not concurrent with other set_var in this process init path.
    unsafe {
        env::set_var("TCT_GO_BIN", &export_go);
        env::set_var("TCT_GOFMT_BIN", &export_gofmt);
        let mut new_path = prefix.into_os_string();
        if let Some(existing) = env::var_os("PATH") {
            new_path.push(":");
            new_path.push(existing);
        }
        env::set_var("PATH", new_path);
    }
    Ok(())
}

/// Prepare paths suitable for export (shim if binary is not named `go`).
///
/// Returns `(export_go, export_gofmt, path_prefix)`. `export_gofmt` is always
/// the real gofmt path (never a launcher). When a shim dir is created, PATH
/// gets a `gofmt` name linking to that real gofmt so bare `gofmt` works.
fn prepare_export_paths(
    resolved: &ResolvedGoToolchain,
    repo_root: Option<&Path>,
) -> Result<(PathBuf, PathBuf, Option<PathBuf>), GoToolchainError> {
    let go_name = resolved
        .go_bin
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if go_name == "go" {
        // SDK or PATH layout: export as-is; PATH prefix is parent dir.
        return Ok((
            resolved.go_bin.clone(),
            resolved.gofmt_bin.clone(),
            resolved.go_bin.parent().map(|p| p.to_path_buf()),
        ));
    }

    // Versioned binary name: create shim so bare `go` / `gofmt` work on PATH.
    let root = repo_root
        .map(|p| p.to_path_buf())
        .or_else(find_repo_root)
        .ok_or_else(|| {
            GoToolchainError::new(
                "cannot create pinned-go shim: repo root not found (no .git/Cargo.toml walk)",
            )
        })?;
    let shim_dir = root.join("target").join("tct-pinned-go").join("bin");
    fs::create_dir_all(&shim_dir).map_err(|err| {
        GoToolchainError::new(format!(
            "failed to create shim dir {}: {err}",
            shim_dir.display()
        ))
    })?;
    let shim_go = shim_dir.join("go");
    let shim_gofmt = shim_dir.join("gofmt");
    write_symlink(&resolved.go_bin, &shim_go)?;
    // Always link gofmt to the real GOROOT gofmt, never to the go launcher.
    write_symlink(&resolved.gofmt_bin, &shim_gofmt)?;
    assert_gofmt_is_real(&shim_go, &shim_gofmt)?;
    // TCT_GOFMT_BIN stays the real gofmt path (not the shim indirection).
    Ok((shim_go, resolved.gofmt_bin.clone(), Some(shim_dir)))
}

fn write_symlink(target: &Path, link: &Path) -> Result<(), GoToolchainError> {
    if link.exists() || link.symlink_metadata().is_ok() {
        let _ = fs::remove_file(link);
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|err| {
            GoToolchainError::new(format!(
                "failed to symlink {} -> {}: {err}",
                link.display(),
                target.display()
            ))
        })?;
    }
    #[cfg(not(unix))]
    {
        fs::copy(target, link).map_err(|err| {
            GoToolchainError::new(format!(
                "failed to copy {} -> {}: {err}",
                target.display(),
                link.display()
            ))
        })?;
    }
    Ok(())
}

fn find_repo_root() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        if dir.join("Cargo.toml").is_file()
            && dir.join("src").join("go_ast_schema_inventory.rs").is_file()
        {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;

    /// Serialize env-mutating tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Stub `go` that answers `version` and `env GOROOT` (GOROOT defaults to
    /// the parent of `bin/` when the stub lives at `.../bin/go`).
    fn write_stub_go(path: &Path, version: &str) {
        write_stub_go_with_goroot(path, version, None);
    }

    fn write_stub_go_with_goroot(path: &Path, version: &str, goroot: Option<&Path>) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        // Default GOROOT: grandparent of the binary when laid out as GOROOT/bin/go.
        let default_goroot = path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/nonexistent-goroot".to_string());
        let goroot_s = goroot
            .map(|p| p.display().to_string())
            .unwrap_or(default_goroot);
        let mut f = fs::File::create(path).unwrap();
        writeln!(
            f,
            "#!/usr/bin/env bash\n\
             if [[ \"$1\" == \"version\" ]]; then echo 'go version {version} linux/amd64'; exit 0; fi\n\
             if [[ \"$1\" == \"env\" && \"$2\" == \"GOROOT\" ]]; then echo '{goroot_s}'; exit 0; fi\n\
             exit 1"
        )
        .unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    fn write_stub_gofmt(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        // Must mention "gofmt" on -h so assert_gofmt_is_real accepts it.
        writeln!(
            f,
            "#!/usr/bin/env bash\n\
             if [[ \"$1\" == \"-h\" || \"$1\" == \"-help\" || \"$1\" == \"--help\" ]]; then\n\
               echo 'usage: gofmt [flags] [path ...]' >&2\n\
               exit 0\n\
             fi\n\
             # Accept -l like real gofmt (no-op success for smoke tests).\n\
             if [[ \"$1\" == \"-l\" ]]; then exit 0; fi\n\
             exit 0"
        )
        .unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    /// Version-launcher layout: `~/go/bin/go1.X.Y` with GOROOT at a separate
    /// SDK tree containing the real gofmt (no sibling gofmt next to launcher).
    fn write_versioned_launcher_layout(home: &Path, version: &str) -> (PathBuf, PathBuf) {
        let launcher = home.join("go").join("bin").join(version);
        let goroot = home.join("sdk").join(version);
        let gofmt = goroot.join("bin").join("gofmt");
        write_stub_go_with_goroot(&launcher, version, Some(&goroot));
        write_stub_gofmt(&gofmt);
        (launcher, gofmt)
    }

    #[test]
    fn parse_go_version_output_accepts_standard_line() {
        assert_eq!(
            parse_go_version_output("go version go1.24.3 linux/arm64\n"),
            Some("go1.24.3".to_string())
        );
    }

    #[test]
    fn pin_constant_matches_inventory() {
        assert_eq!(
            PINNED_GO_VERSION,
            crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION
        );
        assert!(PINNED_GO_VERSION.starts_with("go"));
    }

    #[test]
    fn resolve_prefers_explicit_tct_go_bin() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let goroot = tmp.path().join("goroot");
        let go = tmp.path().join("custom-go");
        let gofmt = goroot.join("bin").join("gofmt");
        write_stub_go_with_goroot(&go, PINNED_GO_VERSION, Some(&goroot));
        write_stub_gofmt(&gofmt);

        let snap = EnvSnapshot {
            tct_go_bin: Some(go.clone()),
            tct_gofmt_bin: Some(gofmt.clone()),
            home: Some(tmp.path().join("no-home")),
            path: Some(OsString::from("/nonexistent")),
        };
        let resolved = resolve_with_env(snap).unwrap();
        assert_eq!(resolved.go_bin, go);
        assert_eq!(resolved.gofmt_bin, gofmt);
        assert_eq!(resolved.source, GoResolveSource::Env);
        assert_eq!(resolved.version, PINNED_GO_VERSION);
    }

    #[test]
    fn resolve_rejects_wrong_explicit_version() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let goroot = tmp.path().join("goroot");
        let go = tmp.path().join("wrong-go");
        let gofmt = goroot.join("bin").join("gofmt");
        write_stub_go_with_goroot(&go, "go9.9.9", Some(&goroot));
        write_stub_gofmt(&gofmt);

        let snap = EnvSnapshot {
            tct_go_bin: Some(go),
            tct_gofmt_bin: Some(gofmt),
            home: None,
            path: None,
        };
        let err = resolve_with_env(snap).unwrap_err();
        assert!(err.message.contains("mismatch"), "{err}");
        assert!(err.message.contains("go9.9.9"), "{err}");
    }

    #[test]
    fn resolve_auto_discovers_sdk_home_over_wrong_path_go() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let path_dir = tmp.path().join("badpath");
        fs::create_dir_all(&path_dir).unwrap();

        let bad_go = path_dir.join("go");
        write_stub_go(&bad_go, "go1.22.2");

        let sdk_go = home
            .join("sdk")
            .join(PINNED_GO_VERSION)
            .join("bin")
            .join("go");
        let sdk_gofmt = home
            .join("sdk")
            .join(PINNED_GO_VERSION)
            .join("bin")
            .join("gofmt");
        write_stub_go(&sdk_go, PINNED_GO_VERSION);
        write_stub_gofmt(&sdk_gofmt);

        let snap = EnvSnapshot {
            tct_go_bin: None,
            tct_gofmt_bin: None,
            home: Some(home),
            path: Some(path_dir.into_os_string()),
        };
        let resolved = resolve_with_env(snap).unwrap();
        assert_eq!(resolved.source, GoResolveSource::SdkHome);
        assert_eq!(resolved.go_bin, sdk_go);
        assert_eq!(resolved.gofmt_bin, sdk_gofmt);
    }

    #[test]
    fn resolve_uses_path_when_version_matches() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // Ordinary toolchain layout: go and gofmt under the same GOROOT/bin.
        let goroot = tmp.path().join("goroot");
        let path_dir = goroot.join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        let path_go = path_dir.join("go");
        let path_gofmt = path_dir.join("gofmt");
        write_stub_go_with_goroot(&path_go, PINNED_GO_VERSION, Some(&goroot));
        write_stub_gofmt(&path_gofmt);

        let snap = EnvSnapshot {
            tct_go_bin: None,
            tct_gofmt_bin: None,
            home: Some(tmp.path().join("empty-home")),
            path: Some(path_dir.into_os_string()),
        };
        let resolved = resolve_with_env(snap).unwrap();
        assert_eq!(resolved.source, GoResolveSource::Path);
        assert_eq!(resolved.go_bin, path_go);
        assert_eq!(resolved.gofmt_bin, path_gofmt);
    }

    #[test]
    fn resolve_versioned_launcher_uses_goroot_gofmt_not_launcher() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let (launcher, gofmt) = write_versioned_launcher_layout(&home, PINNED_GO_VERSION);

        let snap = EnvSnapshot {
            tct_go_bin: Some(launcher.clone()),
            tct_gofmt_bin: None,
            home: Some(home),
            path: Some(OsString::from("/nonexistent")),
        };
        let resolved = resolve_with_env(snap).unwrap();
        assert_eq!(resolved.go_bin, launcher);
        assert_eq!(resolved.gofmt_bin, gofmt);
        assert_ne!(
            fs::canonicalize(&resolved.go_bin).unwrap(),
            fs::canonicalize(&resolved.gofmt_bin).unwrap(),
            "gofmt must not be the version launcher"
        );
        // Behavioral: resolved gofmt accepts -l (launcher would reject it).
        let out = Command::new(&resolved.gofmt_bin)
            .arg("-l")
            .output()
            .expect("gofmt -l should run");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.contains("flag provided but not defined"),
            "gofmt must not be the go launcher: {combined}"
        );
        assert!(out.status.success(), "stub gofmt -l should succeed");
    }

    #[test]
    fn resolve_rejects_gofmt_that_is_the_go_launcher() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let go = tmp.path().join("go-launcher");
        write_stub_go_with_goroot(&go, PINNED_GO_VERSION, Some(tmp.path()));
        // Deliberately point TCT_GOFMT_BIN at the same launcher binary.
        let snap = EnvSnapshot {
            tct_go_bin: Some(go.clone()),
            tct_gofmt_bin: Some(go),
            home: None,
            path: None,
        };
        let err = resolve_with_env(snap).unwrap_err();
        assert!(
            err.message.contains("same binary") || err.message.contains("does not behave"),
            "{err}"
        );
    }

    #[test]
    fn resolve_fails_closed_when_goroot_gofmt_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let goroot = tmp.path().join("empty-goroot");
        fs::create_dir_all(goroot.join("bin")).unwrap();
        let go = tmp.path().join("go-no-gofmt");
        write_stub_go_with_goroot(&go, PINNED_GO_VERSION, Some(&goroot));
        // No gofmt under GOROOT/bin.

        let snap = EnvSnapshot {
            tct_go_bin: Some(go),
            tct_gofmt_bin: None,
            home: Some(tmp.path().join("empty-home")),
            path: Some(OsString::from("/nonexistent")),
        };
        let err = resolve_with_env(snap).unwrap_err();
        assert!(err.message.contains("gofmt not found"), "{err}");
        assert!(err.message.contains("GOROOT"), "{err}");
    }

    #[test]
    fn prepare_export_paths_keeps_real_gofmt_for_versioned_launcher() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let (launcher, gofmt) = write_versioned_launcher_layout(&home, PINNED_GO_VERSION);
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("Cargo.toml"), "[package]\nname=\"t\"\n").unwrap();
        fs::write(
            repo.join("src").join("go_ast_schema_inventory.rs"),
            "pub const GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION: &str = \"go1.24.3\";\n",
        )
        .unwrap();

        let resolved = ResolvedGoToolchain {
            go_bin: launcher,
            gofmt_bin: gofmt.clone(),
            version: PINNED_GO_VERSION.to_string(),
            source: GoResolveSource::Env,
        };
        let (export_go, export_gofmt, path_prefix) =
            prepare_export_paths(&resolved, Some(&repo)).unwrap();
        assert_eq!(
            export_go.file_name().and_then(|s| s.to_str()),
            Some("go"),
            "export go should be a shim named go"
        );
        assert_eq!(
            export_gofmt, gofmt,
            "TCT_GOFMT_BIN must be the real gofmt, not a launcher shim"
        );
        let shim_gofmt = path_prefix.as_ref().unwrap().join("gofmt");
        assert!(shim_gofmt.exists() || shim_gofmt.symlink_metadata().is_ok());
        let target = fs::canonicalize(&shim_gofmt).unwrap();
        assert_eq!(target, fs::canonicalize(&gofmt).unwrap());
    }

    #[test]
    fn resolve_fails_closed_when_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let snap = EnvSnapshot {
            tct_go_bin: None,
            tct_gofmt_bin: None,
            home: Some(tmp.path().join("empty")),
            path: Some(OsString::from("/nonexistent-tct-go-path")),
        };
        let err = resolve_with_env(snap).unwrap_err();
        assert!(err.message.contains("not found"), "{err}");
        assert!(err.message.contains(PINNED_GO_VERSION), "{err}");
    }
}
