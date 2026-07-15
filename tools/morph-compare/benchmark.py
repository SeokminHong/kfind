#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path
from statistics import median

from python.adapters import (
    CandidateSpan,
    spans_overlap,
)
from python.agent_shadow import build_agent_shadow_report
from python.external_baselines import load_external_baselines
from python.report import (
    KFIND_PROFILES,
    build_report,
    product_persona_comparison,
    quality_metrics,
    product_workflows,
    render_markdown,
    untagged_plan_metrics,
)
from python.validation import (
    load_cases,
    select_smoke_cases,
    smoke_metadata,
    validate_dataset,
    validate_hard_negatives,
    validate_untagged_dataset,
    write_cases,
)
from python.workflows.performance import measure_product_workflows


DEFAULT_CASES = Path("/opt/morph-benchmark/data/cases.jsonl")
DEFAULT_METADATA = Path("/opt/morph-benchmark/data/metadata.json")
DEFAULT_DEV_CASES = Path("/opt/morph-benchmark/data/dev-cases.jsonl")
DEFAULT_DEV_METADATA = Path("/opt/morph-benchmark/data/dev-metadata.json")
DEFAULT_HUMAN_UNTAGGED_CASES = Path(
    "/opt/morph-benchmark/data/human-untagged-cases.jsonl"
)
DEFAULT_HUMAN_UNTAGGED_METADATA = Path(
    "/opt/morph-benchmark/data/human-untagged-metadata.json"
)
DEFAULT_HARD_NEGATIVES = Path("/opt/morph-benchmark/hard-negatives.jsonl")
DEFAULT_EXTERNAL_BASELINES = Path(
    "/opt/morph-benchmark/external-baselines.json"
)
DEFAULT_RUNNER = Path("/usr/local/bin/morph-benchmark-runner")
DEFAULT_RUNS = 5
STARTUP_PROFILES = (
    "embedded",
    "embedded-component",
    "full-pos",
    "full-pos-component",
)
BOUNDARY_POLICIES = ("smart", "token", "any")
HUMAN_BOUNDARY_POLICIES = ("smart", "any")
CONSTRAINT_POLICIES = (
    "whole",
    "explicit-component",
    "possible-analysis",
    "unambiguous-analysis",
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


def run_native_boundary_profile(
    runner: Path, profile: str, boundary: str, cases_path: Path
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / f"{profile}-{boundary}.json"
        result = subprocess.run(
            [
                str(runner),
                "boundary",
                profile,
                boundary,
                str(cases_path),
                str(output),
            ],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"{profile}/{boundary} runner failed with exit {result.returncode}: "
                f"{result.stderr.strip()}"
            )
        return json.loads(output.read_text(encoding="utf-8"))


def run_native_untagged_profile(
    runner: Path, profile: str, boundary: str, cases_path: Path
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / f"untagged-{profile}-{boundary}.json"
        result = subprocess.run(
            [
                str(runner),
                "untagged",
                profile,
                boundary,
                str(cases_path),
                str(output),
            ],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"untagged {profile}/{boundary} runner failed with exit "
                f"{result.returncode}: {result.stderr.strip()}"
            )
        return json.loads(output.read_text(encoding="utf-8"))


def run_native_agent_shadow(
    runner: Path, cases_path: Path
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "agent-shadow.json"
        result = subprocess.run(
            [str(runner), "agent-shadow", str(cases_path), str(output)],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Agent shadow runner failed with exit "
                f"{result.returncode}: {result.stderr.strip()}"
            )
        return json.loads(output.read_text(encoding="utf-8"))


def run_native_constraint_evaluation(
    runner: Path,
    profile: str,
    cases_path: Path,
    verify_diagnostic_parity: bool = False,
) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        product_control = Path(directory) / f"constraint-product-{profile}.json"
        output = Path(directory) / f"constraint-{profile}.json"
        product_result = subprocess.run(
            [
                str(runner),
                "constraint-product-control",
                profile,
                str(cases_path),
                str(product_control),
            ],
            text=True,
            capture_output=True,
        )
        if product_result.returncode != 0:
            raise RuntimeError(
                f"constraint product control {profile} runner failed with exit "
                f"{product_result.returncode}: {product_result.stderr.strip()}"
            )
        result = subprocess.run(
            [
                str(runner),
                "constraint-eval-diagnostic"
                if verify_diagnostic_parity
                else "constraint-eval",
                profile,
                str(cases_path),
                str(product_control),
                str(output),
            ],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"constraint {profile} runner failed with exit "
                f"{result.returncode}: {result.stderr.strip()}"
            )
        return json.loads(output.read_text(encoding="utf-8"))


def run_native_startup_profile(runner: Path, profile: str) -> dict[str, object]:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / f"{profile}.json"
        result = subprocess.run(
            [str(runner), "startup", profile, str(output)],
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"{profile} startup runner failed with exit {result.returncode}: "
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
    return aggregate_metrics(
        runs,
        warmup_runs,
        (
            "initialization_seconds",
            "evaluation_seconds",
            "cases_per_second",
            "latency_p50_ms",
            "latency_p95_ms",
            "peak_rss_kib",
        ),
    )


def aggregate_metrics(
    runs: list[dict[str, object]],
    warmup_runs: int,
    metric_names: tuple[str, ...],
) -> dict[str, object]:
    if not runs:
        raise ValueError("at least one measured run is required")
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


def constraint_performance(summary: dict[str, object]) -> dict[str, object]:
    latencies = [float(result["latency_ms"]) for result in summary["results"]]
    evaluation_seconds = float(summary["evaluation_seconds"])
    return {
        "initialization_seconds": round(float(summary["initialization_seconds"]), 6),
        "evaluation_seconds": round(evaluation_seconds, 6),
        "cases_per_second": round(len(latencies) / evaluation_seconds, 1),
        "latency_p50_ms": round(percentile(latencies, 0.50), 4),
        "latency_p95_ms": round(percentile(latencies, 0.95), 4),
        "peak_rss_kib": summary["peak_rss_kib"],
        "compile_seconds": round(float(summary["compile_seconds"]), 6),
        "candidate_enumeration_seconds": round(
            float(summary["candidate_enumeration_seconds"]), 6
        ),
        "resolver_seconds": round(float(summary["resolver_seconds"]), 6),
        "graph_preparation_seconds": round(
            float(summary["graph_preparation_seconds"]), 6
        ),
        "decision_seconds": round(float(summary["decision_seconds"]), 6),
        "policy_seconds": round(float(summary["policy_seconds"]), 6),
        "diagnostic_seconds": round(float(summary["diagnostic_seconds"]), 6),
        "product_seconds": round(float(summary["product_seconds"]), 6),
    }


def constraint_semantics(summary: dict[str, object]) -> list[dict[str, object]]:
    return [
        {
            "id": result["id"],
            "candidate_covered": result["candidate_covered"],
            "product_prediction": result["product_prediction"],
            "policy_predictions": result["policy_predictions"],
            "candidates": [
                {
                    "status": candidate["status"],
                    "outcome": candidate["outcome"],
                    "evidence": candidate["evidence"],
                    "policies": candidate["policies"],
                    "error": candidate["error"],
                }
                for candidate in result["candidates"]
            ],
        }
        for result in summary["results"]
    ]


def compact_constraint_cases(
    cases: list[dict[str, object]], summary: dict[str, object]
) -> list[dict[str, object]]:
    expected_by_id = {case["id"]: bool(case["expected"]) for case in cases}
    diagnostics = []
    for result in summary["results"]:
        case_id = result["id"]
        expected = expected_by_id[case_id]
        predictions = {
            "product": bool(result["product_prediction"]),
            **result["policy_predictions"],
        }
        notable = (
            (expected and not result["candidate_covered"])
            or len(set(predictions.values())) > 1
            or any(predicted != expected for predicted in predictions.values())
        )
        if not notable:
            continue
        outcome_counts: dict[str, int] = {}
        evidence_counts: dict[str, int] = {}
        status_counts: dict[str, int] = {}
        for candidate in result["candidates"]:
            status = candidate["status"]
            status_counts[status] = status_counts.get(status, 0) + 1
            if candidate["outcome"] is not None:
                outcome = candidate["outcome"]
                outcome_counts[outcome] = outcome_counts.get(outcome, 0) + 1
            for evidence in candidate["evidence"]:
                evidence_counts[evidence] = evidence_counts.get(evidence, 0) + 1
        diagnostics.append(
            {
                "id": case_id,
                "expected": expected,
                "candidate_covered": bool(result["candidate_covered"]),
                "predictions": predictions,
                "candidate_statuses": dict(sorted(status_counts.items())),
                "outcomes": dict(sorted(outcome_counts.items())),
                "evidence": dict(sorted(evidence_counts.items())),
            }
        )
    return diagnostics


def evaluate_constraint_runs(
    cases: list[dict[str, object]],
    cases_path: Path,
    runner: Path,
    profile: str,
    runs: int,
    warmup: bool,
) -> dict[str, object]:
    if warmup:
        run_native_constraint_evaluation(runner, profile, cases_path)
    summaries = [
        run_native_constraint_evaluation(runner, profile, cases_path)
        for _ in range(runs)
    ]
    first = summaries[0]
    case_ids = [case["id"] for case in cases]
    if [result["id"] for result in first["results"]] != case_ids:
        raise ValueError("constraint result order differs from fixture order")
    first_semantics = constraint_semantics(first)
    diagnostic = run_native_constraint_evaluation(
        runner, profile, cases_path, verify_diagnostic_parity=True
    )
    if diagnostic["metrics"] != first["metrics"]:
        raise ValueError("constraint diagnostic metrics differ from compact evaluation")
    if constraint_semantics(diagnostic) != first_semantics:
        raise ValueError("constraint diagnostic decisions differ from compact evaluation")
    for summary in summaries[1:]:
        if summary["metrics"] != first["metrics"]:
            raise ValueError("constraint metrics changed between measured runs")
        if constraint_semantics(summary) != first_semantics:
            raise ValueError("constraint decisions changed between measured runs")
    metric_names = (
        "initialization_seconds",
        "evaluation_seconds",
        "cases_per_second",
        "latency_p50_ms",
        "latency_p95_ms",
        "peak_rss_kib",
        "compile_seconds",
        "candidate_enumeration_seconds",
        "resolver_seconds",
        "graph_preparation_seconds",
        "decision_seconds",
        "policy_seconds",
        "diagnostic_seconds",
        "product_seconds",
    )
    return {
        "version": {
            "backend": first["backend"],
            "version": first["version"],
            "profile": first["profile"],
            "lexicon_artifact_sha256": first["lexicon_artifact_sha256"],
            "enriched_artifact_sha256": first["enriched_artifact_sha256"],
            "component_artifact_sha256": first["component_artifact_sha256"],
            "graph_artifact_sha256": first["graph_artifact_sha256"],
        },
        "metrics": first["metrics"],
        "performance": aggregate_metrics(
            [constraint_performance(summary) for summary in summaries],
            int(warmup),
            metric_names,
        ),
        "case_diagnostics": compact_constraint_cases(cases, first),
    }


def evaluate_component_startup(
    runner: Path, runs: int
) -> dict[str, dict[str, object]]:
    measured_runs = max(3, runs)
    results = {}
    metric_names = (
        "base_initialization_seconds",
        "component_initialization_seconds",
        "initialization_seconds",
        "base_peak_rss_kib",
        "peak_rss_kib",
    )
    for profile in STARTUP_PROFILES:
        run_native_startup_profile(runner, profile)
        summaries = [
            run_native_startup_profile(runner, profile)
            for _ in range(measured_runs)
        ]
        expected_full_pos = profile.startswith("full-pos")
        expected_component = profile.endswith("-component")
        if any(
            summary["profile"] != profile
            or summary["full_pos_loaded"] != expected_full_pos
            or summary["enriched_predicates_loaded"] != expected_full_pos
            or summary["component_resource_loaded"] != expected_component
            for summary in summaries
        ):
            raise ValueError(f"{profile} startup state differs from its profile")
        results[profile] = aggregate_metrics(summaries, 1, metric_names)
    return results


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


def evaluate_boundary_profile_runs(
    cases: list[dict[str, object]],
    runner: Path,
    profile: str,
    boundary: str,
    cases_path: Path,
    runs: int,
) -> tuple[dict[str, bool], dict[str, object], dict[str, object]]:
    run_native_boundary_profile(runner, profile, boundary, cases_path)
    evaluations = []
    summaries = []
    for _ in range(runs):
        summary = run_native_boundary_profile(runner, profile, boundary, cases_path)
        summaries.append(summary)
        evaluations.append(evaluate_kfind(cases, summary, f"{profile}/{boundary}"))
    first = evaluations[0]
    for evaluation in evaluations[1:]:
        if evaluation[0] != first[0] or evaluation[1] != first[1]:
            raise ValueError(
                f"{profile}/{boundary} predictions changed between measured runs"
            )
    expected_component = boundary == "smart"
    if any(
        summary["profile"] != profile
        or summary["boundary"] != boundary
        or (summary["component_artifact_sha256"] is not None)
        != expected_component
        for summary in summaries
    ):
        raise ValueError(f"{profile}/{boundary} resource state differs from its profile")
    return (
        first[0],
        aggregate_performance(
            [evaluation[2] for evaluation in evaluations], warmup_runs=1
        ),
        summaries[0],
    )


def evaluate_boundary_comparison(
    cases: list[dict[str, object]],
    cases_path: Path,
    runner: Path,
    runs: int,
    baseline: dict[str, object],
) -> dict[str, object]:
    profiles = {}
    for backend in KFIND_PROFILES:
        profile = backend.removeprefix("kfind-")
        results = {
            "smart": {
                "quality": quality_metrics(cases, baseline["predictions"][backend]),
                "performance": baseline["performance"][backend],
                "component_resource_loaded": (
                    baseline["versions"][backend]["component_artifact_sha256"]
                    is not None
                ),
            }
        }
        for boundary in BOUNDARY_POLICIES[1:]:
            predictions, performance_metrics, summary = evaluate_boundary_profile_runs(
                cases, runner, profile, boundary, cases_path, runs
            )
            results[boundary] = {
                "quality": quality_metrics(cases, predictions),
                "performance": performance_metrics,
                "component_resource_loaded": summary["component_artifact_sha256"]
                is not None,
            }
        profiles[profile] = results
    return {
        "boundaries": list(BOUNDARY_POLICIES),
        "profiles": profiles,
    }


def evaluate_untagged_profile_runs(
    cases: list[dict[str, object]],
    cases_path: Path,
    runner: Path,
    profile: str,
    boundary: str,
    runs: int,
) -> tuple[dict[str, object], dict[str, dict[str, object]]]:
    run_native_untagged_profile(runner, profile, boundary, cases_path)
    evaluations = []
    diagnostics = []
    summaries = []
    for _ in range(runs):
        summary = run_native_untagged_profile(
            runner, profile, boundary, cases_path
        )
        summaries.append(summary)
        evaluations.append(evaluate_kfind(cases, summary, f"{profile}/{boundary}"))
        diagnostics.append(
            {
                result["id"]: result["plan_diagnostic"]
                for result in summary["results"]
            }
        )
    first = evaluations[0]
    for evaluation, plan_diagnostics in zip(evaluations[1:], diagnostics[1:]):
        if evaluation[0] != first[0] or evaluation[1] != first[1]:
            raise ValueError(
                f"untagged {profile}/{boundary} predictions changed between runs"
            )
        if plan_diagnostics != diagnostics[0]:
            raise ValueError(
                f"untagged {profile}/{boundary} plans changed between runs"
            )
    expected_component = boundary == "smart"
    if any(
        summary["profile"] != profile
        or summary["boundary"] != boundary
        or (summary["component_artifact_sha256"] is not None)
        != expected_component
        for summary in summaries
    ):
        raise ValueError(
            f"untagged {profile}/{boundary} resource state differs from its profile"
        )
    failures = [
        {
            "case": case,
            "predicted": first[0][case["id"]],
            "matching_spans": first[1][case["id"]],
            "plan_diagnostic": diagnostics[0][case["id"]],
        }
        for case in cases
        if first[0][case["id"]] != bool(case["expected"])
    ]
    return (
        {
            "quality": quality_metrics(cases, first[0]),
            "performance": aggregate_performance(
                [evaluation[2] for evaluation in evaluations], warmup_runs=1
            ),
            "component_resource_loaded": expected_component,
            "lexicon_artifact_sha256": summaries[0][
                "lexicon_artifact_sha256"
            ],
            "component_artifact_sha256": summaries[0][
                "component_artifact_sha256"
            ],
            "failures": failures,
        },
        diagnostics[0],
    )


def evaluate_human_untagged(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    cases_path: Path,
    runner: Path,
    runs: int,
) -> dict[str, object]:
    profiles = {}
    for backend in KFIND_PROFILES:
        profile = backend.removeprefix("kfind-")
        boundaries = {}
        plan_diagnostics = None
        for boundary in HUMAN_BOUNDARY_POLICIES:
            result, current_diagnostics = evaluate_untagged_profile_runs(
                cases, cases_path, runner, profile, boundary, runs
            )
            if plan_diagnostics is not None and current_diagnostics != plan_diagnostics:
                raise ValueError(
                    f"untagged {profile} plan differs between boundary policies"
                )
            plan_diagnostics = current_diagnostics
            boundaries[boundary] = result
        profiles[profile] = {
            "plan": untagged_plan_metrics(cases, plan_diagnostics or {}),
            "boundaries": boundaries,
        }
    return {
        "task": "sentence lemma presence from an untagged query",
        "dataset": metadata,
        "boundaries": list(HUMAN_BOUNDARY_POLICIES),
        "profiles": profiles,
    }


def evaluate_product_persona_comparison(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    cases_path: Path,
    runner: Path,
    runs: int,
    boundary_comparison: dict[str, object],
) -> dict[str, object]:
    user_result, diagnostics = evaluate_untagged_profile_runs(
        cases, cases_path, runner, "full-pos", "smart", runs
    )
    return product_persona_comparison(
        boundary_comparison,
        user_result,
        untagged_plan_metrics(cases, diagnostics),
        metadata,
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
    versions = {
        profile: {
            "backend": kfind[profile][5]["backend"],
            "version": kfind[profile][5]["version"],
            "profile": kfind[profile][5]["profile"],
            "lexicon_artifact_sha256": kfind[profile][5][
                "lexicon_artifact_sha256"
            ],
            "enriched_artifact_sha256": kfind[profile][5][
                "enriched_artifact_sha256"
            ],
            "morphology_artifact_sha256": kfind[profile][5][
                "morphology_artifact_sha256"
            ],
            "component_artifact_sha256": kfind[profile][5][
                "component_artifact_sha256"
            ],
            "graph_artifact_sha256": kfind[profile][5]["graph_artifact_sha256"],
        }
        for profile in KFIND_PROFILES
    }
    return {
        "versions": versions,
        "predictions": {profile: kfind[profile][0] for profile in KFIND_PROFILES},
        "matches": {profile: kfind[profile][1] for profile in KFIND_PROFILES},
        "performance": {profile: kfind[profile][2] for profile in KFIND_PROFILES},
        "diagnostics": {profile: kfind[profile][3] for profile in KFIND_PROFILES},
        "shadow_verification": {
            profile: kfind[profile][4] for profile in KFIND_PROFILES
        },
    }


def evaluate_constraint_suite(
    test_cases: list[dict[str, object]],
    test_path: Path,
    development_cases: list[dict[str, object]],
    development_path: Path,
    hard_cases: list[dict[str, object]],
    hard_path: Path,
    runner: Path,
    runs: int,
) -> dict[str, object]:
    return {
        "profile": "full-pos",
        "policies": list(CONSTRAINT_POLICIES),
        "test": evaluate_constraint_runs(
            test_cases, test_path, runner, "full-pos", runs, True
        ),
        "development": evaluate_constraint_runs(
            development_cases,
            development_path,
            runner,
            "full-pos",
            1,
            False,
        ),
        "hard_negatives": evaluate_constraint_runs(
            hard_cases, hard_path, runner, "full-pos", 1, False
        ),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--dev-cases", type=Path, default=DEFAULT_DEV_CASES)
    parser.add_argument("--dev-metadata", type=Path, default=DEFAULT_DEV_METADATA)
    parser.add_argument(
        "--human-untagged-cases", type=Path, default=DEFAULT_HUMAN_UNTAGGED_CASES
    )
    parser.add_argument(
        "--human-untagged-metadata",
        type=Path,
        default=DEFAULT_HUMAN_UNTAGGED_METADATA,
    )
    parser.add_argument("--hard-negatives", type=Path, default=DEFAULT_HARD_NEGATIVES)
    parser.add_argument(
        "--external-baselines", type=Path, default=DEFAULT_EXTERNAL_BASELINES
    )
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
        human_untagged_cases = load_cases(args.human_untagged_cases)
        human_untagged_metadata = json.loads(
            args.human_untagged_metadata.read_text(encoding="utf-8")
        )
        validate_untagged_dataset(
            args.human_untagged_cases,
            human_untagged_cases,
            human_untagged_metadata,
        )
        hard_cases = load_cases(args.hard_negatives)
        hard_metadata = validate_hard_negatives(args.hard_negatives, hard_cases)
        if args.smoke:
            with tempfile.TemporaryDirectory() as directory:
                smoke_path = Path(directory) / "smoke-cases.jsonl"
                hard_smoke_path = Path(directory) / "hard-smoke-cases.jsonl"
                human_untagged_smoke_path = (
                    Path(directory) / "human-untagged-smoke-cases.jsonl"
                )
                smoke_cases = select_smoke_cases(dev_cases)
                hard_smoke_cases = select_smoke_cases(hard_cases)
                human_untagged_smoke_cases = select_smoke_cases(
                    human_untagged_cases
                )
                write_cases(smoke_path, smoke_cases)
                write_cases(hard_smoke_path, hard_smoke_cases)
                write_cases(
                    human_untagged_smoke_path, human_untagged_smoke_cases
                )
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
                report["component_startup"] = evaluate_component_startup(
                    args.runner, args.runs
                )
                report["boundary_comparison"] = evaluate_boundary_comparison(
                    smoke_cases, smoke_path, args.runner, 1, baseline
                )
                report["human_untagged"] = evaluate_human_untagged(
                    human_untagged_smoke_cases,
                    smoke_metadata(
                        human_untagged_smoke_path,
                        human_untagged_smoke_cases,
                        human_untagged_metadata,
                        "test-human-untagged-smoke",
                    ),
                    human_untagged_smoke_path,
                    args.runner,
                    1,
                )
                report["product_workflows"] = product_workflows(
                    report["boundary_comparison"], report["human_untagged"]
                )
                report["product_persona_comparison"] = (
                    evaluate_product_persona_comparison(
                        smoke_cases,
                        smoke_metadata(smoke_path, smoke_cases, dev_metadata),
                        smoke_path,
                        args.runner,
                        1,
                        report["boundary_comparison"],
                    )
                )
                report["product_use_cases"] = measure_product_workflows(
                    runs=1, smoke=True
                )
                report["agent_precision_shadow"] = {
                    "development": build_agent_shadow_report(
                        smoke_cases,
                        run_native_agent_shadow(args.runner, smoke_path),
                    )
                }
                report["constraint_evaluation"] = evaluate_constraint_suite(
                    smoke_cases,
                    smoke_path,
                    smoke_cases,
                    smoke_path,
                    hard_smoke_cases,
                    hard_smoke_path,
                    args.runner,
                    1,
                )
                return write_report(args.output, report)

        baseline = evaluate_dataset(cases, args.cases, args.runner, args.runs, True)
        external = load_external_baselines(args.external_baselines, cases, metadata)
        for key in ("versions", "predictions", "matches"):
            baseline[key].update(external[key])
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
        report["external_baselines"] = {
            "availability": external["availability"],
            "environment": external["environment"],
            "performance": external["performance"],
        }
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
        report["component_startup"] = evaluate_component_startup(
            args.runner, args.runs
        )
        report["boundary_comparison"] = evaluate_boundary_comparison(
            cases, args.cases, args.runner, args.runs, baseline
        )
        report["human_untagged"] = evaluate_human_untagged(
            human_untagged_cases,
            human_untagged_metadata,
            args.human_untagged_cases,
            args.runner,
            args.runs,
        )
        report["product_workflows"] = product_workflows(
            report["boundary_comparison"], report["human_untagged"]
        )
        report["product_persona_comparison"] = evaluate_product_persona_comparison(
            cases,
            metadata,
            args.cases,
            args.runner,
            args.runs,
            report["boundary_comparison"],
        )
        report["product_use_cases"] = measure_product_workflows(
            runs=args.runs, smoke=False
        )
        report["agent_precision_shadow"] = {
            "development": build_agent_shadow_report(
                dev_cases,
                run_native_agent_shadow(args.runner, args.dev_cases),
            ),
            "hard_negatives": build_agent_shadow_report(
                hard_cases,
                run_native_agent_shadow(args.runner, args.hard_negatives),
            ),
            "test": build_agent_shadow_report(
                cases,
                run_native_agent_shadow(args.runner, args.cases),
            ),
        }
        report["constraint_evaluation"] = evaluate_constraint_suite(
            cases,
            args.cases,
            dev_cases,
            args.dev_cases,
            hard_cases,
            args.hard_negatives,
            args.runner,
            args.runs,
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
