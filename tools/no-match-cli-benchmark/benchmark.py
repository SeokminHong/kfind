#!/usr/bin/env python3
"""Compare native CLI no-match searches and the opt-in retry hint."""

import argparse
import hashlib
import json
import os
import platform
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def measure(binary: Path, input_path: Path, explain: bool) -> int:
    command = [str(binary), "--embedded", "--no-pager"]
    if explain:
        command.append("--explain-no-match")
    command.extend(["걷다", str(input_path)])
    environment = dict(os.environ, LC_ALL="C")
    start = time.perf_counter_ns()
    result = subprocess.run(command, capture_output=True, env=environment, check=False)
    elapsed = time.perf_counter_ns() - start
    if result.returncode != 1 or result.stdout:
        raise RuntimeError(f"unexpected search result from {binary}: {result.returncode}")
    if explain and b"No matches." not in result.stderr:
        raise RuntimeError("candidate did not provide a retry hint")
    if not explain and result.stderr:
        raise RuntimeError(f"unexpected diagnostic from {binary}: {result.stderr!r}")
    return elapsed


def summarize(samples: list[int]) -> dict[str, float]:
    ordered = sorted(samples)
    return {
        "median_ms": statistics.median(ordered) / 1_000_000,
        "min_ms": ordered[0] / 1_000_000,
        "max_ms": ordered[-1] / 1_000_000,
        "p95_ms": ordered[(95 * len(ordered) + 99) // 100 - 1] / 1_000_000,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--runs", type=int, default=20)
    args = parser.parse_args()
    if args.warmup < 1 or args.runs < 5:
        parser.error("warmup must be positive and runs must be at least 5")

    baseline = args.baseline.resolve(strict=True)
    candidate = args.candidate.resolve(strict=True)
    corpus = ("대상이 없습니다.\n" * 512).encode()
    samples = {"baseline": [], "candidate": [], "candidate_explain": []}
    with tempfile.TemporaryDirectory() as directory:
        input_path = Path(directory) / "no-match.txt"
        input_path.write_bytes(corpus)
        for index in range(args.warmup + args.runs):
            order = [("baseline", baseline), ("candidate", candidate)]
            if index % 2:
                order.reverse()
            for label, binary in order:
                elapsed = measure(binary, input_path, explain=False)
                if index >= args.warmup:
                    samples[label].append(elapsed)
            elapsed = measure(candidate, input_path, explain=True)
            if index >= args.warmup:
                samples["candidate_explain"].append(elapsed)

    report = {
        "baseline_revision": args.baseline_revision,
        "candidate_revision": args.candidate_revision,
        "environment": {"platform": platform.platform(), "python": platform.python_version()},
        "warmup": args.warmup,
        "runs": args.runs,
        "input_bytes": len(corpus),
        "input_sha256": digest(corpus),
        "binary_sha256": {
            "baseline": digest(baseline.read_bytes()),
            "candidate": digest(candidate.read_bytes()),
        },
        "workloads": {label: summarize(values) for label, values in samples.items()},
        "samples_ns": samples,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report["workloads"], indent=2))


if __name__ == "__main__":
    main()
