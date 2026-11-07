#!/usr/bin/env bash
# Unified pinned Go toolchain resolver (shell authority for process env).
#
# Pin version SoT: GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION in
# src/go_ast_schema_inventory.rs (parsed here; no hardcoded pin string as the
# search key beyond what that file declares).
#
# Algorithm (keep in lockstep with src/go_toolchain.rs and
# docs/frontends-development.md "Pinned Go Toolchain Resolution"):
#
#   1. TCT_GO_BIN if set — absolute operator intent; version must equal pin.
#   2. PATH go if `go version` equals pin.
#   3. Auto-discover:
#        $HOME/sdk/<pin>/bin/go     (preferred: named go + sibling gofmt)
#        $HOME/go/bin/<pin>         (versioned binary name)
#   4. Else fail closed.
#
# gofmt: TCT_GOFMT_BIN if set (must be executable and not the go launcher),
# else `"$go_bin" env GOROOT` -> `$GOROOT/bin/gofmt`. Fail closed if that
# path is missing or not executable. Never link gofmt to the go launcher
# (versioned `go1.X.Y` installers are launchers, not toolchains-with-gofmt).
#
# When exporting, ensure PATH has a directory containing binaries named `go`
# and `gofmt` (shim under $REPO/target/tct-pinned-go/bin when needed). The
# gofmt shim always targets the real `$GOROOT/bin/gofmt`, and TCT_GOFMT_BIN
# is exported as that real path (or the operator override), never as a
# launcher.
#
# Usage:
#   ./scripts/testing/resolve-pinned-go.sh --export [--repo-root DIR]
#   ./scripts/testing/resolve-pinned-go.sh --print-go
#   ./scripts/testing/resolve-pinned-go.sh --print-gofmt
#   ./scripts/testing/resolve-pinned-go.sh --print-version
#   ./scripts/testing/resolve-pinned-go.sh --check
#   ./scripts/testing/resolve-pinned-go.sh --json
#   eval "$(./scripts/testing/resolve-pinned-go.sh --export)"
#
# Exit 0 on success; non-zero on failure (messages on stderr).

set -euo pipefail

MODE=""
REPO_ROOT=""

usage() {
  cat <<'EOF'
Usage: resolve-pinned-go.sh [mode] [--repo-root DIR]

Modes (exactly one):
  --export          Print shell export lines for TCT_GO_BIN, TCT_GOFMT_BIN, PATH
  --print-go        Print absolute path to resolved go
  --print-gofmt     Print absolute path to resolved gofmt
  --print-version   Print pin version string (goX.Y.Z)
  --check           Resolve and print a one-line PASS summary
  --json            Print JSON object with go_bin, gofmt_bin, version, source
  -h, --help        Show this help

Options:
  --repo-root DIR   Repository root (default: walk from script location)
EOF
}

while (($#)); do
  case "$1" in
    --export|--print-go|--print-gofmt|--print-version|--check|--json)
      if [[ -n "${MODE}" ]]; then
        echo "error: multiple modes specified" >&2
        exit 2
      fi
      MODE="${1#--}"
      ;;
    --repo-root)
      shift
      REPO_ROOT="${1:-}"
      if [[ -z "${REPO_ROOT}" ]]; then
        echo "error: --repo-root requires a directory" >&2
        exit 2
      fi
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [[ -z "${MODE}" ]]; then
  echo "error: mode required (--export, --print-go, --check, --json, ...)" >&2
  usage >&2
  exit 2
fi

script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "${REPO_ROOT}" ]]; then
  REPO_ROOT="$(CDPATH='' cd -- "${script_dir}/../.." && pwd)"
fi

pin_file="${REPO_ROOT}/src/go_ast_schema_inventory.rs"
if [[ ! -f "${pin_file}" ]]; then
  echo "error: pin source missing: ${pin_file}" >&2
  echo "  Expected GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION in that file." >&2
  exit 1
fi

PINNED="$(
  grep -E 'GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION' "${pin_file}" \
    | head -n 1 \
    | sed -n 's/.*"\(go[0-9][^"]*\)".*/\1/p'
)"
if [[ -z "${PINNED}" ]]; then
  echo "error: could not parse GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION from ${pin_file}" >&2
  exit 1
fi

is_runnable() {
  local p="$1"
  [[ -n "${p}" && -f "${p}" && -x "${p}" ]]
}

# Canonical path for same-file comparison (launcher vs real gofmt).
realpath_or() {
  local p="$1"
  if command -v realpath >/dev/null 2>&1; then
    realpath -- "${p}" 2>/dev/null || printf '%s\n' "${p}"
  else
    # readlink -f is GNU; fall back to the input path.
    readlink -f -- "${p}" 2>/dev/null || printf '%s\n' "${p}"
  fi
}

go_version_of() {
  local bin="$1"
  local out
  out="$("${bin}" version 2>/dev/null)" || return 1
  # "go version go1.24.3 linux/amd64" -> go1.24.3
  printf '%s\n' "${out}" | sed -n 's/^go version \(go[^ ]*\).*/\1/p' | head -n 1
}

