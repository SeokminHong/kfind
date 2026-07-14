from __future__ import annotations

import errno
import fcntl
import json
import math
import os
import shlex
import signal
import subprocess
import sys
import threading
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Sequence

EXIT_USAGE = 64
EXIT_TEMPORARY_FAILURE = 75
EXIT_TIMEOUT = 124
DEFAULT_STATUS_INTERVAL = 30.0
DEFAULT_TERMINATION_GRACE = 10.0
LOCK_FILE_NAME = "kfind-benchmark.lock"
METADATA_FILE_NAME = "kfind-benchmark-owner.json"
SESSION_ENVIRONMENT = "KFIND_BENCHMARK_SESSION"


class BenchmarkGuardError(Exception):
    pass


class ConfigurationError(BenchmarkGuardError):
    pass


class LockWaitTimeout(BenchmarkGuardError):
    def __init__(self, elapsed: float) -> None:
        super().__init__(f"benchmark lock wait timed out after {elapsed:.1f}s")
        self.elapsed = elapsed


class CommandNotFound(BenchmarkGuardError):
    pass


@dataclass(frozen=True)
class GuardPaths:
    repository: Path
    common_directory: Path
    lock: Path
    metadata: Path

    @classmethod
    def discover(cls, cwd: Path) -> GuardPaths:
        repository = Path(git_output(cwd, "rev-parse", "--show-toplevel")).resolve()
        common_directory_override = os.environ.get("KFIND_BENCHMARK_LOCK_DIR")
        if common_directory_override:
            common_directory = Path(common_directory_override).expanduser().resolve()
        else:
            raw_common_directory = Path(
                git_output(repository, "rev-parse", "--git-common-dir")
            )
            if not raw_common_directory.is_absolute():
                raw_common_directory = repository / raw_common_directory
            common_directory = raw_common_directory.resolve()
        return cls(
            repository=repository,
            common_directory=common_directory,
            lock=common_directory / LOCK_FILE_NAME,
            metadata=common_directory / METADATA_FILE_NAME,
        )


