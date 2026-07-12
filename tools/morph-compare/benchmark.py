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
from statistics import median

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
DEFAULT_DEV_CASES = Path("/opt/morph-benchmark/data/dev-cases.jsonl")
DEFAULT_DEV_METADATA = Path("/opt/morph-benchmark/data/dev-metadata.json")
DEFAULT_HARD_NEGATIVES = Path("/opt/morph-benchmark/hard-negatives.jsonl")
DEFAULT_RUNNER = Path("/usr/local/bin/morph-benchmark-runner")
DEFAULT_RUNS = 5
HARD_NEGATIVE_SLICES = {
    "homonymous-other-pos",
    "compound-substring",
    "attached-predecessor-predicate",
    "same-surface-different-lemma",
    "one-syllable-boundary",
}


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


def validate_hard_negatives(
    cases_path: Path, cases: list[dict[str, object]]
) -> dict[str, object]:
    if not cases or any(case["expected"] for case in cases):
        raise ValueError("hard-negative fixture must contain only negative cases")
    case_ids = {case["id"] for case in cases}
    if len(case_ids) != len(cases):
        raise ValueError("hard-negative case IDs are not unique")
    slices = {str(case.get("slice")) for case in cases}
    if slices != HARD_NEGATIVE_SLICES:
        raise ValueError(
            f"hard-negative slices differ: expected {sorted(HARD_NEGATIVE_SLICES)}, "
            f"got {sorted(slices)}"
        )
    return {
        "schema_version": 1,
        "split": "hard-negative",
        "fixture_sha256": sha256(cases_path),
        "cases": len(cases),
        "positive_cases": 0,
        "negative_cases": len(cases),
        "seed": "version-controlled",
        "ud_release": "n/a",
        "sources": [],
    }


def select_smoke_cases(cases: list[dict[str, object]]) -> list[dict[str, object]]:
    selected_ids = set()
    selected_groups = set()
    for case in cases:
        group = (case["source"], case["pos"], case["expected"])
        if group not in selected_groups:
            selected_groups.add(group)
            selected_ids.add(case["id"])
    return [case for case in cases if case["id"] in selected_ids]


def write_cases(path: Path, cases: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8") as fixture_file:
        for case in cases:
            fixture_file.write(
                json.dumps(case, ensure_ascii=False, sort_keys=True) + "\n"
            )


def smoke_metadata(
    cases_path: Path,
    cases: list[dict[str, object]],
    development_metadata: dict[str, object],
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "split": "dev-smoke",
        "fixture_sha256": sha256(cases_path),
        "cases": len(cases),
        "positive_cases": sum(bool(case["expected"]) for case in cases),
        "negative_cases": sum(not case["expected"] for case in cases),
        "seed": development_metadata["seed"],
        "ud_release": development_metadata["ud_release"],
        "sources": development_metadata["sources"],
    }


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
) -> tuple[
    dict[str, bool],
    dict[str, list[dict[str, object]]],
    dict[str, object],
    dict[str, dict[str, object] | None],
    dict[str, dict[str, object]],
]:
    case_ids = [case["id"] for case in cases]
    result_ids = [result["id"] for result in summary["results"]]
    if result_ids != case_ids:
        raise ValueError(f"{backend} result order differs from fixture order")
    results = {result["id"]: result for result in summary["results"]}
    predictions = {}
    matches = {}
    diagnostics = {}
    shadow_verification = {}
    latencies = []
    for case in cases:
        result = results.get(case["id"])
        if result is None:
            raise ValueError(f"{backend} omitted case {case['id']}")
        spans = result["spans"]
        predictions[case["id"]] = span_prediction(case, spans)
        matches[case["id"]] = spans
        diagnostics[case["id"]] = result["failure_diagnostic"]
        shadow_verification[case["id"]] = result["shadow_verification"]
        latencies.append(float(result["latency_ms"]))
    return (
        predictions,
        matches,
        performance(summary, latencies),
        diagnostics,
        shadow_verification,
    )


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


