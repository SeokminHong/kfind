#!/usr/bin/env python3

import argparse
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import time
from pathlib import Path


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return parsed


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Measure fresh-process agent hook latency against a CLI startup control."
    )
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--warmups", type=positive_int, default=10)
    parser.add_argument("--runs", type=positive_int, default=200)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def payload(event: str, tool: str, command: str) -> bytes:
    document = {
        "hook_event_name": event,
        "tool_name": tool,
        "tool_input": {"command": command},
    }
    return json.dumps(document, ensure_ascii=False, separators=(",", ":")).encode()


CODEX_ALLOW = payload("PreToolUse", "Bash", "rg TODO crates")
CODEX_DENY = payload("PreToolUse", "Bash", "rg 사용자 crates")
GEMINI_DENY = payload("BeforeTool", "run_shell_command", "grep 검색 docs")


def verify_version(completed: subprocess.CompletedProcess[bytes]) -> None:
    if completed.returncode != 0:
        raise RuntimeError(f"version command exited {completed.returncode}")
    if not completed.stdout.startswith(b"kfind "):
        raise RuntimeError(f"unexpected version output: {completed.stdout!r}")
    if completed.stderr:
        raise RuntimeError(f"unexpected version diagnostics: {completed.stderr!r}")


def verify_codex_allow(completed: subprocess.CompletedProcess[bytes]) -> None:
    if completed.returncode != 0:
        raise RuntimeError(f"Codex allow hook exited {completed.returncode}")
    if completed.stdout or completed.stderr:
        raise RuntimeError("Codex allow hook must be silent")


def decoded_output(completed: subprocess.CompletedProcess[bytes]) -> dict:
    if completed.returncode != 0:
        raise RuntimeError(f"deny hook exited {completed.returncode}")
    if completed.stderr:
        raise RuntimeError(f"unexpected hook diagnostics: {completed.stderr!r}")
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("hook output is not valid JSON") from error


def verify_codex_deny(completed: subprocess.CompletedProcess[bytes]) -> None:
    document = decoded_output(completed)
    output = document.get("hookSpecificOutput", {})
    if output.get("permissionDecision") != "deny":
        raise RuntimeError(f"unexpected Codex denial: {document!r}")


def verify_gemini_deny(completed: subprocess.CompletedProcess[bytes]) -> None:
    document = decoded_output(completed)
    if document.get("decision") != "deny":
        raise RuntimeError(f"unexpected Gemini denial: {document!r}")


class Workload:
    def __init__(self, name, binary, arguments, stdin, verify):
        self.name = name
        self.binary = binary
        self.arguments = arguments
        self.stdin = stdin
        self.verify = verify
        self.samples_ns = []

    def run(self, measured: bool) -> None:
        started = time.perf_counter_ns()
        completed = subprocess.run(
            [self.binary, *self.arguments],
            input=self.stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        elapsed = time.perf_counter_ns() - started
        self.verify(completed)
        if measured:
            self.samples_ns.append(elapsed)

    def summary(self) -> dict:
        samples_ms = sorted(sample / 1_000_000 for sample in self.samples_ns)
        p95_index = math.ceil(len(samples_ms) * 0.95) - 1
        return {
            "command": [str(self.binary), *self.arguments],
            "fresh_process": True,
            "runs": len(samples_ms),
            "latency_ms": {
                "median": round(statistics.median(samples_ms), 6),
                "min": round(samples_ms[0], 6),
                "max": round(samples_ms[-1], 6),
                "p95": round(samples_ms[p95_index], 6),
            },
        }


def validate_binary(path: Path, label: str) -> Path:
    resolved = path.resolve()
    if not resolved.is_file():
        raise SystemExit(f"{label} binary does not exist: {resolved}")
    if not os.access(resolved, os.X_OK):
        raise SystemExit(f"{label} binary is not executable: {resolved}")
    return resolved


def main() -> None:
    args = parse_args()
    baseline = validate_binary(args.baseline, "baseline")
    candidate = validate_binary(args.candidate, "candidate")
    workloads = [
        Workload("baseline_version", baseline, ["--version"], None, verify_version),
        Workload("candidate_version", candidate, ["--version"], None, verify_version),
        Workload(
            "candidate_codex_allow",
            candidate,
            ["--agent-hook"],
            CODEX_ALLOW,
            verify_codex_allow,
        ),
        Workload(
            "candidate_codex_deny",
            candidate,
            ["--agent-hook"],
            CODEX_DENY,
            verify_codex_deny,
        ),
        Workload(
            "candidate_gemini_deny",
            candidate,
            ["--agent-hook"],
            GEMINI_DENY,
            verify_gemini_deny,
        ),
    ]

    for round_index in range(args.warmups):
        for offset in range(len(workloads)):
            workloads[(round_index + offset) % len(workloads)].run(measured=False)
    for round_index in range(args.runs):
        for offset in range(len(workloads)):
            workloads[(round_index + offset) % len(workloads)].run(measured=True)

    report = {
        "schema": 1,
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "measurement": {
            "clock": "time.perf_counter_ns",
            "warmup_runs_per_workload": args.warmups,
            "measured_runs_per_workload": args.runs,
            "ordering": "round-robin rotation",
        },
        "binaries": {
            "baseline": {
                "path": str(baseline),
                "revision": args.baseline_revision,
                "sha256": sha256(baseline),
                "size_bytes": baseline.stat().st_size,
            },
            "candidate": {
                "path": str(candidate),
                "revision": args.candidate_revision,
                "sha256": sha256(candidate),
                "size_bytes": candidate.stat().st_size,
            },
        },
        "inputs": {
            "codex_allow_sha256": hashlib.sha256(CODEX_ALLOW).hexdigest(),
            "codex_deny_sha256": hashlib.sha256(CODEX_DENY).hexdigest(),
            "gemini_deny_sha256": hashlib.sha256(GEMINI_DENY).hexdigest(),
        },
        "workloads": {workload.name: workload.summary() for workload in workloads},
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