class ExclusiveLock:
    def __init__(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.path = path
        self.file_descriptor = os.open(path, os.O_CREAT | os.O_RDWR, 0o600)

    def acquire(self, wait_timeout: float | None, status_interval: float) -> float:
        started = time.monotonic()
        next_status = started
        while True:
            try:
                fcntl.flock(
                    self.file_descriptor,
                    fcntl.LOCK_EX | fcntl.LOCK_NB,
                )
                return time.monotonic() - started
            except OSError as error:
                if error.errno not in (errno.EACCES, errno.EAGAIN):
                    raise

            now = time.monotonic()
            elapsed = now - started
            if wait_timeout is not None and elapsed >= wait_timeout:
                raise LockWaitTimeout(elapsed)
            if now >= next_status:
                print_wait_status(self.path, elapsed)
                next_status = now + status_interval

            sleep_seconds = 0.2
            if wait_timeout is not None:
                sleep_seconds = min(sleep_seconds, max(0.0, wait_timeout - elapsed))
            time.sleep(sleep_seconds)

    def close(self) -> None:
        if self.file_descriptor >= 0:
            os.close(self.file_descriptor)
            self.file_descriptor = -1


def git_output(cwd: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise ConfigurationError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def parse_timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace(
        "+00:00", "Z"
    )


def read_json(path: Path) -> dict[str, Any] | None:
    try:
        with path.open(encoding="utf-8") as source:
            value = json.load(source)
    except (FileNotFoundError, json.JSONDecodeError, OSError):
        return None
    return value if isinstance(value, dict) else None


def write_json(path: Path, value: dict[str, Any]) -> None:
    temporary = path.with_name(
        f".{path.name}.{os.getpid()}.{uuid.uuid4().hex}.tmp"
    )
    try:
        with temporary.open("x", encoding="utf-8") as destination:
            json.dump(value, destination, ensure_ascii=False, indent=2, sort_keys=True)
            destination.write("\n")
        os.chmod(temporary, 0o600)
        os.replace(temporary, path)
    except OSError as error:
        raise BenchmarkGuardError(f"could not write benchmark metadata: {error}") from error
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def remove_owned_metadata(path: Path, session: str) -> None:
    metadata = read_json(path)
    if metadata is None or metadata.get("session") != session:
        return
    try:
        path.unlink()
    except FileNotFoundError:
        pass
    except OSError as error:
        print(f"benchmark guard warning: could not remove metadata: {error}", file=sys.stderr)


def print_wait_status(lock_path: Path, elapsed: float) -> None:
    metadata = read_json(lock_path.with_name(METADATA_FILE_NAME))
    if metadata is None:
        owner = "owner metadata unavailable"
    else:
        name = metadata.get("name", "unknown")
        worktree = metadata.get("worktree", "unknown")
        owner = f"{name} in {worktree}"
    print(
        f"benchmark lock busy ({owner}); waiting {format_duration(elapsed)}",
        file=sys.stderr,
        flush=True,
    )


def format_duration(seconds: float) -> str:
    total = max(0, int(seconds))
    hours, remainder = divmod(total, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f"{hours:02d}:{minutes:02d}:{seconds:02d}"


def normalize_timeout(value: float, name: str) -> float | None:
    if not math.isfinite(value) or value < 0:
        raise ConfigurationError(f"{name} must be zero or greater")
    return None if value == 0 else value


def environment_number(name: str, default: float) -> float:
    raw_value = os.environ.get(name)
    if raw_value is None:
        return default
    try:
        return float(raw_value)
    except ValueError as error:
        raise ConfigurationError(f"{name} must be a number, got {raw_value!r}") from error


def process_is_alive(pid: Any) -> bool:
    if not isinstance(pid, int) or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def lock_is_held(path: Path) -> bool:
    path.parent.mkdir(parents=True, exist_ok=True)
    file_descriptor = os.open(path, os.O_CREAT | os.O_RDWR, 0o600)
    try:
        try:
            fcntl.flock(file_descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            if error.errno in (errno.EACCES, errno.EAGAIN):
                return True
            raise
        fcntl.flock(file_descriptor, fcntl.LOCK_UN)
        return False
    finally:
        os.close(file_descriptor)


def status_snapshot(paths: GuardPaths) -> dict[str, Any]:
    held = lock_is_held(paths.lock)
    metadata = read_json(paths.metadata)
    snapshot: dict[str, Any] = {
        "lock_path": str(paths.lock),
        "metadata_path": str(paths.metadata),
        "repository": str(paths.repository),
    }
    if not held:
        snapshot.update(
            {
                "state": "idle",
                "healthy": True,
                "stale_metadata": metadata is not None,
            }
        )
        return snapshot

    if metadata is None:
        snapshot.update(
            {
                "state": "unhealthy",
                "healthy": False,
                "reason": "lock is held without readable owner metadata",
            }
        )
        return snapshot

    supervisor_alive = process_is_alive(metadata.get("supervisor_pid"))
    healthy = supervisor_alive
    started_at = parse_timestamp(metadata.get("started_at"))
    elapsed_seconds = None
    if started_at is not None:
        elapsed_seconds = max(
            0.0, (datetime.now(timezone.utc) - started_at).total_seconds()
        )
    snapshot.update(metadata)
    snapshot.update(
        {
            "state": "running" if healthy else "unhealthy",
            "healthy": healthy,
            "supervisor_alive": supervisor_alive,
            "child_alive": process_is_alive(metadata.get("child_pid")),
            "elapsed_seconds": elapsed_seconds,
        }
    )
    if not healthy:
        snapshot["reason"] = "supervisor is not alive while the lock remains held"
    return snapshot


def human_status(snapshot: dict[str, Any]) -> str:
    lines = [f"benchmark lock: {snapshot['state']}"]
    if snapshot["state"] == "idle":
        if snapshot.get("stale_metadata"):
            lines.append("stale metadata: present")
        lines.append(f"lock: {snapshot['lock_path']}")
        return "\n".join(lines)

    for label, key in (
        ("name", "name"),
        ("worktree", "worktree"),
        ("revision", "revision"),
        ("command", "command_display"),
        ("supervisor pid", "supervisor_pid"),
        ("child pid", "child_pid"),
        ("supervisor alive", "supervisor_alive"),
        ("child alive", "child_alive"),
        ("started", "started_at"),
    ):
        if snapshot.get(key) is not None:
            lines.append(f"{label}: {snapshot[key]}")
    elapsed_seconds = snapshot.get("elapsed_seconds")
    if isinstance(elapsed_seconds, (int, float)):
        lines.append(f"elapsed: {format_duration(elapsed_seconds)}")
    if snapshot.get("reason"):
        lines.append(f"reason: {snapshot['reason']}")
    return "\n".join(lines)


def terminate_process_group(process: subprocess.Popen[Any], grace: float) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=grace)
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        return
    process.wait()


def monitor_process(
    process: subprocess.Popen[Any],
    run_timeout: float | None,
    termination_grace: float,
) -> int:
    termination_lock = threading.RLock()
    termination_reason: list[str | None] = [None]
    kill_timer: list[threading.Timer | None] = [None]

    def kill_process_group() -> None:
        if process.poll() is not None:
            return
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    def begin_termination(reason: str) -> None:
        with termination_lock:
            if termination_reason[0] is not None or process.poll() is not None:
                return
            termination_reason[0] = reason
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                return
            if termination_grace == 0:
                kill_process_group()
                return
            kill_timer[0] = threading.Timer(termination_grace, kill_process_group)
            kill_timer[0].daemon = True
            kill_timer[0].start()

    timeout_timer = None
    if run_timeout is not None:
        timeout_timer = threading.Timer(run_timeout, begin_termination, args=("timeout",))
        timeout_timer.daemon = True
        timeout_timer.start()

    previous_handlers: dict[int, Any] = {}

    def forward_signal(signal_number: int, _frame: Any) -> None:
        begin_termination(f"signal:{signal_number}")

    for signal_number in (signal.SIGHUP, signal.SIGINT, signal.SIGTERM):
        previous_handlers[signal_number] = signal.getsignal(signal_number)
        signal.signal(signal_number, forward_signal)
    try:
        return_code = process.wait()
    finally:
        for signal_number, previous_handler in previous_handlers.items():
            signal.signal(signal_number, previous_handler)
        if timeout_timer is not None:
            timeout_timer.cancel()
        if kill_timer[0] is not None:
            kill_timer[0].cancel()

    reason = termination_reason[0]
    if reason == "timeout":
        print(
            f"benchmark run timed out after {run_timeout:.1f}s; process group stopped",
            file=sys.stderr,
        )
        return EXIT_TIMEOUT
    if reason is not None and reason.startswith("signal:"):
        signal_number = int(reason.partition(":")[2])
        print(
            f"benchmark guard received signal {signal_number}; process group stopped",
            file=sys.stderr,
        )
        return 128 + signal_number
    return return_code if return_code >= 0 else 128 + abs(return_code)


def run_benchmark(
    paths: GuardPaths,
    name: str,
    command: Sequence[str],
    wait_timeout: float | None,
    run_timeout: float | None,
    status_interval: float,
    termination_grace: float,
) -> int:
    if not command:
        raise ConfigurationError("run requires a command after --")
    if (
        not name.strip()
        or name != name.strip()
        or len(name) > 100
        or any(ord(character) < 32 for character in name)
    ):
        raise ConfigurationError("benchmark name must be a short printable value")
    intervals = (status_interval, termination_grace)
    if (
        not all(math.isfinite(value) for value in intervals)
        or status_interval <= 0
        or termination_grace < 0
    ):
        raise ConfigurationError("intervals must be positive and grace must not be negative")

    lock = ExclusiveLock(paths.lock)
    session = uuid.uuid4().hex
    process: subprocess.Popen[Any] | None = None
    try:
        wait_seconds = lock.acquire(wait_timeout, status_interval)
        print(
            f"benchmark lock acquired for {name} after {wait_seconds:.1f}s",
            file=sys.stderr,
        )
        metadata: dict[str, Any] = {
            "schema": 1,
            "session": session,
            "name": name,
            "repository": str(paths.repository),
            "worktree": str(paths.repository),
            "revision": git_output(paths.repository, "rev-parse", "HEAD"),
            "dirty": bool(
                git_output(
                    paths.repository,
                    "status",
                    "--porcelain",
                )
            ),
            "command": list(command),
            "command_display": shlex.join(command),
            "supervisor_pid": os.getpid(),
            "child_pid": None,
            "process_group_id": None,
            "started_at": utc_now(),
            "lock_wait_seconds": round(wait_seconds, 6),
            "run_timeout_seconds": run_timeout,
        }
        write_json(paths.metadata, metadata)

        environment = os.environ.copy()
        environment[SESSION_ENVIRONMENT] = session
        try:
            process = subprocess.Popen(
                list(command),
                cwd=Path.cwd(),
                env=environment,
                start_new_session=True,
                pass_fds=(lock.file_descriptor,),
            )
        except FileNotFoundError as error:
            raise CommandNotFound(f"command not found: {command[0]}") from error
        except OSError as error:
            raise BenchmarkGuardError(f"could not start benchmark command: {error}") from error

        metadata["child_pid"] = process.pid
        metadata["process_group_id"] = process.pid
        write_json(paths.metadata, metadata)
        return monitor_process(
            process,
            run_timeout,
            termination_grace,
        )
    finally:
        try:
            if process is not None and process.poll() is None:
                terminate_process_group(process, termination_grace)
        finally:
            remove_owned_metadata(paths.metadata, session)
            lock.close()
