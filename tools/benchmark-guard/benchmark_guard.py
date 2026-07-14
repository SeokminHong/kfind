#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import signal
import sys
from pathlib import Path
from typing import Sequence

from guard import (
    BenchmarkGuardError,
    CommandNotFound,
    ConfigurationError,
    DEFAULT_STATUS_INTERVAL,
    DEFAULT_TERMINATION_GRACE,
    EXIT_TEMPORARY_FAILURE,
    EXIT_USAGE,
    GuardPaths,
    LockWaitTimeout,
    environment_number,
    human_status,
    normalize_timeout,
    run_benchmark,
    status_snapshot,
)


def run_command(arguments: argparse.Namespace) -> int:
    command = list(arguments.command)
    if command and command[0] == "--":
        command = command[1:]
    paths = GuardPaths.discover(Path.cwd())
    wait_timeout = configured_number(
        arguments.wait_timeout,
        "KFIND_BENCHMARK_WAIT_TIMEOUT",
        0.0,
    )
    run_timeout = configured_number(
        arguments.run_timeout,
        "KFIND_BENCHMARK_RUN_TIMEOUT",
        0.0,
    )
    return run_benchmark(
        paths=paths,
        name=arguments.name,
        command=command,
        wait_timeout=normalize_timeout(wait_timeout, "wait timeout"),
        run_timeout=normalize_timeout(run_timeout, "run timeout"),
        status_interval=configured_number(
            arguments.status_interval,
            "KFIND_BENCHMARK_STATUS_INTERVAL",
            DEFAULT_STATUS_INTERVAL,
        ),
        termination_grace=configured_number(
            arguments.termination_grace,
            "KFIND_BENCHMARK_TERMINATION_GRACE",
            DEFAULT_TERMINATION_GRACE,
        ),
    )


def configured_number(value: float | None, environment: str, default: float) -> float:
    return environment_number(environment, default) if value is None else value


def status_command(arguments: argparse.Namespace) -> int:
    snapshot = status_snapshot(GuardPaths.discover(Path.cwd()))
    if arguments.json:
        print(json.dumps(snapshot, ensure_ascii=False, sort_keys=True))
    else:
        print(human_status(snapshot))
    return 0 if snapshot["healthy"] else 1


def doctor_command(arguments: argparse.Namespace) -> int:
    paths = GuardPaths.discover(Path.cwd())
    snapshot = status_snapshot(paths)
    result = {
        "healthy": bool(snapshot["healthy"])
        and os.access(paths.common_directory, os.W_OK),
        "lock_directory_writable": os.access(paths.common_directory, os.W_OK),
        "python": sys.version.split()[0],
        "platform": sys.platform,
        "repository": str(paths.repository),
        "common_directory": str(paths.common_directory),
        "lock": snapshot,
    }
    if arguments.json:
        print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    else:
        print(f"benchmark guard: {'healthy' if result['healthy'] else 'unhealthy'}")
        print(f"repository: {result['repository']}")
        print(f"common directory: {result['common_directory']}")
        print(f"writable: {str(result['lock_directory_writable']).lower()}")
        print(human_status(snapshot))
    return 0 if result["healthy"] else 1


def parser() -> argparse.ArgumentParser:
    argument_parser = argparse.ArgumentParser(
        description="Serialize kfind benchmarks across Git worktrees."
    )
    subparsers = argument_parser.add_subparsers(dest="subcommand", required=True)

    run_parser = subparsers.add_parser("run", help="run a command with the global lock")
    run_parser.add_argument("--name", required=True, help="benchmark name shown by status")
    run_parser.add_argument(
        "--wait-timeout",
        type=float,
        default=None,
        help="seconds to wait for the lock; zero waits without a limit",
    )
    run_parser.add_argument(
        "--run-timeout",
        type=float,
        default=None,
        help="seconds the command may run; zero has no limit",
    )
    run_parser.add_argument(
        "--status-interval",
        type=float,
        default=None,
        help="seconds between lock wait messages",
    )
    run_parser.add_argument(
        "--termination-grace",
        type=float,
        default=None,
        help="seconds between TERM and KILL after a timeout",
    )
    run_parser.add_argument("command", nargs=argparse.REMAINDER)
    run_parser.set_defaults(function=run_command)

    status_parser = subparsers.add_parser("status", help="show the current lock owner")
    status_parser.add_argument("--json", action="store_true")
    status_parser.set_defaults(function=status_command)

    doctor_parser = subparsers.add_parser("doctor", help="check lock health and paths")
    doctor_parser.add_argument("--json", action="store_true")
    doctor_parser.set_defaults(function=doctor_command)
    return argument_parser


def main(arguments: Sequence[str] | None = None) -> int:
    try:
        parsed_arguments = parser().parse_args(arguments)
        return int(parsed_arguments.function(parsed_arguments))
    except LockWaitTimeout as error:
        print(f"benchmark guard: {error}", file=sys.stderr)
        return EXIT_TEMPORARY_FAILURE
    except CommandNotFound as error:
        print(f"benchmark guard: {error}", file=sys.stderr)
        return 127
    except ConfigurationError as error:
        print(f"benchmark guard: {error}", file=sys.stderr)
        return EXIT_USAGE
    except BenchmarkGuardError as error:
        print(f"benchmark guard: {error}", file=sys.stderr)
        return 1
    except OSError as error:
        print(f"benchmark guard: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 128 + signal.SIGINT


if __name__ == "__main__":
    raise SystemExit(main())
