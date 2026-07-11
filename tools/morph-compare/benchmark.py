#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
import platform
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


DEFAULT_CASES = Path("/opt/morph-benchmark/data/cases.jsonl")
DEFAULT_METADATA = Path("/opt/morph-benchmark/data/metadata.json")
DEFAULT_RUNNER = Path("/usr/local/bin/morph-benchmark-runner")
BACKENDS = ("kfind", "kiwi", "lindera")


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
    cases: list[dict[str, object]], summary: dict[str, object]
) -> tuple[dict[str, bool], dict[str, list[dict[str, object]]], dict[str, object]]:
    results = {result["id"]: result for result in summary["results"]}
    predictions = {}
    matches = {}
    latencies = []
    for case in cases:
        result = results.get(case["id"])
        if result is None:
            raise ValueError(f"kfind omitted case {case['id']}")
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


def quality_metrics(
    cases: list[dict[str, object]], predictions: dict[str, bool]
) -> dict[str, object]:
    tp = fp = tn = fn = 0
    for case in cases:
        expected = bool(case["expected"])
        predicted = predictions[case["id"]]
        if expected and predicted:
            tp += 1
        elif expected:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "cases": len(cases),
        "tp": tp,
        "fp": fp,
        "tn": tn,
        "fn": fn,
        "accuracy_percent": round(100 * (tp + tn) / len(cases), 2),
        "precision_percent": round(100 * precision, 2),
        "recall_percent": round(100 * recall, 2),
        "f1_percent": round(100 * f1, 2),
    }


def grouped_quality(
    cases: list[dict[str, object]], predictions: dict[str, bool], key: str
) -> dict[str, dict[str, object]]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        groups[str(case[key])].append(case)
    return {
        name: quality_metrics(group_cases, predictions)
        for name, group_cases in sorted(groups.items())
    }


def build_report(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    versions: dict[str, str],
    predictions: dict[str, dict[str, bool]],
    matches: dict[str, dict[str, list[dict[str, object]]]],
    performance_metrics: dict[str, dict[str, object]],
) -> dict[str, object]:
    quality = {
        backend: {
            "overall": quality_metrics(cases, predictions[backend]),
            "by_source": grouped_quality(cases, predictions[backend], "source"),
            "by_pos": grouped_quality(cases, predictions[backend], "pos"),
        }
        for backend in BACKENDS
    }
    failures = []
    for case in cases:
        backend_predictions = {
            backend: predictions[backend][case["id"]] for backend in BACKENDS
        }
        if all(value == case["expected"] for value in backend_predictions.values()):
            continue
        failures.append(
            {
                "case": case,
                "predictions": backend_predictions,
                "matching_spans": {
                    backend: matches[backend][case["id"]] for backend in BACKENDS
                },
            }
        )
    return {
        "schema_version": 1,
        "task": "sentence lemma/POS presence with positive gold-span overlap",
        "dataset": metadata,
        "versions": versions,
        "environment": environment_metadata(),
        "quality": quality,
        "performance": performance_metrics,
        "failures": failures,
        "adapter_errors": [],
    }


def environment_metadata() -> dict[str, object]:
    memory_kib = None
    meminfo = Path("/proc/meminfo")
    if meminfo.exists():
        total = next(
            (line for line in meminfo.read_text().splitlines() if line.startswith("MemTotal:")),
            None,
        )
        if total is not None:
            memory_kib = int(total.split()[1])
    return {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "logical_cpu_count": os.cpu_count(),
        "memory_kib": memory_kib,
        "python": platform.python_version(),
    }


