#!/usr/bin/env python3

import argparse
import fcntl
import hashlib
import json
import os
import pty
import re
import select
import statistics
import struct
import subprocess
import sys
import termios
import time
from pathlib import Path


FIXTURE_LINES = 2_000
FIXTURE_LINE_BYTES = b"x" * 180 + b" needle " + b"y" * 200 + b"\n"
QUERY = "needle"
SCROLL_UP_PATTERN = re.compile(rb"\x1b\[([0-9]+)S")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Measure held-key TUI scrolling through a continuously drained PTY."
    )
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("target/benchmark/tui-scroll/input.txt"),
    )
    parser.add_argument(
        "--geometry",
        action="append",
        type=parse_geometry,
        default=None,
        metavar="ROWSxCOLS",
    )
    parser.add_argument("--events", type=positive_int, default=300)
    parser.add_argument("--interval-ms", type=positive_float, default=20.0)
    parser.add_argument("--warmups", type=nonnegative_int, default=1)
    parser.add_argument("--runs", type=positive_int, default=5)
    args = parser.parse_args()
    args.geometry = args.geometry or [(25, 80), (73, 316)]
    return args


def parse_geometry(value: str) -> tuple[int, int]:
    try:
        rows, columns = (int(part) for part in value.lower().split("x", 1))
    except ValueError as error:
        raise argparse.ArgumentTypeError("geometry must be ROWSxCOLS") from error
    if rows <= 1 or columns <= 0:
        raise argparse.ArgumentTypeError("geometry must have at least 2 rows and 1 column")
    return rows, columns


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def nonnegative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("value must not be negative")
    return parsed


def positive_float(value: str) -> float:
    parsed = float(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def write_fixture(path: Path) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    with path.open("wb") as output:
        for _ in range(FIXTURE_LINES):
            output.write(FIXTURE_LINE_BYTES)
            digest.update(FIXTURE_LINE_BYTES)
    return digest.hexdigest()


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as input_file:
        while chunk := input_file.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def read_available(master: int, output: bytearray) -> None:
    while select.select([master], [], [], 0)[0]:
        try:
            chunk = os.read(master, 65_536)
        except OSError:
            return
        if not chunk:
            return
        output.extend(chunk)


def wait_for(master: int, output: bytearray, marker: bytes, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while marker not in output:
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not select.select([master], [], [], remaining)[0]:
            raise TimeoutError(f"timed out waiting for {marker!r}")
        try:
            chunk = os.read(master, 65_536)
        except OSError as error:
            raise RuntimeError("benchmark process closed its PTY") from error
        if not chunk:
            raise RuntimeError("benchmark process closed its PTY")
        output.extend(chunk)


def wait_for_exit(process: subprocess.Popen[bytes], master: int, output: bytearray) -> None:
    deadline = time.monotonic() + 5
    while process.poll() is None and time.monotonic() < deadline:
        select.select([master], [], [], 0.01)
        read_available(master, output)
    if process.poll() is None:
        process.kill()
        raise TimeoutError("benchmark process did not exit")
    read_available(master, output)
    if process.returncode != 0:
        raise RuntimeError(f"benchmark process exited with {process.returncode}")


def run_once(
    binary: Path,
    fixture: Path,
    rows: int,
    columns: int,
    events: int,
    interval_seconds: float,
) -> dict[str, float | int]:
    if events >= FIXTURE_LINES - (rows - 1):
        raise ValueError("event count must leave room before the viewport boundary")

    master, slave = pty.openpty()
    fcntl.ioctl(
        slave,
        termios.TIOCSWINSZ,
        struct.pack("HHHH", rows, columns, 0, 0),
    )
    environment = dict(os.environ)
    environment["LC_ALL"] = "C"
    process = subprocess.Popen(
        [str(binary), "--literal", QUERY, str(fixture)],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        close_fds=True,
        env=environment,
    )
    os.close(slave)
    output = bytearray()
    try:
        wait_for(master, output, f"\x1b[7m1/{FIXTURE_LINES}".encode(), 10)
        started = time.monotonic()
        next_send = started
        for _ in range(events):
            while True:
                now = time.monotonic()
                if now >= next_send:
                    break
                select.select([master], [], [], next_send - now)
                read_available(master, output)
            os.write(master, b"j")
            next_send += interval_seconds
            read_available(master, output)
        sent = time.monotonic()
        wait_for(
            master,
            output,
            f"\x1b[7m{events + 1}/{FIXTURE_LINES}".encode(),
            10,
        )
        caught_up = time.monotonic()
        os.write(master, b"q")
        wait_for_exit(process, master, output)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)

    scrolls = [int(value) for value in SCROLL_UP_PATTERN.findall(output)]
    if sum(scrolls) != events:
        raise RuntimeError(
            f"expected to scroll {events} rows, observed {sum(scrolls)}"
        )
    return {
        "output_bytes": len(output),
        "scroll_frames": len(scrolls),
        "scrolled_rows": sum(scrolls),
        "send_ms": (sent - started) * 1_000,
        "catchup_ms": (caught_up - sent) * 1_000,
    }


def summarize(values: list[dict[str, float | int]]) -> dict[str, dict[str, float]]:
    return {
        metric: {
            "median": statistics.median(float(value[metric]) for value in values),
            "min": min(float(value[metric]) for value in values),
            "max": max(float(value[metric]) for value in values),
        }
        for metric in values[0]
    }


def main() -> None:
    args = parse_args()
    binary = args.binary.resolve(strict=True)
    fixture = args.fixture.resolve()
    fixture_sha256 = write_fixture(fixture)
    profiles = []
    for rows, columns in args.geometry:
        for _ in range(args.warmups):
            run_once(
                binary,
                fixture,
                rows,
                columns,
                args.events,
                args.interval_ms / 1_000,
            )
        samples = [
            run_once(
                binary,
                fixture,
                rows,
                columns,
                args.events,
                args.interval_ms / 1_000,
            )
            for _ in range(args.runs)
        ]
        profiles.append(
            {
                "rows": rows,
                "columns": columns,
                "samples": samples,
                "summary": summarize(samples),
            }
        )

    report = {
        "schema": 1,
        "label": args.label,
        "revision": args.revision,
        "binary": str(binary),
        "binary_sha256": file_sha256(binary),
        "fixture": {
            "path": str(fixture),
            "sha256": fixture_sha256,
            "lines": FIXTURE_LINES,
            "line_bytes": len(FIXTURE_LINE_BYTES),
            "query": QUERY,
        },
        "input": {
            "events": args.events,
            "interval_ms": args.interval_ms,
            "warmups": args.warmups,
            "runs": args.runs,
        },
        "profiles": profiles,
    }
    json.dump(report, fp=sys.stdout, ensure_ascii=False, indent=2)
    print()


if __name__ == "__main__":
    main()