# Real gofmt for a resolved go binary: `$("$go_bin" env GOROOT)/bin/gofmt`.
# Works for both version launchers (~/go/bin/go1.24.3 -> ~/sdk/go1.24.3)
# and ordinary toolchains (/usr/local/go/bin/go -> sibling gofmt under GOROOT).
gofmt_from_goroot() {
  local go_bin="$1"
  local goroot candidate
  goroot="$("${go_bin}" env GOROOT 2>/dev/null)" || return 1
  # Trim whitespace/newlines from `go env` output.
  goroot="$(printf '%s' "${goroot}" | tr -d '\r\n')"
  if [[ -z "${goroot}" ]]; then
    return 1
  fi
  candidate="${goroot}/bin/gofmt"
  if is_runnable "${candidate}"; then
    printf '%s\n' "${candidate}"
    return 0
  fi
  return 1
}

# Fail closed if the candidate is the go launcher (same realpath) or does not
# accept gofmt's -h usage banner. Catches silent "gofmt -> go" mislinks.
assert_gofmt_is_real() {
  local go_bin="$1"
  local gofmt_bin="$2"
  local go_real gofmt_real help_out

  if ! is_runnable "${gofmt_bin}"; then
    echo "error: gofmt is not an executable file: ${gofmt_bin}" >&2
    echo "  Resolved go: ${go_bin}" >&2
    echo "  Expected real gofmt at \"\$(${go_bin} env GOROOT)/bin/gofmt\"." >&2
    exit 1
  fi

  go_real="$(realpath_or "${go_bin}")"
  gofmt_real="$(realpath_or "${gofmt_bin}")"
  if [[ -n "${go_real}" && -n "${gofmt_real}" && "${go_real}" == "${gofmt_real}" ]]; then
    echo "error: resolved gofmt is the same binary as go (launcher mislink)." >&2
    echo "  go:    ${go_bin} -> ${go_real}" >&2
    echo "  gofmt: ${gofmt_bin} -> ${gofmt_real}" >&2
    echo "  Use \$GOROOT/bin/gofmt for this toolchain, not the version launcher." >&2
    exit 1
  fi

  # Behavioral smoke: real gofmt prints "usage: gofmt ..." on -h; the go
  # launcher rejects -h / -l with "flag provided but not defined".
  help_out="$("${gofmt_bin}" -h 2>&1 || true)"
  if ! printf '%s\n' "${help_out}" | grep -q 'gofmt'; then
    echo "error: resolved gofmt does not behave as gofmt: ${gofmt_bin}" >&2
    echo "  \`gofmt -h\` output did not mention gofmt:" >&2
    printf '%s\n' "${help_out}" | head -n 8 | sed 's/^/    /' >&2
    echo "  Refusing to export a broken gofmt (would break PATH bare \`gofmt\`)." >&2
    exit 1
  fi
}

which_go_on_path() {
  command -v go 2>/dev/null || true
}

fail_not_found() {
  cat >&2 <<EOF
error: pinned Go ${PINNED} not found.
Checked: TCT_GO_BIN (${TCT_GO_BIN:-unset}), PATH go (missing or wrong version),
  \$HOME/sdk/${PINNED}/bin/go, \$HOME/go/bin/${PINNED}.
Remedy:
  export PATH="\$HOME/sdk/${PINNED}/bin:\$PATH"
  export TCT_GO_BIN="\$HOME/sdk/${PINNED}/bin/go"
  export TCT_GOFMT_BIN="\$HOME/sdk/${PINNED}/bin/gofmt"
Or:
  eval "\$(${REPO_ROOT}/scripts/testing/resolve-pinned-go.sh --export)"
EOF
  exit 1
}

SOURCE=""
GO_BIN=""
GOFMT_BIN=""

# 1. Explicit TCT_GO_BIN
if [[ -n "${TCT_GO_BIN:-}" ]]; then
  if ! is_runnable "${TCT_GO_BIN}"; then
    echo "error: TCT_GO_BIN=${TCT_GO_BIN} is not an executable file" >&2
    exit 1
  fi
  GO_BIN="${TCT_GO_BIN}"
  SOURCE="env"
else
  # 2. PATH go if pin matches
  path_go="$(which_go_on_path)"
  if [[ -n "${path_go}" ]]; then
    path_ver="$(go_version_of "${path_go}" || true)"
    if [[ "${path_ver}" == "${PINNED}" ]]; then
      GO_BIN="${path_go}"
      SOURCE="path"
    fi
  fi

  # 3. Auto-discover
  if [[ -z "${GO_BIN}" && -n "${HOME:-}" ]]; then
    sdk_go="${HOME}/sdk/${PINNED}/bin/go"
    if is_runnable "${sdk_go}"; then
      sdk_ver="$(go_version_of "${sdk_go}" || true)"
      if [[ "${sdk_ver}" == "${PINNED}" ]]; then
        GO_BIN="${sdk_go}"
        SOURCE="sdk"
      fi
    fi
  fi
  if [[ -z "${GO_BIN}" && -n "${HOME:-}" ]]; then
    versioned="${HOME}/go/bin/${PINNED}"
    if is_runnable "${versioned}"; then
      ver="$(go_version_of "${versioned}" || true)"
      if [[ "${ver}" == "${PINNED}" ]]; then
        GO_BIN="${versioned}"
        SOURCE="versioned_home"
      fi
    fi
  fi
