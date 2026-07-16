#!/usr/bin/env python3

from __future__ import annotations

import argparse
import base64
import importlib.metadata
import json
import os
import platform
import resource
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from benchmark import (
    DEFAULT_CASES,
    DEFAULT_METADATA,
    aggregate_performance,
    matching_candidate_spans,
    percentile,
    performance as native_performance,
)
from python.adapters import (
    CandidateSpan,
    komoran_candidates,
    kiwi_candidates,
    lindera_candidates,
    mecab_candidates,
)
from python.external_baselines import EXTERNAL_BACKENDS, SCHEMA_VERSION
from python.validation import (
    load_cases,
    validate_dataset,
    validate_query_matrix_dataset,
)


KOMORAN_CLASSPATH = "/opt/morph-benchmark/external:/opt/morph-benchmark/external/komoran.jar"
DEFAULT_LINDERA_RUNNER = Path("/usr/local/bin/lindera-benchmark-runner")
DEFAULT_RUNS = 5


def results_from_candidates(
    cases: list[dict[str, object]],
    candidates_by_id: dict[str, set[CandidateSpan]],
) -> list[dict[str, object]]:
    return [
        {
            "id": case["id"],
            "matching_spans": matching_candidate_spans(
                case, candidates_by_id[str(case["id"])]
            ),
        }
        for case in cases
    ]


def performance_metrics(
    initialization_seconds: float,
    evaluation_seconds: float,
    latencies_ms: list[float],
    peak_rss_kib: int,
) -> dict[str, object]:
    return {
        "initialization_seconds": round(initialization_seconds, 6),
        "evaluation_seconds": round(evaluation_seconds, 6),
        "cases_per_second": round(len(latencies_ms) / evaluation_seconds, 1),
        "latency_p50_ms": round(percentile(latencies_ms, 0.50), 4),
        "latency_p95_ms": round(percentile(latencies_ms, 0.95), 4),
        "peak_rss_kib": peak_rss_kib,
    }


def capture_python_backend(
    cases: list[dict[str, object]],
    initialize: Callable[[], Any],
    analyze: Callable[[Any, str], set[CandidateSpan]],
    version: str,
    configuration: dict[str, object],
) -> dict[str, object]:
    initialization_started = time.perf_counter()
    backend = initialize()
    initialization_seconds = time.perf_counter() - initialization_started
    results = []
    latencies_ms = []
    evaluation_started = time.perf_counter()
    for case in cases:
        case_started = time.perf_counter()
        candidates = analyze(backend, str(case["text"]))
        matching_spans = matching_candidate_spans(case, candidates)
        latencies_ms.append((time.perf_counter() - case_started) * 1_000)
        results.append({"id": case["id"], "matching_spans": matching_spans})
    evaluation_seconds = time.perf_counter() - evaluation_started
    peak_rss_kib = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return available(
        version,
        configuration,
        results,
        performance_metrics(
            initialization_seconds,
            evaluation_seconds,
            latencies_ms,
            peak_rss_kib,
        ),
    )


def capture_kiwi(cases: list[dict[str, object]]) -> dict[str, object]:
    from kiwipiepy import Kiwi

    def initialize() -> Any:
        kiwi = Kiwi()
        kiwi.tokenize("")
        return kiwi

    return capture_python_backend(
        cases,
        initialize,
        lambda kiwi, text: kiwi_candidates(text, kiwi.tokenize(text)),
        importlib.metadata.version("kiwipiepy"),
        {"model": importlib.metadata.version("kiwipiepy_model")},
    )


