#!/usr/bin/env python3
"""Compare native CLI phrase search and measure grouped alternatives."""

import argparse
import hashlib
import json
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def measure(binary: Path, query: str, corpus: bytes, expected_lines: int) -> tuple[int, bytes]:
    command = [
        str(binary),
        "--embedded",
        "--literal",
        "--boundary", "token",
        "--max-gap", "1",
        "--no-pager",
        "--json",
        query,
    ]
    start = time.perf_counter_ns()
    result = subprocess.run(command, input=corpus, capture_output=True, check=False)
    elapsed = time.perf_counter_ns() - start
    if result.returncode != 0 or result.stderr:
        raise RuntimeError(f"{binary}: exit {result.returncode}: {result.stderr.decode(errors='replace')}")
    if len(result.stdout.splitlines()) != expected_lines:
        raise RuntimeError(f"{binary}: expected {expected_lines} JSON lines")
    return elapsed, result.stdout


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

    corpus = ("사과 가격\n배 품질\n" * 512).encode()
    common_query = "사과 가격"
    grouped_query = "(사과 | 배) (가격 | 품질)"
    baseline = args.baseline.resolve(strict=True)
    candidate = args.candidate.resolve(strict=True)
    common_samples = {"baseline": [], "candidate": []}
    grouped_samples = []

    for index in range(args.warmup + args.runs):
        order = [("baseline", baseline), ("candidate", candidate)]
        if index % 2:
            order.reverse()
        outputs = {}
        for label, binary in order:
            elapsed, output = measure(binary, common_query, corpus, 512)
            outputs[label] = output
            if index >= args.warmup:
                common_samples[label].append(elapsed)
        if outputs["baseline"] != outputs["candidate"]:
            raise RuntimeError("common phrase output differs between revisions")
        elapsed, grouped_output = measure(candidate, grouped_query, corpus, 1_024)
        if index >= args.warmup:
            grouped_samples.append(elapsed)

    first_grouped = json.loads(grouped_output.splitlines()[0])
    if [span["atom"] for span in first_grouped["spans"]] != [0, 2]:
        raise RuntimeError("grouped JSON lost selected query atom indices")

    report = {
        "baseline_revision": args.baseline_revision,
        "candidate_revision": args.candidate_revision,
        "environment": {"platform": platform.platform(), "python": platform.python_version()},
        "warmup": args.warmup,
        "runs": args.runs,
        "input_bytes": len(corpus),
        "input_sha256": digest(corpus),
        "binary_sha256": {"baseline": digest(baseline.read_bytes()), "candidate": digest(candidate.read_bytes())},
        "workloads": {
            "phrase_json": {label: summarize(samples) for label, samples in common_samples.items()},
            "grouped_json": {"candidate": summarize(grouped_samples)},
        },
        "samples_ns": {
            "phrase_json": common_samples,
            "grouped_json": grouped_samples,
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report["workloads"], indent=2))


if __name__ == "__main__":
    main()