fi

if [[ -z "${GO_BIN}" ]]; then
  fail_not_found
fi

ACTUAL="$(go_version_of "${GO_BIN}" || true)"
if [[ "${ACTUAL}" != "${PINNED}" ]]; then
  cat >&2 <<EOF
error: Go toolchain version mismatch.
  expected: ${PINNED}
  actual:   ${ACTUAL:-<unparseable>}
  resolved: ${GO_BIN} (source=${SOURCE})
EOF
  exit 1
fi

# gofmt resolution: prefer operator override, else GOROOT/bin/gofmt from the
# resolved go binary. Never invent a sibling of a versioned launcher.
if [[ -n "${TCT_GOFMT_BIN:-}" ]]; then
  GOFMT_BIN="${TCT_GOFMT_BIN}"
else
  if ! GOFMT_BIN="$(gofmt_from_goroot "${GO_BIN}")"; then
    goroot_hint="$("${GO_BIN}" env GOROOT 2>/dev/null || true)"
    goroot_hint="$(printf '%s' "${goroot_hint}" | tr -d '\r\n')"
    cat >&2 <<EOF
error: gofmt not found for pinned Go ${PINNED}.
  Resolved go: ${GO_BIN} (source=${SOURCE})
  Queried:     ${GO_BIN} env GOROOT -> ${goroot_hint:-<empty>}
  Expected:    \${GOROOT}/bin/gofmt to be an executable file.
  Set TCT_GOFMT_BIN to the real gofmt for this toolchain, or install the
  full SDK so GOROOT/bin/gofmt exists (version launchers alone are not enough).
EOF
    exit 1
  fi
fi

assert_gofmt_is_real "${GO_BIN}" "${GOFMT_BIN}"

# Export paths: ensure bare `go` / `gofmt` names on PATH.
# TCT_GOFMT_BIN always exports the real gofmt path (never a launcher). The
# shim directory only provides a PATH entry named `gofmt` that links to it.
EXPORT_GO="${GO_BIN}"
EXPORT_GOFMT="${GOFMT_BIN}"
PATH_PREFIX="$(dirname -- "${GO_BIN}")"

go_base="$(basename -- "${GO_BIN}")"
if [[ "${go_base}" != "go" ]]; then
  shim_dir="${REPO_ROOT}/target/tct-pinned-go/bin"
  mkdir -p "${shim_dir}"
  ln -sfn "${GO_BIN}" "${shim_dir}/go"
  # Always link gofmt to the real GOROOT gofmt, never to the go launcher.
  ln -sfn "${GOFMT_BIN}" "${shim_dir}/gofmt"
  # Re-validate the shim target itself (guards against a bad prior link).
  assert_gofmt_is_real "${shim_dir}/go" "${shim_dir}/gofmt"
  EXPORT_GO="${shim_dir}/go"
  # Keep TCT_GOFMT_BIN as the real gofmt, not the shim path, so call sites
  # that exec TCT_GOFMT_BIN never depend on the shim indirection.
  EXPORT_GOFMT="${GOFMT_BIN}"
  PATH_PREFIX="${shim_dir}"
fi

shell_escape() {
  # Single-quote for eval-safe export values.
  local s="$1"
  printf "'%s'" "${s//\'/\'\\\'\'}"
}

case "${MODE}" in
  export)
    printf 'export TCT_GO_BIN=%s\n' "$(shell_escape "${EXPORT_GO}")"
    printf 'export TCT_GOFMT_BIN=%s\n' "$(shell_escape "${EXPORT_GOFMT}")"
    printf 'export PATH=%s\n' "$(shell_escape "${PATH_PREFIX}:${PATH:-}")"
    # Diagnostic on stderr so eval stays clean.
    echo "resolve-pinned-go: ${PINNED} via ${SOURCE} (${GO_BIN})" >&2
    ;;
  print-go)
    printf '%s\n' "${EXPORT_GO}"
    ;;
  print-gofmt)
    printf '%s\n' "${EXPORT_GOFMT}"
    ;;
  print-version)
    printf '%s\n' "${PINNED}"
    ;;
  check)
    echo "PASS: pinned Go ${PINNED} via ${SOURCE} (${GO_BIN})"
    ;;
  json)
    # Minimal JSON (paths unlikely to contain quotes in normal installs).
    python3 - <<PY
import json
print(json.dumps({
    "version": """${PINNED}""",
    "go_bin": """${GO_BIN}""",
    "export_go_bin": """${EXPORT_GO}""",
    "gofmt_bin": """${GOFMT_BIN}""",
    "export_gofmt_bin": """${EXPORT_GOFMT}""",
    "source": """${SOURCE}""",
    "path_prefix": """${PATH_PREFIX}""",
}, sort_keys=True))
PY
    ;;
  *)
    echo "error: internal: unknown mode ${MODE}" >&2
    exit 2
    ;;
esac