def aggregate_performance(
    runs: list[dict[str, object]], warmup_runs: int
) -> dict[str, object]:
    if not runs:
        raise ValueError("at least one measured run is required")
    metric_names = (
        "initialization_seconds",
        "evaluation_seconds",
        "cases_per_second",
        "latency_p50_ms",
        "latency_p95_ms",
        "peak_rss_kib",
    )
    result: dict[str, object] = {"runs": len(runs), "warmup_runs": warmup_runs}
    minimum = {}
    maximum = {}
    for name in metric_names:
        values = [run[name] for run in runs if run[name] is not None]
        result[name] = median(values) if values else None
        minimum[name] = min(values) if values else None
        maximum[name] = max(values) if values else None
    result["run_min"] = minimum
    result["run_max"] = maximum
    return result


def evaluate_kfind_runs(
    cases: list[dict[str, object]],
    runner: Path,
    profile: str,
    cases_path: Path,
    runs: int,
    warmup: bool,
) -> tuple[
    dict[str, bool],
    dict[str, list[dict[str, object]]],
    dict[str, object],
    dict[str, dict[str, object] | None],
    dict[str, dict[str, object]],
    dict[str, object],
]:
    if warmup:
        run_native_backend(runner, profile, cases_path)
    evaluations = []
    summaries = []
    for _ in range(runs):
        summary = run_native_backend(runner, profile, cases_path)
        summaries.append(summary)
        evaluations.append(evaluate_kfind(cases, summary, profile))
    first = evaluations[0]
    for evaluation in evaluations[1:]:
        if evaluation[0] != first[0] or evaluation[1] != first[1]:
            raise ValueError(f"{profile} predictions changed between measured runs")
        if evaluation[3] != first[3]:
            raise ValueError(f"{profile} diagnostics changed between measured runs")
        if evaluation[4] != first[4]:
            raise ValueError(
                f"{profile} shadow verification changed between measured runs"
            )
    return (
        first[0],
        first[1],
        aggregate_performance(
            [evaluation[2] for evaluation in evaluations], int(warmup)
        ),
        first[3],
        first[4],
        summaries[0],
    )


def evaluate_lindera_runs(
    cases: list[dict[str, object]],
    runner: Path,
    cases_path: Path,
    runs: int,
    warmup: bool,
) -> tuple[
    dict[str, bool],
    dict[str, list[dict[str, object]]],
    dict[str, object],
    dict[str, object],
]:
    if warmup:
        run_native_backend(runner, "lindera", cases_path)
    evaluations = []
    summaries = []
    for _ in range(runs):
        summary = run_native_backend(runner, "lindera", cases_path)
        summaries.append(summary)
        evaluations.append(evaluate_lindera(cases, summary))
    first = evaluations[0]
    for evaluation in evaluations[1:]:
        if evaluation[0] != first[0] or evaluation[1] != first[1]:
            raise ValueError("Lindera predictions changed between measured runs")
    return (
        first[0],
        first[1],
        aggregate_performance(
            [evaluation[2] for evaluation in evaluations], int(warmup)
        ),
        summaries[0],
    )


def evaluate_kiwi_runs(
    cases: list[dict[str, object]], runs: int, warmup: bool
) -> tuple[
    dict[str, bool],
    dict[str, list[dict[str, object]]],
    dict[str, object],
]:
    if warmup:
        evaluate_kiwi(cases)
    evaluations = [evaluate_kiwi(cases) for _ in range(runs)]
    first = evaluations[0]
    for evaluation in evaluations[1:]:
        if evaluation[0] != first[0] or evaluation[1] != first[1]:
            raise ValueError("Kiwi predictions changed between measured runs")
    return (
        first[0],
        first[1],
        aggregate_performance(
            [evaluation[2] for evaluation in evaluations], int(warmup)
        ),
    )


