#!/usr/bin/env python3
"""Tests for the local quality helper shell wrapper.

Fork-recursion containment: these tests spawn
`quality-check.sh`, whose Python maintenance gate runs
`python3 -m unittest discover tests/python` — the suite this file belongs to.
Relying on a PATH-prepended fake `python3` alone is not enough when a tool
on PATH shadows it, so the harness layers several containment measures.

Layered containment, all of which must stay in place:

1. The helper is copied into an isolated script tree whose `tests/python` is
   a single trivial stub test, so even a fully bypassed interception cannot
   re-enter the real repository suite.
2. Stub interception is verified before the helper is spawned; if the fake
   `python3` cannot execute or win PATH lookup (e.g. noexec TMPDIR), the
   test skips instead of spawning.
3. The helper runs in its own process group with a hard timeout, and the
   group is killed on any interruption so children cannot outlive the test.
4. `TCT_PYTHON_SUITE_ACTIVE` marks suite processes; the helper refuses a
   nested full-suite invocation outright (regression tests below).
"""

import os
import shutil
import signal
import subprocess
import tempfile
import textwrap
import time
import unittest
from pathlib import Path
from typing import Dict, Optional


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts/testing/quality-check.sh"
LOCAL_CI = REPO_ROOT / "scripts/testing/local-ci.sh"
CONFTEST = REPO_ROOT / "tests/python/conftest.py"

SUITE_SENTINEL = "TCT_PYTHON_SUITE_ACTIVE"
SCRIPT_TIMEOUT_SECONDS = 180

# Mark this process as a tests/python suite member as soon as the spawning
# module is imported. pytest runs get the same mark from conftest.py, but
# `python3 -m unittest discover tests/python` does not load conftest.py (nor
# the package __init__), so the module that owns the dangerous spawn sets it
# too. Descendant processes inherit the mark and quality-check.sh refuses to
# run the real suite beneath it.
os.environ.setdefault(SUITE_SENTINEL, "1")


def write_executable(path: Path, contents: str) -> None:
    path.write_text(textwrap.dedent(contents), encoding="utf-8")
    os.chmod(path, 0o755)


def install_script_tree(tmp: Path) -> Path:
    """Copy quality-check.sh and its script dependencies into an isolated tree.

    The tree carries a stub `tests/python` containing one trivial test, so a
    `python3 -m unittest discover tests/python` that escapes the fake
    interpreter terminates immediately instead of re-entering the real
    repository suite.
    """
    tree = tmp / "tree"
    (tree / "scripts" / "testing").mkdir(parents=True)
    (tree / "src").mkdir()
    (tree / "target").mkdir()
    shutil.copy2(SCRIPT, tree / "scripts" / "testing" / "quality-check.sh")
    stub_tests = tree / "tests" / "python"
    stub_tests.mkdir(parents=True)
    (stub_tests / "test_stub_suite.py").write_text(
        textwrap.dedent(
            """\
            import unittest


            class TestStubSuite(unittest.TestCase):
                def test_noop(self):
                    self.assertTrue(True)


            if __name__ == "__main__":
                unittest.main()
            """
        ),
        encoding="utf-8",
    )
    return tree


def base_quality_env(tmp: Path, extra: Optional[Dict[str, str]] = None) -> Dict[str, str]:
    bin_dir = tmp / "bin"
    bin_dir.mkdir(exist_ok=True)
    report_dir = tmp / "quality"
    lock_file = tmp / "quality.lock"
    cargo_log = tmp / "cargo.log"
    python_log = tmp / "python.log"
    env = os.environ.copy()
    # This harness simulates a TOP-LEVEL operator invocation of the helper
    # with fully stubbed tools, so the suite sentinel set by the enclosing
    # run is dropped; the refusal path has its own explicit test below.
    # Safety against interception failure comes from the isolated script
    # tree and verify_stub_interception().
    env.pop(SUITE_SENTINEL, None)
    env.update(
        {
            "PATH": f"{bin_dir}{os.pathsep}{env['PATH']}",
            "TCT_DOC_COVERAGE_THRESHOLD": "0",
            "TCT_QUALITY_MIN_FREE_KIB": "0",
            "TCT_QUALITY_REPORT_DIR": str(report_dir),
            "TCT_QUALITY_LOCK_FILE": str(lock_file),
            "TCT_TEST_CARGO_LOG": str(cargo_log),
            "TCT_TEST_PYTHON_LOG": str(python_log),
        }
    )
    if extra:
        env.update(extra)
    return env




