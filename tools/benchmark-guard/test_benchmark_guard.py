from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from guard import GuardPaths


TOOL_DIRECTORY = Path(__file__).resolve().parent
REPOSITORY = TOOL_DIRECTORY.parents[1]
CLI = TOOL_DIRECTORY / "benchmark_guard.py"
METADATA_FILE_NAME = "kfind-benchmark-owner.json"
CHILD_TIMELINE = """
import pathlib
import sys
import time

path = pathlib.Path(sys.argv[1])
with path.open("a", encoding="utf-8") as output:
    output.write(f"start {time.monotonic()}\\n")
    output.flush()
time.sleep(float(sys.argv[2]))
with path.open("a", encoding="utf-8") as output:
    output.write(f"end {time.monotonic()}\\n")
"""


class BenchmarkGuardTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.lock_directory = Path(self.temporary_directory.name)
        self.environment = os.environ.copy()
        self.environment["KFIND_BENCHMARK_LOCK_DIR"] = str(self.lock_directory)
        for name in (
            "KFIND_BENCHMARK_WAIT_TIMEOUT",
            "KFIND_BENCHMARK_RUN_TIMEOUT",
            "KFIND_BENCHMARK_STATUS_INTERVAL",
            "KFIND_BENCHMARK_TERMINATION_GRACE",
        ):
            self.environment.pop(name, None)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def guard_command(
        self,
        name: str,
        command: list[str],
        *options: str,
    ) -> list[str]:
        return [
            sys.executable,
            str(CLI),
            "run",
            "--name",
            name,
            "--status-interval",
            "0.05",
            *options,
            "--",
            *command,
        ]

    def start_guard(
        self,
        name: str,
        command: list[str],
        *options: str,
    ) -> subprocess.Popen[str]:
        return subprocess.Popen(
            self.guard_command(name, command, *options),
            cwd=REPOSITORY,
            env=self.environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def run_guard(
        self,
        name: str,
        command: list[str],
        *options: str,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            self.guard_command(name, command, *options),
            cwd=REPOSITORY,
            env=self.environment,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )

    def wait_for_metadata(self, timeout: float = 2.0) -> dict[str, object]:
        metadata_path = self.lock_directory / METADATA_FILE_NAME
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                value = json.loads(metadata_path.read_text(encoding="utf-8"))
            except (FileNotFoundError, json.JSONDecodeError):
                time.sleep(0.02)
                continue
            if value.get("child_pid"):
                return value
            time.sleep(0.02)
        self.fail("benchmark owner metadata was not written")

    def status(self) -> tuple[subprocess.CompletedProcess[str], dict[str, object]]:
        result = subprocess.run(
            [sys.executable, str(CLI), "status", "--json"],
            cwd=REPOSITORY,
            env=self.environment,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
        return result, json.loads(result.stdout)

    def test_default_lock_uses_git_common_directory(self) -> None:
        with mock.patch.dict(
            os.environ,
            {"KFIND_BENCHMARK_LOCK_DIR": ""},
        ):
            paths = GuardPaths.discover(REPOSITORY)
        common_directory = subprocess.run(
            ["git", "rev-parse", "--git-common-dir"],
            cwd=REPOSITORY,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        expected = Path(common_directory)
        if not expected.is_absolute():
            expected = REPOSITORY / expected

        self.assertEqual(paths.common_directory, expected.resolve())
        self.assertEqual(paths.lock.parent, expected.resolve())

    def test_commands_run_exclusively(self) -> None:
        first_timeline = self.lock_directory / "first.txt"
        second_timeline = self.lock_directory / "second.txt"
        first = self.start_guard(
            "first",
            [sys.executable, "-c", CHILD_TIMELINE, str(first_timeline), "0.4"],
        )
        self.wait_for_metadata()
        second = self.start_guard(
            "second",
            [sys.executable, "-c", CHILD_TIMELINE, str(second_timeline), "0.1"],
        )

        first_stdout, first_stderr = first.communicate(timeout=5)
        second_stdout, second_stderr = second.communicate(timeout=5)
        self.assertEqual((first.returncode, first_stdout), (0, ""), first_stderr)
        self.assertEqual((second.returncode, second_stdout), (0, ""), second_stderr)

        first_end = float(first_timeline.read_text().splitlines()[1].split()[1])
        second_start = float(second_timeline.read_text().splitlines()[0].split()[1])
        self.assertGreaterEqual(second_start, first_end)
        self.assertIn("benchmark lock busy", second_stderr)

    def test_wait_timeout_does_not_start_command(self) -> None:
        owner = self.start_guard(
            "owner",
            [sys.executable, "-c", "import time; time.sleep(0.5)"],
        )
        self.wait_for_metadata()
        marker = self.lock_directory / "unexpected.txt"
        contender = self.run_guard(
            "contender",
            [sys.executable, "-c", f"from pathlib import Path; Path({str(marker)!r}).touch()"],
            "--wait-timeout",
            "0.1",
        )
        owner.communicate(timeout=5)

        self.assertEqual(contender.returncode, 75, contender.stderr)
        self.assertFalse(marker.exists())
        self.assertIn("lock wait timed out", contender.stderr)

    def test_run_timeout_stops_process_group(self) -> None:
        result = self.run_guard(
            "timeout",
            [
                sys.executable,
                "-c",
                (
                    "import signal,time; "
                    "signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                    "time.sleep(10)"
                ),
            ],
            "--run-timeout",
            "0.2",
            "--termination-grace",
            "0.1",
        )
        status_result, snapshot = self.status()

        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertIn("run timed out", result.stderr)
        self.assertEqual(status_result.returncode, 0, status_result.stderr)
        self.assertEqual(snapshot["state"], "idle")

    def test_status_reports_the_active_owner(self) -> None:
        owner = self.start_guard(
            "status-owner",
            [sys.executable, "-c", "import time; time.sleep(0.4)"],
        )
        metadata = self.wait_for_metadata()
        metadata_path = self.lock_directory / METADATA_FILE_NAME
        metadata_modified_at = metadata_path.stat().st_mtime_ns
        status_result, snapshot = self.status()
        human_status_result = subprocess.run(
            [sys.executable, str(CLI), "status"],
            cwd=REPOSITORY,
            env=self.environment,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
        metadata_modified_after_status = metadata_path.stat().st_mtime_ns
        owner.communicate(timeout=5)

        self.assertEqual(status_result.returncode, 0, status_result.stderr)
        self.assertEqual(snapshot["state"], "running")
        self.assertEqual(snapshot["name"], "status-owner")
        self.assertEqual(snapshot["child_pid"], metadata["child_pid"])
        self.assertTrue(snapshot["supervisor_alive"])
        self.assertGreaterEqual(snapshot["elapsed_seconds"], 0)
        self.assertEqual(human_status_result.returncode, 0, human_status_result.stderr)
        self.assertIn("supervisor alive: True", human_status_result.stdout)
        self.assertIn("child alive: True", human_status_result.stdout)
        self.assertEqual(metadata_modified_after_status, metadata_modified_at)

    def test_termination_signal_stops_process_group(self) -> None:
        owner = self.start_guard(
            "signal-owner",
            [
                sys.executable,
                "-c",
                (
                    "import signal,time; "
                    "signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                    "time.sleep(10)"
                ),
            ],
            "--termination-grace",
            "0.1",
        )
        self.wait_for_metadata()
        os.kill(owner.pid, signal.SIGTERM)
        _, stderr = owner.communicate(timeout=5)

        self.assertEqual(owner.returncode, 128 + signal.SIGTERM, stderr)
        self.assertIn("received signal", stderr)

    def test_child_keeps_lock_if_supervisor_is_killed(self) -> None:
        child_pid_path = self.lock_directory / "child.pid"
        owner = self.start_guard(
            "orphan-owner",
            [
                sys.executable,
                "-c",
                (
                    "import os,pathlib,sys,time; "
                    "pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); "
                    "time.sleep(0.8)"
                ),
                str(child_pid_path),
            ],
        )
        metadata = self.wait_for_metadata()
        child_pid = int(metadata["child_pid"])
        try:
            os.kill(owner.pid, signal.SIGKILL)
            owner.wait(timeout=2)
            if owner.stdout is not None:
                owner.stdout.close()
            if owner.stderr is not None:
                owner.stderr.close()
            status_result, orphan_snapshot = self.status()
            self.assertEqual(status_result.returncode, 1, status_result.stderr)
            self.assertEqual(orphan_snapshot["state"], "unhealthy")
            self.assertFalse(orphan_snapshot["supervisor_alive"])
            self.assertTrue(orphan_snapshot["child_alive"])

            contender = self.run_guard(
                "blocked-by-child",
                [sys.executable, "-c", "pass"],
                "--wait-timeout",
                "0.15",
            )
            self.assertEqual(contender.returncode, 75, contender.stderr)

            deadline = time.monotonic() + 3
            snapshot: dict[str, object] = {}
            while time.monotonic() < deadline:
                _, snapshot = self.status()
                if snapshot["state"] == "idle":
                    break
                time.sleep(0.05)
            self.assertEqual(snapshot["state"], "idle")
            self.assertTrue(snapshot["stale_metadata"])

            successor = self.run_guard(
                "successor",
                [sys.executable, "-c", "pass"],
                "--wait-timeout",
                "0.5",
            )
            self.assertEqual(successor.returncode, 0, successor.stderr)
        finally:
            try:
                os.killpg(child_pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def test_child_exit_code_is_preserved(self) -> None:
        result = self.run_guard(
            "failure",
            [sys.executable, "-c", "raise SystemExit(7)"],
        )

        self.assertEqual(result.returncode, 7, result.stderr)

    def test_invalid_run_environment_does_not_break_status(self) -> None:
        self.environment["KFIND_BENCHMARK_RUN_TIMEOUT"] = "invalid"
        status_result, snapshot = self.status()
        run_result = self.run_guard(
            "invalid-environment",
            [sys.executable, "-c", "pass"],
        )

        self.assertEqual(status_result.returncode, 0, status_result.stderr)
        self.assertEqual(snapshot["state"], "idle")
        self.assertEqual(run_result.returncode, 64, run_result.stderr)
        self.assertIn("must be a number", run_result.stderr)


if __name__ == "__main__":
    unittest.main()