def evaluate_dataset(
    cases: list[dict[str, object]],
    cases_path: Path,
    runner: Path,
    runs: int,
    warmup: bool,
) -> dict[str, object]:
    kfind = {
        profile: evaluate_kfind_runs(
            cases, runner, profile, cases_path, runs, warmup
        )
        for profile in KFIND_PROFILES
    }
    lindera = evaluate_lindera_runs(cases, runner, cases_path, runs, warmup)
    kiwi = evaluate_kiwi_runs(cases, runs, warmup)
    versions = {
        profile: {
            "backend": kfind[profile][5]["backend"],
            "version": kfind[profile][5]["version"],
            "profile": kfind[profile][5]["profile"],
            "lexicon_artifact_sha256": kfind[profile][5][
                "lexicon_artifact_sha256"
            ],
        }
        for profile in KFIND_PROFILES
    }
    versions.update(
        {
            "kiwi": {
                "backend": "kiwi",
                "version": importlib.metadata.version("kiwipiepy"),
                "profile": None,
                "lexicon_artifact_sha256": None,
            },
            "lindera": {
                "backend": lindera[3]["backend"],
                "version": lindera[3]["version"],
                "profile": None,
                "lexicon_artifact_sha256": None,
            },
        }
    )
    return {
        "versions": versions,
        "predictions": {profile: kfind[profile][0] for profile in KFIND_PROFILES}
        | {"kiwi": kiwi[0], "lindera": lindera[0]},
        "matches": {profile: kfind[profile][1] for profile in KFIND_PROFILES}
        | {"kiwi": kiwi[1], "lindera": lindera[1]},
        "performance": {profile: kfind[profile][2] for profile in KFIND_PROFILES}
        | {"kiwi": kiwi[2], "lindera": lindera[2]},
        "diagnostics": {profile: kfind[profile][3] for profile in KFIND_PROFILES},
        "shadow_verification": {
            profile: kfind[profile][4] for profile in KFIND_PROFILES
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--dev-cases", type=Path, default=DEFAULT_DEV_CASES)
    parser.add_argument("--dev-metadata", type=Path, default=DEFAULT_DEV_METADATA)
    parser.add_argument("--hard-negatives", type=Path, default=DEFAULT_HARD_NEGATIVES)
    parser.add_argument("--runner", type=Path, default=DEFAULT_RUNNER)
    parser.add_argument("--runs", type=int, default=DEFAULT_RUNS)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--output", type=Path, default=Path("/output/report.json"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.runs < 1:
            raise ValueError("--runs must be at least 1")
        cases = load_cases(args.cases)
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        validate_dataset(args.cases, cases, metadata)
        dev_cases = load_cases(args.dev_cases)
        dev_metadata = json.loads(args.dev_metadata.read_text(encoding="utf-8"))
        validate_dataset(args.dev_cases, dev_cases, dev_metadata)
        hard_cases = load_cases(args.hard_negatives)
        hard_metadata = validate_hard_negatives(args.hard_negatives, hard_cases)

        if args.smoke:
            with tempfile.TemporaryDirectory() as directory:
                smoke_path = Path(directory) / "smoke-cases.jsonl"
                smoke_cases = select_smoke_cases(dev_cases)
                write_cases(smoke_path, smoke_cases)
                baseline = evaluate_dataset(
                    smoke_cases, smoke_path, args.runner, 1, True
                )
                report = build_report(
                    smoke_cases,
                    smoke_metadata(smoke_path, smoke_cases, dev_metadata),
                    baseline["versions"],
                    baseline["predictions"],
                    baseline["matches"],
                    baseline["performance"],
                    baseline["diagnostics"],
                    baseline["shadow_verification"],
                )
                return write_report(args.output, report)

        baseline = evaluate_dataset(cases, args.cases, args.runner, args.runs, True)
        development = evaluate_dataset(dev_cases, args.dev_cases, args.runner, 1, False)
        hard_negatives = evaluate_dataset(
            hard_cases, args.hard_negatives, args.runner, 1, False
        )
        report = build_report(
            cases,
            metadata,
            baseline["versions"],
            baseline["predictions"],
            baseline["matches"],
            baseline["performance"],
            baseline["diagnostics"],
            baseline["shadow_verification"],
        )
        report["development"] = build_report(
            dev_cases,
            dev_metadata,
            development["versions"],
            development["predictions"],
            development["matches"],
            development["performance"],
            development["diagnostics"],
            development["shadow_verification"],
        )
        report["hard_negatives"] = build_report(
            hard_cases,
            hard_metadata,
            hard_negatives["versions"],
            hard_negatives["predictions"],
            hard_negatives["matches"],
            hard_negatives["performance"],
            hard_negatives["diagnostics"],
            hard_negatives["shadow_verification"],
        )
        return write_report(args.output, report)
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"benchmark failed: {error}", file=sys.stderr)
        return 2


def write_report(output: Path, report: dict[str, object]) -> int:
    markdown = render_markdown(report)
    print(markdown, end="")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    output.with_suffix(".md").write_text(markdown, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