def install_fake_tools(tmp: Path, *, stub_df: bool = False) -> None:
    bin_dir = tmp / "bin"
    bin_dir.mkdir(exist_ok=True)
    write_executable(
        bin_dir / "cargo",
        """\
            #!/usr/bin/env bash
            set -euo pipefail
            printf 'cargo %s\\n' "$*" >> "${TCT_TEST_CARGO_LOG}"
            """,
    )
    write_executable(
        bin_dir / "python3",
        """\
            #!/usr/bin/env bash
            set -euo pipefail
            script="$1"
            shift
            printf '%s %s\\n' "${script}" "$*" >> "${TCT_TEST_PYTHON_LOG}"
            json_output=""
            args=("$@")
            for ((index = 0; index < ${#args[@]}; index++)); do
                if [[ "${args[$index]}" == "--json-output" ]]; then
                    json_output="${args[$((index + 1))]}"
                fi
            done
            if [[ -n "${json_output}" ]]; then
                mkdir -p "$(dirname "${json_output}")"
                printf '{"script":"%s"}\\n' "${script}" > "${json_output}"
            fi
            printf '%s fake output\\n' "${script}"
            """,
    )
    if stub_df:
        write_executable(
            bin_dir / "df",
            """\
                #!/usr/bin/env bash
                cat <<'EOF'
                Filesystem 1024-blocks Used Available Capacity Mounted on
                fakefs 1000000 999999 1 100% /fake
                EOF
                """,
        )