def render_markdown(report: dict[str, object]) -> str:
    dataset = report["dataset"]
    lines = [
        "# kfind / Kiwi / Lindera held-out morphology benchmark",
        "",
        f"- fixture: `{dataset['fixture_sha256']}`",
        f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
        f"{dataset['negative_cases']} negative)",
        f"- seed: `{dataset['seed']}`",
        f"- UD release: {dataset['ud_release']}",
        f"- environment: `{report['environment']['platform']}` / "
        f"{report['environment']['logical_cpu_count']} logical CPUs",
        "",
        "## Sources",
        "",
        "| source | license | SHA-256 |",
        "| --- | --- | --- |",
    ]
    for source in dataset["sources"]:
        lines.append(
            f"| [{source['name']}]({source['data_url']}) | {source['license']} | "
            f"`{source['data_sha256']}` |"
        )
    lines.extend(
        [
            "",
            "## Overall quality",
            "",
            "| backend | accuracy | precision | recall | F1 | TP | FP | TN | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = report["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['accuracy_percent']}% | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | {metrics['tp']} | "
            f"{metrics['fp']} | {metrics['tn']} | {metrics['fn']} |"
        )
    lines.extend(
        [
            "",
            "## End-to-end performance",
            "",
            "| backend | init | cases/s | p50 | p95 | peak RSS |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = report["performance"][backend]
        rss = metrics["peak_rss_kib"]
        rss_text = f"{rss / 1024:.1f} MiB" if rss is not None else "n/a"
        lines.append(
            f"| {backend} | {metrics['initialization_seconds']:.4f}s | "
            f"{metrics['cases_per_second']} | {metrics['latency_p50_ms']}ms | "
            f"{metrics['latency_p95_ms']}ms | {rss_text} |"
        )
    lines.extend(["", "## Quality by source", ""])
    lines.extend(
        [
            "| source | backend | accuracy | precision | recall | F1 |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    sources = sorted(report["quality"]["kfind"]["by_source"])
    for source in sources:
        for backend in BACKENDS:
            metrics = report["quality"][backend]["by_source"][source]
            lines.append(
                f"| {source} | {backend} | {metrics['accuracy_percent']}% | "
                f"{metrics['precision_percent']}% | {metrics['recall_percent']}% | "
                f"{metrics['f1_percent']}% |"
            )
    lines.extend(["", "## Quality by POS", ""])
    lines.extend(
        [
            "| POS | backend | accuracy | precision | recall | F1 |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    parts_of_speech = sorted(report["quality"]["kfind"]["by_pos"])
    for pos in parts_of_speech:
        for backend in BACKENDS:
            metrics = report["quality"][backend]["by_pos"][pos]
            lines.append(
                f"| {pos} | {backend} | {metrics['accuracy_percent']}% | "
                f"{metrics['precision_percent']}% | {metrics['recall_percent']}% | "
                f"{metrics['f1_percent']}% |"
            )
    lines.extend(
        [
            "",
            f"## Failures ({len(report['failures'])} cases)",
            "",
            "| case | source | query/POS | expected | kfind | Kiwi | Lindera |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for failure in report["failures"][:30]:
        case = failure["case"]
        predicted = failure["predictions"]
        lines.append(
            f"| {case['id']} | {case['source']} | {case['query']}/{case['pos']} | "
            f"{case['expected']} | {predicted['kfind']} | {predicted['kiwi']} | "
            f"{predicted['lindera']} |"
        )
    if len(report["failures"]) > 30:
        lines.extend(["", "The JSON report contains every failure and matching span."])
    lines.extend(
        [
            "",
            "Performance measures each backend's end-to-end search path after one initialization; "
            "it is not a tokenizer-only throughput comparison.",
        ]
    )
    return "\n".join(lines) + "\n"


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
        kfind_summary = run_native_backend(args.runner, "kfind", args.cases)
        lindera_summary = run_native_backend(args.runner, "lindera", args.cases)
        kfind_predictions, kfind_matches, kfind_performance = evaluate_kfind(
            cases, kfind_summary
        )
        kiwi_predictions, kiwi_matches, kiwi_performance = evaluate_kiwi(cases)
        lindera_predictions, lindera_matches, lindera_performance = evaluate_lindera(
            cases, lindera_summary
        )
        report = build_report(
            cases,
            metadata,
            {
                "kfind": f"kfind {kfind_summary['version']}",
                "kiwi": importlib.metadata.version("kiwipiepy"),
                "lindera": f"lindera {lindera_summary['version']}",
            },
            {
                "kfind": kfind_predictions,
                "kiwi": kiwi_predictions,
                "lindera": lindera_predictions,
            },
            {
                "kfind": kfind_matches,
                "kiwi": kiwi_matches,
                "lindera": lindera_matches,
            },
            {
                "kfind": kfind_performance,
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
