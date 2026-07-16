from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from python.external_baselines import load_external_baselines
from python.query_matrix import (
    query_matrix_metrics,
    query_matrix_smoke_metadata,
    select_query_matrix_smoke_cases,
)
from python.report import build_report, product_workflows
from python.validation import write_cases


Evaluation = dict[str, object]
EvaluateDataset = Callable[
    [list[dict[str, object]], Path, Path, int, bool], Evaluation
]
EvaluateBoundary = Callable[
    [list[dict[str, object]], Path, Path, int, Evaluation], dict[str, object]
]
EvaluateHuman = Callable[
    [list[dict[str, object]], dict[str, object], Path, Path, int],
    dict[str, object],
]


def build_query_matrix_report(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    evaluation: Evaluation,
    *,
    include_performance: bool = True,
) -> dict[str, object]:
    report = build_report(
        cases,
        metadata,
        evaluation["versions"],
        evaluation["predictions"],
        evaluation["matches"],
        evaluation["performance"],
        evaluation["diagnostics"],
        evaluation["shadow_verification"],
        include_performance=include_performance,
    )
    report["sentence_coverage"] = {
        backend: query_matrix_metrics(
            cases,
            predictions,
            "query-matrix-bootstrap-v1",
        )
        for backend, predictions in evaluation["predictions"].items()
    }
    return report


def evaluate_query_matrix_smoke(
    *,
    directory: Path,
    explicit_cases: list[dict[str, object]],
    explicit_metadata: dict[str, object],
    untagged_cases: list[dict[str, object]],
    untagged_metadata: dict[str, object],
    runner: Path,
    evaluate_dataset: EvaluateDataset,
    evaluate_boundary: EvaluateBoundary,
    evaluate_human: EvaluateHuman,
) -> dict[str, object]:
    explicit_path = directory / "query-matrix-smoke-cases.jsonl"
    untagged_path = directory / "query-matrix-untagged-smoke-cases.jsonl"
    explicit_smoke = select_query_matrix_smoke_cases(explicit_cases)
    untagged_smoke = select_query_matrix_smoke_cases(untagged_cases)
    write_cases(explicit_path, explicit_smoke)
    write_cases(untagged_path, untagged_smoke)

    evaluation = evaluate_dataset(explicit_smoke, explicit_path, runner, 1, True)
    explicit_report = build_query_matrix_report(
        explicit_smoke,
        query_matrix_smoke_metadata(
            explicit_path, explicit_smoke, explicit_metadata
        ),
        evaluation,
    )
    explicit_report["boundary_comparison"] = evaluate_boundary(
        explicit_smoke, explicit_path, runner, 1, evaluation
    )
    human = evaluate_human(
        untagged_smoke,
        query_matrix_smoke_metadata(
            untagged_path, untagged_smoke, untagged_metadata
        ),
        untagged_path,
        runner,
        1,
    )
    return {
        "explicit_pos": explicit_report,
        "human_untagged": human,
        "product_workflows": product_workflows(
            explicit_report["boundary_comparison"], human
        ),
        "development": None,
    }


def evaluate_query_matrix_full(
    *,
    explicit_cases: list[dict[str, object]],
    explicit_metadata: dict[str, object],
    explicit_path: Path,
    external_baselines_path: Path,
    development_cases: list[dict[str, object]],
    development_metadata: dict[str, object],
    development_path: Path,
    untagged_cases: list[dict[str, object]],
    untagged_metadata: dict[str, object],
    untagged_path: Path,
    runner: Path,
    runs: int,
    evaluate_dataset: EvaluateDataset,
    evaluate_boundary: EvaluateBoundary,
    evaluate_human: EvaluateHuman,
) -> dict[str, object]:
    evaluation = evaluate_dataset(
        explicit_cases, explicit_path, runner, runs, True
    )
    external = load_external_baselines(
        external_baselines_path, explicit_cases, explicit_metadata
    )
    for key in ("versions", "predictions", "matches"):
        evaluation[key].update(external[key])
    explicit_report = build_query_matrix_report(
        explicit_cases, explicit_metadata, evaluation
    )
    explicit_report["external_baselines"] = {
        "availability": external["availability"],
        "environment": external["environment"],
        "performance": external["performance"],
    }
    explicit_report["boundary_comparison"] = evaluate_boundary(
        explicit_cases, explicit_path, runner, runs, evaluation
    )
    development_evaluation = evaluate_dataset(
        development_cases, development_path, runner, 1, False
    )
    human = evaluate_human(
        untagged_cases, untagged_metadata, untagged_path, runner, runs
    )
    return {
        "explicit_pos": explicit_report,
        "human_untagged": human,
        "product_workflows": product_workflows(
            explicit_report["boundary_comparison"], human
        ),
        "development": build_query_matrix_report(
            development_cases,
            development_metadata,
            development_evaluation,
            include_performance=False,
        ),
    }