def capture_lindera(
    cases: list[dict[str, object]], cases_path: Path, lindera_runner: Path
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "lindera.json"
        process = subprocess.run(
            [str(lindera_runner), str(cases_path), str(output)],
            text=True,
            capture_output=True,
        )
        if process.returncode != 0:
            raise RuntimeError(
                f"lindera runner failed with exit {process.returncode}: "
                f"{process.stderr.strip()}"
            )
        summary = json.loads(output.read_text(encoding="utf-8"))
    results = {str(result["id"]): result for result in summary["results"]}
    candidates = {
        str(case["id"]): lindera_candidates(results[str(case["id"])]["tokens"])
        for case in cases
    }
    return available(
        str(summary["version"]),
        {"dictionary": "embedded-ko-dic"},
        results_from_candidates(cases, candidates),
        native_performance(
            summary,
            [float(result["latency_ms"]) for result in summary["results"]],
        ),
    )


def capture_mecab(cases: list[dict[str, object]]) -> dict[str, object]:
    import mecab_ko

    def initialize() -> Any:
        tagger = mecab_ko.Tagger()
        tagger.parse("")
        return tagger

    return capture_python_backend(
        cases,
        initialize,
        lambda tagger, text: mecab_candidates(text, tagger.parse(text)),
        importlib.metadata.version("mecab-ko"),
        {"dictionary": importlib.metadata.version("mecab-ko-dic")},
    )


def capture_komoran(cases: list[dict[str, object]]) -> dict[str, object]:
    encoded = "\n".join(
        base64.b64encode(str(case["text"]).encode()).decode() for case in cases
    )
    process_started = time.perf_counter()
    process = subprocess.run(
        ["java", "-cp", KOMORAN_CLASSPATH, "KomoranRunner"],
        input=encoded + "\n",
        text=True,
        capture_output=True,
    )
    process_seconds = time.perf_counter() - process_started
    if process.returncode != 0:
        raise RuntimeError(f"KOMORAN failed: {process.stderr.strip()}")
    performance_lines = [
        line for line in process.stderr.splitlines() if line.startswith("KFIND_PERF\t")
    ]
    if len(performance_lines) != 1:
        raise ValueError("KOMORAN did not return one performance summary")
    _, initialization_ns, evaluation_ns, encoded_latencies = performance_lines[0].split(
        "\t"
    )
    java_evaluation_seconds = int(evaluation_ns) / 1_000_000_000
    latencies_ms = [
        int(value) / 1_000_000 for value in filter(None, encoded_latencies.split(","))
    ]
    output_lines = process.stdout.splitlines()
    if len(output_lines) != len(cases):
        raise ValueError("KOMORAN result count differs from fixture")
    parsing_started = time.perf_counter()
    results = []
    for case, line in zip(cases, output_lines):
        tokens = []
        for encoded_token in filter(None, line.split(";")):
            morph, pos, begin, end = encoded_token.split(",")
            tokens.append(
                {
                    "morph": base64.b64decode(morph).decode(),
                    "pos": pos,
                    "begin": int(begin),
                    "end": int(end),
                }
            )
        candidates = komoran_candidates(str(case["text"]), tokens)
        results.append(
            {
                "id": case["id"],
                "matching_spans": matching_candidate_spans(case, candidates),
            }
        )
    parsing_seconds = time.perf_counter() - parsing_started
    if len(latencies_ms) != len(cases):
        raise ValueError("KOMORAN latency count differs from fixture")
    per_case_parsing_ms = parsing_seconds * 1_000 / len(cases)
    latencies_ms = [value + per_case_parsing_ms for value in latencies_ms]
    evaluation_seconds = java_evaluation_seconds + parsing_seconds
    initialization_seconds = max(
        int(initialization_ns) / 1_000_000_000,
        process_seconds - java_evaluation_seconds,
    )
    peak_rss_kib = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    return available(
        "3.3.9",
        {"model": "FULL"},
        results,
        performance_metrics(
            initialization_seconds,
            evaluation_seconds,
            latencies_ms,
            peak_rss_kib,
        ),
    )


def available(
    version: str,
    configuration: dict[str, object],
    results: list[dict[str, object]],
    performance: dict[str, object],
) -> dict[str, object]:
    return {
        "status": "available",
        "version": version,
        "configuration": configuration,
        "results": results,
        "performance": performance,
    }


def capture_once(
    backend: str,
    cases: list[dict[str, object]],
    cases_path: Path,
    lindera_runner: Path,
) -> dict[str, object]:
    captures = {
        "kiwi": lambda: capture_kiwi(cases),
        "lindera": lambda: capture_lindera(cases, cases_path, lindera_runner),
        "mecab-ko": lambda: capture_mecab(cases),
        "komoran": lambda: capture_komoran(cases),
    }
    return captures[backend]()


def capture_backend_runs(
    backend: str,
    cases_path: Path,
    metadata_path: Path,
    lindera_runner: Path,
    runs: int,
) -> dict[str, object]:
    def run_worker() -> dict[str, object]:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / f"{backend}.json"
            process = subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).resolve()),
                    "--worker-backend",
                    backend,
                    "--cases",
                    str(cases_path),
                    "--metadata",
                    str(metadata_path),
                    "--lindera-runner",
                    str(lindera_runner),
                    "--output",
                    str(output),
                ],
                text=True,
                capture_output=True,
            )
            if process.returncode != 0:
                raise RuntimeError(
                    f"{backend} performance worker failed with exit "
                    f"{process.returncode}: {process.stderr.strip()}"
                )
            return json.loads(output.read_text(encoding="utf-8"))

    run_worker()
    measured = [run_worker() for _ in range(runs)]
    first = measured[0]
    for result in measured[1:]:
        if result["version"] != first["version"]:
            raise ValueError(f"{backend} version changed between measured runs")
        if result["configuration"] != first["configuration"]:
            raise ValueError(f"{backend} configuration changed between measured runs")
        if result["results"] != first["results"]:
            raise ValueError(f"{backend} results changed between measured runs")
    return available(
        str(first["version"]),
        dict(first["configuration"]),
        list(first["results"]),
        aggregate_performance(
            [dict(result["performance"]) for result in measured], warmup_runs=1
        ),
    )


