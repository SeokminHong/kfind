#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import resource
import subprocess
import sys
import tempfile
import time
from collections import defaultdict
from pathlib import Path

from kiwipiepy import Kiwi

from python.adapters import (
    CandidateSpan,
    candidate_prediction,
    kiwi_candidates,
    lindera_candidates,
    spans_overlap,
)
from python.report import KFIND_PROFILES, build_report, render_markdown


DEFAULT_CASES = Path("/opt/morph-benchmark/data/cases.jsonl")
DEFAULT_METADATA = Path("/opt/morph-benchmark/data/metadata.json")
DEFAULT_RUNNER = Path("/usr/local/bin/morph-benchmark-runner")
def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_cases(path: Path) -> list[dict[str, object]]:
    with path.open(encoding="utf-8") as fixture_file:
        return [json.loads(line) for line in fixture_file if line.strip()]


def validate_dataset(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if len(cases) != 1_000 or metadata["cases"] != 1_000:
        raise ValueError("benchmark requires exactly 1,000 cases")
    if sha256(cases_path) != metadata["fixture_sha256"]:
        raise ValueError("fixture SHA-256 does not match metadata")
    expected_ids = {case["id"] for case in cases}
    if len(expected_ids) != len(cases):
        raise ValueError("benchmark case IDs are not unique")
    positives = sum(bool(case["expected"]) for case in cases)
    if positives != 500:
        raise ValueError(f"benchmark requires 500 positive cases, got {positives}")
    counts: dict[tuple[str, str, bool], int] = defaultdict(int)
    for case in cases:
        counts[(str(case["source"]), str(case["pos"]), bool(case["expected"]))] += 1
    quotas = metadata["positive_quotas_per_source"]
    for source in metadata["sources"]:
        for pos, quota in quotas.items():
            for expected in (True, False):
                actual = counts[(source["name"], pos, expected)]
                if actual != quota:
                    raise ValueError(
                        f"quota mismatch for {source['name']}/{pos}/{expected}: "
                        f"expected {quota}, got {actual}"
                    )


def run_native_backend(
    runner: Path, backend: str, cases_path: Path
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / f"{backend}.json"
        result = subprocess.run(
            [str(runner), backend, str(cases_path), str(output)],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"{backend} runner failed with exit {result.returncode}: "
                f"{result.stderr.strip()}"
            )
        return json.loads(output.read_text(encoding="utf-8"))


def span_prediction(case: dict[str, object], spans: list[dict[str, int]]) -> bool:
    if not case["expected"]:
        return bool(spans)
    gold_start = case["gold_byte_start"]
    gold_end = case["gold_byte_end"]
    if gold_start is None or gold_end is None:
        raise ValueError(f"positive case {case['id']} has no gold span")
    return any(
        spans_overlap(span["byte_start"], span["byte_end"], gold_start, gold_end)
        for span in spans
    )


def matching_candidate_spans(
    case: dict[str, object], candidates: set[CandidateSpan]
) -> list[dict[str, object]]:
    return [
        {
            "byte_start": item.byte_start,
            "byte_end": item.byte_end,
            "raw_tag": item.raw_tag,
        }
        for item in sorted(
            candidates,
            key=lambda item: (item.byte_start, item.byte_end, item.raw_tag),
        )
        if item.lemma == case["query"] and item.pos == case["pos"]
    ]


def evaluate_kfind(
    cases: list[dict[str, object]], summary: dict[str, object], backend: str
) -> tuple[dict[str, bool], dict[str, list[dict[str, object]]], dict[str, object]]:
    case_ids = [case["id"] for case in cases]
    result_ids = [result["id"] for result in summary["results"]]
    if result_ids != case_ids:
        raise ValueError(f"{backend} result order differs from fixture order")
    results = {result["id"]: result for result in summary["results"]}
    predictions = {}
    matches = {}
    latencies = []
    for case in cases:
        result = results.get(case["id"])
        if result is None:
            raise ValueError(f"{backend} omitted case {case['id']}")
        spans = result["spans"]
        predictions[case["id"]] = span_prediction(case, spans)
        matches[case["id"]] = spans
        latencies.append(float(result["latency_ms"]))
    return predictions, matches, performance(summary, latencies)


def evaluate_lindera(
    cases: list[dict[str, object]], summary: dict[str, object]
) -> tuple[dict[str, bool], dict[str, list[dict[str, object]]], dict[str, object]]:
    results = {result["id"]: result for result in summary["results"]}
    predictions = {}
    matches = {}
    latencies = []
    postprocess_seconds = 0.0
    for case in cases:
        result = results.get(case["id"])
        if result is None:
            raise ValueError(f"Lindera omitted case {case['id']}")
        postprocess_started = time.perf_counter()
        candidates = lindera_candidates(result["tokens"])
        predictions[case["id"]] = candidate_prediction(
            str(case["query"]),
            str(case["pos"]),
            bool(case["expected"]),
            case["gold_byte_start"],
            case["gold_byte_end"],
            candidates,
        )
        matches[case["id"]] = matching_candidate_spans(case, candidates)
        postprocess_ms = (time.perf_counter() - postprocess_started) * 1_000.0
        postprocess_seconds += postprocess_ms / 1_000.0
        latencies.append(float(result["latency_ms"]) + postprocess_ms)
    summary = dict(summary)
    summary["evaluation_seconds"] = (
        float(summary["evaluation_seconds"]) + postprocess_seconds
    )
    return predictions, matches, performance(summary, latencies)


def evaluate_kiwi(
    cases: list[dict[str, object]],
) -> tuple[
    dict[str, bool],
    dict[str, list[dict[str, object]]],
    dict[str, object],
]:
    initialization_started = time.perf_counter()
    kiwi = Kiwi()
    kiwi.tokenize("")
    initialization_seconds = time.perf_counter() - initialization_started
    predictions = {}
    matches = {}
    latencies = []
    evaluation_started = time.perf_counter()
    for case in cases:
        case_started = time.perf_counter()
        candidates = kiwi_candidates(str(case["text"]), kiwi.tokenize(str(case["text"])))
        predictions[case["id"]] = candidate_prediction(
            str(case["query"]),
            str(case["pos"]),
            bool(case["expected"]),
            case["gold_byte_start"],
            case["gold_byte_end"],
            candidates,
        )
        matches[case["id"]] = matching_candidate_spans(case, candidates)
        latencies.append((time.perf_counter() - case_started) * 1_000.0)
    summary = {
        "initialization_seconds": initialization_seconds,
        "evaluation_seconds": time.perf_counter() - evaluation_started,
        "peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
    }
    return predictions, matches, performance(summary, latencies)


def percentile(values: list[float], percentile_value: float) -> float:
    ordered = sorted(values)
    index = max(0, math.ceil(percentile_value * len(ordered)) - 1)
    return ordered[index]


def performance(summary: dict[str, object], latencies: list[float]) -> dict[str, object]:
    evaluation_seconds = float(summary["evaluation_seconds"])
    return {
        "initialization_seconds": round(float(summary["initialization_seconds"]), 6),
        "evaluation_seconds": round(evaluation_seconds, 6),
        "cases_per_second": round(len(latencies) / evaluation_seconds, 1),
        "latency_p50_ms": round(percentile(latencies, 0.50), 4),
        "latency_p95_ms": round(percentile(latencies, 0.95), 4),
        "peak_rss_kib": summary["peak_rss_kib"],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--runner", type=Path, default=DEFAULT_RUNNER)
    parser.add_argument("--output", type=Path, default=Path("/output/report.json"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        cases = load_cases(args.cases)
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        validate_dataset(args.cases, cases, metadata)
        kfind_summaries = {
            profile: run_native_backend(args.runner, profile, args.cases)
            for profile in KFIND_PROFILES
        }
        lindera_summary = run_native_backend(args.runner, "lindera", args.cases)
        kfind_results = {
            profile: evaluate_kfind(cases, kfind_summaries[profile], profile)
            for profile in KFIND_PROFILES
        }
        kiwi_predictions, kiwi_matches, kiwi_performance = evaluate_kiwi(cases)
        lindera_predictions, lindera_matches, lindera_performance = evaluate_lindera(
            cases, lindera_summary
        )
        report = build_report(
            cases,
            metadata,
            {
                profile: {
                    "backend": kfind_summaries[profile]["backend"],
                    "version": kfind_summaries[profile]["version"],
                    "profile": kfind_summaries[profile]["profile"],
                    "lexicon_artifact_sha256": kfind_summaries[profile][
                        "lexicon_artifact_sha256"
                    ],
                }
                for profile in KFIND_PROFILES
            }
            | {
                "kiwi": {
                    "backend": "kiwi",
                    "version": importlib.metadata.version("kiwipiepy"),
                    "profile": None,
                    "lexicon_artifact_sha256": None,
                },
                "lindera": {
                    "backend": lindera_summary["backend"],
                    "version": lindera_summary["version"],
                    "profile": None,
                    "lexicon_artifact_sha256": None,
                },
            },
            {
                profile: kfind_results[profile][0] for profile in KFIND_PROFILES
            }
            | {
                "kiwi": kiwi_predictions,
                "lindera": lindera_predictions,
            },
            {
                profile: kfind_results[profile][1] for profile in KFIND_PROFILES
            }
            | {
                "kiwi": kiwi_matches,
                "lindera": lindera_matches,
            },
            {
                profile: kfind_results[profile][2] for profile in KFIND_PROFILES
            }
            | {
                "kiwi": kiwi_performance,
                "lindera": lindera_performance,
            },
        )
        markdown = render_markdown(report)
        print(markdown, end="")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        args.output.with_suffix(".md").write_text(markdown, encoding="utf-8")
        return 0
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"benchmark failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
