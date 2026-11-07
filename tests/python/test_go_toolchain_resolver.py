#!/usr/bin/env python3
"""Parity and smoke tests for the unified pinned Go resolver."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
RESOLVER = REPO_ROOT / "scripts" / "testing" / "resolve-pinned-go.sh"
PIN_SOURCE = REPO_ROOT / "src" / "go_ast_schema_inventory.rs"
GO_TOOLCHAIN_RS = REPO_ROOT / "src" / "go_toolchain.rs"


def _pinned_from_source() -> str:
    text = PIN_SOURCE.read_text(encoding="utf-8")
    match = re.search(
        r'GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION[^=]*=\s*"(go[^"]+)"',
        text,
    )
    assert match, "pin const missing"
    return match.group(1)


def _versioned_launcher_candidates(pin: str) -> list[Path]:
    home = Path.home()
    return [
        home / "go" / "bin" / pin,
        Path(os.environ.get("HOME", str(home))) / "go" / "bin" / pin,
    ]


def _find_versioned_launcher(pin: str) -> Path | None:
    for candidate in _versioned_launcher_candidates(pin):
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


class TestResolvePinnedGoShell(unittest.TestCase):
    def test_resolver_script_exists_and_is_executable(self) -> None:
        self.assertTrue(RESOLVER.is_file())
        self.assertTrue(os.access(RESOLVER, os.X_OK))

    def test_check_reports_inventory_pin(self) -> None:
        pin = _pinned_from_source()
        env = {k: v for k, v in os.environ.items() if k not in {"TCT_GO_BIN", "TCT_GOFMT_BIN"}}
        completed = subprocess.run(
            [str(RESOLVER), "--check", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn(pin, completed.stdout)
        self.assertIn("PASS", completed.stdout)

    def test_json_version_matches_inventory_const(self) -> None:
        pin = _pinned_from_source()
        env = {k: v for k, v in os.environ.items() if k not in {"TCT_GO_BIN", "TCT_GOFMT_BIN"}}
        completed = subprocess.run(
            [str(RESOLVER), "--json", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        payload = json.loads(completed.stdout)
        self.assertEqual(payload["version"], pin)
        self.assertIn(payload["source"], {"env", "path", "sdk", "versioned_home"})
        self.assertTrue(Path(payload["go_bin"]).is_file())
        self.assertTrue(Path(payload["gofmt_bin"]).is_file())

    def test_rust_module_documents_same_algorithm(self) -> None:
        text = GO_TOOLCHAIN_RS.read_text(encoding="utf-8")
        self.assertIn("TCT_GO_BIN", text)
        self.assertIn("sdk/<pin>/bin/go", text)
        self.assertIn("$HOME/go/bin/<pin>", text)
        self.assertIn("resolve-pinned-go.sh", text)
        self.assertIn("env GOROOT", text)
        self.assertIn("GOROOT", text)

    def test_resolved_gofmt_behaves_as_gofmt_not_go_launcher(self) -> None:
        """gofmt must accept -l; the go launcher rejects it with a flag error."""
        pin = _pinned_from_source()
        env = {k: v for k, v in os.environ.items() if k not in {"TCT_GO_BIN", "TCT_GOFMT_BIN"}}
        launcher = _find_versioned_launcher(pin)
        if launcher is not None:
            env["TCT_GO_BIN"] = str(launcher)

        completed = subprocess.run(
            [str(RESOLVER), "--json", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        payload = json.loads(completed.stdout)
        gofmt_bin = Path(payload["gofmt_bin"])
        go_bin = Path(payload["go_bin"])
        self.assertTrue(gofmt_bin.is_file(), f"gofmt missing: {gofmt_bin}")

        # Realpaths must differ when a version launcher is in play.
        go_real = go_bin.resolve()
        gofmt_real = gofmt_bin.resolve()
        self.assertNotEqual(
            go_real,
            gofmt_real,
            f"gofmt must not be the go launcher: go={go_real} gofmt={gofmt_real}",
        )

        # Behavioral identity: gofmt -l is valid; go -l is not.
        gofmt_l = subprocess.run(
            [str(gofmt_bin), "-l"],
            check=False,
            capture_output=True,
            text=True,
            input="",
        )
        combined = (gofmt_l.stdout or "") + (gofmt_l.stderr or "")
        self.assertNotIn(
            "flag provided but not defined",
            combined,
            f"gofmt looks like the go launcher: {combined!r}",
        )
        self.assertNotIn(
            "unknown command",
            combined,
            f"gofmt treated a path as a go subcommand: {combined!r}",
        )

        help_out = subprocess.run(
            [str(gofmt_bin), "-h"],
            check=False,
            capture_output=True,
            text=True,
        )
        help_text = (help_out.stdout or "") + (help_out.stderr or "")
        self.assertIn("gofmt", help_text.lower(), help_text)

        # Export path: TCT_GOFMT_BIN must be the real gofmt (not a launcher).
        export = subprocess.run(
            [str(RESOLVER), "--export", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(export.returncode, 0, export.stderr)
        export_gofmt = None
        for line in export.stdout.splitlines():
            if line.startswith("export TCT_GOFMT_BIN="):
                raw = line.split("=", 1)[1].strip()
                if raw.startswith("'") and raw.endswith("'"):
                    raw = raw[1:-1]
                export_gofmt = Path(raw)
                break
        self.assertIsNotNone(export_gofmt, export.stdout)
        assert export_gofmt is not None
        export_l = subprocess.run(
            [str(export_gofmt), "-l"],
            check=False,
            capture_output=True,
            text=True,
            input="",
        )
        export_combined = (export_l.stdout or "") + (export_l.stderr or "")
        self.assertNotIn(
            "flag provided but not defined",
            export_combined,
            f"exported TCT_GOFMT_BIN is not real gofmt: {export_gofmt} -> {export_combined!r}",
        )

        # When a version launcher forced a shim, gofmt shim must not point at go.
        shim_gofmt = REPO_ROOT / "target" / "tct-pinned-go" / "bin" / "gofmt"
        shim_go = REPO_ROOT / "target" / "tct-pinned-go" / "bin" / "go"
        if shim_gofmt.exists() or shim_gofmt.is_symlink():
            self.assertNotEqual(
                shim_gofmt.resolve(),
                shim_go.resolve(),
                f"shim gofmt must not target the go launcher: {shim_gofmt} -> {shim_gofmt.resolve()}",
            )
            shim_help = subprocess.run(
                [str(shim_gofmt), "-h"],
                check=False,
                capture_output=True,
                text=True,
            )
            shim_text = (shim_help.stdout or "") + (shim_help.stderr or "")
            self.assertIn("gofmt", shim_text.lower(), shim_text)

    def test_rejects_tct_gofmt_bin_pointing_at_go_launcher(self) -> None:
        """Fail closed when TCT_GOFMT_BIN is the go binary (launcher mislink)."""
        pin = _pinned_from_source()
        env = {k: v for k, v in os.environ.items()}
        # Prefer a real pin-matching go so version checks pass.
        launcher = _find_versioned_launcher(pin)
        json_env = {k: v for k, v in os.environ.items() if k not in {"TCT_GO_BIN", "TCT_GOFMT_BIN"}}
        if launcher is not None:
            json_env["TCT_GO_BIN"] = str(launcher)
        probe = subprocess.run(
            [str(RESOLVER), "--json", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=json_env,
        )
        if probe.returncode != 0:
            self.skipTest(f"pinned go unavailable: {probe.stderr}")
        go_bin = json.loads(probe.stdout)["go_bin"]

        env["TCT_GO_BIN"] = go_bin
        env["TCT_GOFMT_BIN"] = go_bin  # deliberate misconfiguration
        completed = subprocess.run(
            [str(RESOLVER), "--check", "--repo-root", str(REPO_ROOT)],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertNotEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        err = completed.stderr.lower()
        self.assertTrue(
            "same binary" in err
            or "does not behave" in err
            or "launcher" in err
            or "gofmt" in err,
            completed.stderr,
        )

    def test_synthetic_version_launcher_shim_uses_goroot_gofmt(self) -> None:
        """Synthetic version-launcher layout: shim gofmt -> GOROOT/bin/gofmt."""
        pin = _pinned_from_source()
        with tempfile.TemporaryDirectory(prefix="tct-go-resolver-") as tmp:
            tmp_path = Path(tmp)
            home = tmp_path / "home"
            goroot = home / "sdk" / pin
            launcher = home / "go" / "bin" / pin
            real_gofmt = goroot / "bin" / "gofmt"
            launcher.parent.mkdir(parents=True)
            real_gofmt.parent.mkdir(parents=True)

            # Minimal go stub: version + env GOROOT.
            launcher.write_text(
                "#!/usr/bin/env bash\n"
                f'if [[ "$1" == "version" ]]; then echo "go version {pin} linux/amd64"; exit 0; fi\n'
                f'if [[ "$1" == "env" && "$2" == "GOROOT" ]]; then echo "{goroot}"; exit 0; fi\n'
                "exit 1\n",
                encoding="utf-8",
            )
            launcher.chmod(0o755)
            real_gofmt.write_text(
                "#!/usr/bin/env bash\n"
                'if [[ "$1" == "-h" || "$1" == "-help" ]]; then echo "usage: gofmt [flags]" >&2; exit 0; fi\n'
                'if [[ "$1" == "-l" ]]; then exit 0; fi\n'
                "exit 0\n",
                encoding="utf-8",
            )
            real_gofmt.chmod(0o755)

            # Copy resolver into a temp repo root that points pin parsing at real SoT.
            repo = tmp_path / "repo"
            (repo / "scripts" / "testing").mkdir(parents=True)
            (repo / "src").mkdir(parents=True)
            shutil.copy2(RESOLVER, repo / "scripts" / "testing" / "resolve-pinned-go.sh")
            # Pin file must exist for the resolver; copy the real const line.
            pin_text = PIN_SOURCE.read_text(encoding="utf-8")
            # Keep the file small but include the pin const the resolver greps.
            match = re.search(
                r'pub const GO_AST_SCHEMA_INVENTORY_PINNED_GO_VERSION[^\n]+',
                pin_text,
            )
            self.assertIsNotNone(match)
            assert match is not None
            (repo / "src" / "go_ast_schema_inventory.rs").write_text(
                match.group(0) + "\n",
                encoding="utf-8",
            )
            resolver = repo / "scripts" / "testing" / "resolve-pinned-go.sh"
            resolver.chmod(0o755)

            env = {
                "HOME": str(home),
                "PATH": "/usr/bin:/bin",
                "TCT_GO_BIN": str(launcher),
            }
            # Clear any ambient override.
            env = {**{k: v for k, v in os.environ.items() if k not in {"TCT_GO_BIN", "TCT_GOFMT_BIN"}}, **env}

            completed = subprocess.run(
                [str(resolver), "--json", "--repo-root", str(repo)],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            payload = json.loads(completed.stdout)
            self.assertEqual(Path(payload["gofmt_bin"]).resolve(), real_gofmt.resolve())
            self.assertEqual(Path(payload["export_gofmt_bin"]).resolve(), real_gofmt.resolve())

            shim_gofmt = repo / "target" / "tct-pinned-go" / "bin" / "gofmt"
            shim_go = repo / "target" / "tct-pinned-go" / "bin" / "go"
            self.assertTrue(shim_go.is_symlink() or shim_go.is_file())
            self.assertTrue(shim_gofmt.is_symlink() or shim_gofmt.is_file())
            self.assertEqual(shim_gofmt.resolve(), real_gofmt.resolve())
            self.assertNotEqual(shim_gofmt.resolve(), shim_go.resolve())

            # Behavioral: shim gofmt accepts -l.
            smoke = subprocess.run(
                [str(shim_gofmt), "-l"],
                check=False,
                capture_output=True,
                text=True,
                input="",
            )
            smoke_text = (smoke.stdout or "") + (smoke.stderr or "")
            self.assertNotIn("flag provided but not defined", smoke_text, smoke_text)
            self.assertEqual(smoke.returncode, 0, smoke_text)


if __name__ == "__main__":
    unittest.main()