class QualityScriptHarness(unittest.TestCase):
    """Shared spawning discipline for tests that execute the quality helper."""

    def verify_stub_interception(self, tmp: Path, env: Dict[str, str]) -> None:
        """Prove the fake python3 executes and wins PATH lookup before spawning.

        If the stub cannot run (e.g. a noexec TMPDIR) bash silently falls
        through to the real python3 during PATH search, and the helper's
        Python gate would reach a real interpreter. Skip instead of spawning
        when interception cannot be proven.
        """
        bin_dir = tmp / "bin"
        try:
            probe = subprocess.run(
                ["bash", "-c", "type -P python3 && python3 --tct-stub-preflight"],
                env=env,
                cwd=str(tmp),
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            self.skipTest(f"cannot probe fake-python3 interception: {exc}")
        resolved = probe.stdout.splitlines()[0].strip() if probe.stdout else ""
        if probe.returncode != 0 or resolved != str(bin_dir / "python3"):
            self.skipTest(
                "cannot prove fake-python3 PATH interception "
                f"(resolved={resolved!r}, rc={probe.returncode}); spawning "
                "quality-check.sh could reach a real python3"
            )
        # Drop the preflight marker so log assertions see only helper calls.
        probe_log = Path(env["TCT_TEST_PYTHON_LOG"])
        if probe_log.exists():
            probe_log.unlink()

    def run_quality_script(
        self, tree: Path, env: Dict[str, str]
    ) -> subprocess.CompletedProcess:
        """Run the copied helper in its own process group with a hard timeout.

        Interrupted suite runs must not leave orphaned gate scripts behind;
        on timeout or interruption the entire process group is killed.
        """
        script = tree / "scripts" / "testing" / "quality-check.sh"
        proc = subprocess.Popen(
            ["bash", str(script)],
            cwd=str(tree),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True,
        )
        try:
            stdout, stderr = proc.communicate(timeout=SCRIPT_TIMEOUT_SECONDS)
        except BaseException:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                pass
            proc.wait(timeout=10)
            raise
        return subprocess.CompletedProcess(
            proc.args, proc.returncode, stdout, stderr
        )


class TestQualityCheckScript(QualityScriptHarness):
    def test_writes_advisory_json_artifacts(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            env = base_quality_env(tmp)
            report_dir = Path(env["TCT_QUALITY_REPORT_DIR"])
            cargo_log = Path(env["TCT_TEST_CARGO_LOG"])
            python_log = Path(env["TCT_TEST_PYTHON_LOG"])
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            self.assertIn(f"Advisory JSON reports: {report_dir}", completed.stdout)
            self.assertIn("Generated-source substring inventory", completed.stdout)
            self.assertIn("Stable quality gates passed.", completed.stdout)
            self.assertTrue((report_dir / "module-size-report.json").is_file())
            self.assertTrue((report_dir / "documentation-coverage-report.json").is_file())
            self.assertTrue((report_dir / "generated-source-inventory.json").is_file())
            self.assertTrue((report_dir / "unwrap-expect-report.json").is_file())

            python_calls = python_log.read_text(encoding="utf-8")
            self.assertIn(
                "scripts/testing/module-size-report.py --limit 600 --json-output",
                python_calls,
            )
            self.assertIn(
                "scripts/testing/generated-source-inventory.py --mode all --json-format artifact --json-output",
                python_calls,
            )
            self.assertIn(
                "scripts/testing/documentation-coverage-report.py --threshold 0 --json-output",
                python_calls,
            )
            self.assertIn("-m unittest discover tests/python", python_calls)
            self.assertIn("--fail-on-production --fail-on-test-assertions", python_calls)
            self.assertIn(
                "scripts/testing/unwrap-expect-report.py --root src --limit 40 --json-output",
                python_calls,
            )

            cargo_calls = cargo_log.read_text(encoding="utf-8")
            self.assertIn("cargo clippy --all-targets --quiet", cargo_calls)
            self.assertIn("cargo fmt --all -- --check", cargo_calls)
            self.assertIn("cargo test --quiet", cargo_calls)
            self.assertIn("cargo build --release --quiet", cargo_calls)

    def test_defaults_to_quality_owned_generated_cargo_target(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            env = base_quality_env(tmp)
            env.pop("TCT_GENERATED_CARGO_TARGET_DIR", None)
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            expected = str(tree / "target" / "tct-quality-generated-cargo")
            self.assertIn(f"TCT_GENERATED_CARGO_TARGET_DIR={expected}", completed.stdout)
            self.assertNotIn(
                f"TCT_GENERATED_CARGO_TARGET_DIR={tree / 'target' / 'tct-generated-cargo'}\n",
                completed.stdout,
            )

    def test_preserves_caller_generated_cargo_target(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            caller_target = tmp / "caller-generated-cargo"
            env = base_quality_env(
                tmp,
                {"TCT_GENERATED_CARGO_TARGET_DIR": str(caller_target)},
            )
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            self.assertIn(
                f"TCT_GENERATED_CARGO_TARGET_DIR={caller_target}",
                completed.stdout,
            )

    def test_local_ci_preserves_caller_generated_cargo_target(self):
        text = LOCAL_CI.read_text(encoding="utf-8")
        self.assertIn(
            'TCT_GENERATED_CARGO_TARGET_DIR="${TCT_GENERATED_CARGO_TARGET_DIR:-',
            text,
        )
        self.assertIn("local-ci-default-round-trip", text)

    def test_quality_lock_blocks_concurrent_helper(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            lock_file = tmp / "quality.lock"
            holder_ready = tmp / "holder-ready"
            release_holder = tmp / "release-holder"
            env = base_quality_env(tmp)
            env["TCT_QUALITY_LOCK_FILE"] = str(lock_file)
            env["TCT_QUALITY_LOCK_WAIT_SECONDS"] = "0"
            self.verify_stub_interception(tmp, env)

            # The holder loop is self-limiting (~30s) so an interrupted test
            # cannot leave an orphaned spin loop behind.
            holder = subprocess.Popen(
                [
                    "bash",
                    "-c",
                    textwrap.dedent(
                        f"""\
                        set -euo pipefail
                        : >>"{lock_file}"
                        exec 8<>"{lock_file}"
                        flock -n 8
                        printf 'pid=%s\\nrepo_root=holder\\nstarted_at=test\\n' "$$" >"{lock_file}"
                        touch "{holder_ready}"
                        for _ in $(seq 1 600); do
                          if [[ -f "{release_holder}" ]]; then
                            exit 0
                          fi
                          sleep 0.05
                        done
                        exit 0
                        """
                    ),
                ],
                cwd=str(tree),
                env=env,
                start_new_session=True,
            )
            try:
                deadline = time.time() + 5
                while not holder_ready.is_file() and time.time() < deadline:
                    time.sleep(0.05)
                self.assertTrue(holder_ready.is_file(), "holder never acquired the lock")

                completed = self.run_quality_script(tree, env)
                self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
                self.assertIn("another quality helper holds", completed.stderr)
                self.assertIn(str(lock_file), completed.stderr)
                self.assertIn("lock holder metadata:", completed.stderr)
                self.assertIn("repo_root=holder", completed.stderr)
                self.assertIn("TCT_QUALITY_LOCK_WAIT_SECONDS", completed.stderr)
            finally:
                release_holder.write_text("1\n", encoding="utf-8")
                try:
                    holder.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(holder.pid, signal.SIGKILL)
                    holder.wait(timeout=5)

    def test_low_disk_runs_earlier_gates_and_skips_release_build(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp, stub_df=True)
            # Override the default python3 stub so low-disk path still records
            # calls without needing JSON sidecars.
            write_executable(
                tmp / "bin" / "python3",
                """\
                    #!/usr/bin/env bash
                    set -euo pipefail
                    script="$1"
                    shift
                    printf '%s %s\\n' "${script}" "$*" >> "${TCT_TEST_PYTHON_LOG}"
                    """,
            )
            env = base_quality_env(tmp, {"TCT_QUALITY_MIN_FREE_KIB": "2"})
            cargo_log = Path(env["TCT_TEST_CARGO_LOG"])
            python_log = Path(env["TCT_TEST_PYTHON_LOG"])
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
            self.assertIn("Disk space preflight", completed.stdout)
            self.assertIn("Run `cargo clean` for repo-local build artifacts", completed.stdout)
            self.assertIn("Python maintenance tests", completed.stdout)
            self.assertIn(
                "Skipping release build because disk space preflight failed.",
                completed.stdout,
            )

            python_calls = python_log.read_text(encoding="utf-8")
            self.assertIn("-m unittest discover tests/python", python_calls)

            cargo_calls = cargo_log.read_text(encoding="utf-8")
            self.assertIn("cargo clippy --all-targets --quiet", cargo_calls)
            self.assertIn("cargo fmt --all -- --check", cargo_calls)
            self.assertIn("cargo test --quiet", cargo_calls)
            self.assertNotIn("cargo build --release --quiet", cargo_calls)


class TestQualityCheckSuiteRecursionGuard(QualityScriptHarness):
    """Regression coverage for the fork-recursion guard.

    The helper's Python maintenance gate re-runs this very suite. A nested
    invocation (helper spawned from inside a tests/python run) must refuse
    instead of recursing, and a top-level invocation must mark the suite it
    launches so descendants can refuse in turn. Every test here runs against
    the isolated script tree with a stubbed interpreter, so even a regressed
    helper cannot re-enter the real repository suite.
    """

    def test_refuses_python_suite_gate_when_suite_sentinel_set(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            env = base_quality_env(tmp, {SUITE_SENTINEL: "1"})
            python_log = Path(env["TCT_TEST_PYTHON_LOG"])
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
            self.assertIn("Refusing nested tests/python suite", completed.stdout)
            self.assertIn(SUITE_SENTINEL, completed.stdout)
            self.assertIn("FAIL stable: Python maintenance tests", completed.stdout)
            self.assertIn("Stable quality gates failed.", completed.stdout)
            python_calls = ""
            if python_log.exists():
                python_calls = python_log.read_text(encoding="utf-8")
            self.assertNotIn(
                "-m unittest discover tests/python",
                python_calls,
                "the nested helper must not invoke the Python suite at all",
            )

    def test_python_suite_gate_exports_suite_sentinel(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            tree = install_script_tree(tmp)
            install_fake_tools(tmp)
            # Record the sentinel each python3 call observes, so the gate's
            # export (inherited by every descendant of the real suite) is
            # verified mechanically.
            write_executable(
                tmp / "bin" / "python3",
                """\
                    #!/usr/bin/env bash
                    set -euo pipefail
                    script="$1"
                    shift
                    printf '%s %s sentinel=%s\\n' "${script}" "$*" \\
                        "${TCT_PYTHON_SUITE_ACTIVE:-unset}" >> "${TCT_TEST_PYTHON_LOG}"
                    """,
            )
            env = base_quality_env(tmp)
            python_log = Path(env["TCT_TEST_PYTHON_LOG"])
            self.verify_stub_interception(tmp, env)

            completed = self.run_quality_script(tree, env)

            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            python_calls = python_log.read_text(encoding="utf-8")
            self.assertIn(
                "-m unittest discover tests/python sentinel=1",
                python_calls,
                "the Python suite gate must export the suite sentinel to the "
                "suite it launches",
            )

    def test_base_quality_env_strips_suite_sentinel(self):
        """The harness env models a top-level run; the sentinel must be gone."""
        previous = os.environ.get(SUITE_SENTINEL)
        os.environ[SUITE_SENTINEL] = "1"
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                env = base_quality_env(Path(tmpdir))
        finally:
            if previous is None:
                os.environ.pop(SUITE_SENTINEL, None)
            else:
                os.environ[SUITE_SENTINEL] = previous
        self.assertNotIn(SUITE_SENTINEL, env)

    def test_suite_entry_points_set_suite_sentinel(self):
        """Mechanical drift guard: every suite runner must mark its run."""
        conftest_text = CONFTEST.read_text(encoding="utf-8")
        self.assertIn(
            'os.environ.setdefault("TCT_PYTHON_SUITE_ACTIVE", "1")',
            conftest_text,
            "tests/python/conftest.py must mark pytest suite runs",
        )
        script_text = SCRIPT.read_text(encoding="utf-8")
        self.assertIn(
            "TCT_PYTHON_SUITE_ACTIVE=1 python3 -m unittest discover tests/python",
            script_text,
            "the quality helper must mark the suite it launches",
        )
        self.assertIn(
            "Refusing nested tests/python suite",
            script_text,
            "the quality helper must refuse a nested full-suite invocation",
        )


if __name__ == "__main__":
    unittest.main()