def snapshot_environment() -> dict[str, object]:
    return {
        "platform": platform.platform(),
        "logical_cpus": os.cpu_count(),
        "python": platform.python_version(),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument(
        "--lindera-runner", type=Path, default=DEFAULT_LINDERA_RUNNER
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=DEFAULT_RUNS)
    parser.add_argument("--worker-backend", choices=EXTERNAL_BACKENDS)
    parser.add_argument(
        "--backends", default=",".join(EXTERNAL_BACKENDS), help="comma-separated"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    selected = tuple(filter(None, args.backends.split(",")))
    unknown = sorted(set(selected) - set(EXTERNAL_BACKENDS))
    if unknown:
        raise ValueError(f"unknown external backends: {', '.join(unknown)}")
    cases = load_cases(args.cases)
    metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
    if metadata.get("fixture_type") == "query-matrix":
        validate_query_matrix_dataset(
            args.cases, cases, metadata, "explicit-pos"
        )
    else:
        validate_dataset(args.cases, cases, metadata)
    if args.worker_backend is not None:
        result = capture_once(
            args.worker_backend, cases, args.cases, args.lindera_runner
        )
        args.output.write_text(
            json.dumps(result, ensure_ascii=False, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return 0
    backends = {
        backend: {"status": "unavailable", "reason": "snapshot not captured"}
        for backend in EXTERNAL_BACKENDS
    }
    if args.output.exists():
        current = json.loads(args.output.read_text(encoding="utf-8"))
        if (
            current.get("schema_version") == SCHEMA_VERSION
            and current.get("fixture_sha256") == metadata["fixture_sha256"]
        ):
            backends.update(current.get("backends", {}))
    for backend in selected:
        backends[backend] = capture_backend_runs(
            backend,
            args.cases,
            args.metadata,
            args.lindera_runner,
            max(DEFAULT_RUNS, args.runs),
        )
    snapshot = {
        "schema_version": SCHEMA_VERSION,
        "fixture_sha256": metadata["fixture_sha256"],
        "case_count": len(cases),
        "environment": snapshot_environment(),
        "backends": backends,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
