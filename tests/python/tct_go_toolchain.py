"""Unified pinned Go resolution for Python tests (shell resolver authority)."""

from __future__ import annotations

import json
import os
import subprocess
from functools import lru_cache
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
RESOLVER = REPO_ROOT / "scripts" / "testing" / "resolve-pinned-go.sh"


class GoToolchainError(RuntimeError):
    """Pinned Go could not be resolved."""


@lru_cache(maxsize=1)
def resolve_json() -> dict[str, Any]:
    if not RESOLVER.is_file():
        raise GoToolchainError(f"missing resolver: {RESOLVER}")
    completed = subprocess.run(
        [str(RESOLVER), "--json", "--repo-root", str(REPO_ROOT)],
        check=False,
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    if completed.returncode != 0:
        raise GoToolchainError(
            completed.stderr.strip() or completed.stdout.strip() or "resolve-pinned-go failed"
        )
    return json.loads(completed.stdout)


def export_into_os_environ() -> dict[str, Any]:
    """Export TCT_GO_BIN / TCT_GOFMT_BIN / PATH for the current process."""
    if not RESOLVER.is_file():
        raise GoToolchainError(f"missing resolver: {RESOLVER}")
    completed = subprocess.run(
        [str(RESOLVER), "--export", "--repo-root", str(REPO_ROOT)],
        check=False,
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    if completed.returncode != 0:
        raise GoToolchainError(
            completed.stderr.strip() or completed.stdout.strip() or "resolve-pinned-go --export failed"
        )
    for line in completed.stdout.splitlines():
        line = line.strip()
        if not line.startswith("export "):
            continue
        # export VAR='value'
        body = line[len("export ") :]
        if "=" not in body:
            continue
        key, _, raw = body.partition("=")
        value = raw.strip()
        if len(value) >= 2 and value[0] == value[-1] == "'":
            value = value[1:-1].replace("'\\''", "'")
        os.environ[key] = value
    return resolve_json()


def go_binary() -> str:
    """Return absolute path to pinned go (export env first if needed)."""
    configured = os.environ.get("TCT_GO_BIN")
    if configured and Path(configured).is_file():
        return configured
    info = export_into_os_environ()
    return str(info.get("export_go_bin") or info["go_bin"])
