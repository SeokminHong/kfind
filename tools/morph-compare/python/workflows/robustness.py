from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from python.external_baselines import load_external_baselines
from python.quality import (
    contract_quality_metrics,
    grouped_contract_quality,
    grouped_quality,
    quality_metrics,
)
from python.validation import select_smoke_cases, smoke_metadata, write_cases


Evaluation = dict[str, object]
EvaluateDataset = Callable[
    [list[dict[str, object]], Path, Path, int, bool], Evaluation
]
EvaluateBoundaryProfile = Callable[
    [list[dict[str, object]], Path, str, str, Path, int],
    tuple[dict[str, bool], dict[str, object], dict[str, object]],
]
EvaluateUntaggedProfile = Callable[
    [list[dict[str, object]], Path, Path, str, str, int],
    tuple[dict[str, object], dict[str, dict[str, object]]],
]


def robustness_quality(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    matches: dict[str, list[dict[str, object]]] | None = None,
) -> dict[str, object]:
    result = {
        "overall": quality_metrics(cases, predictions),
        "by_noise_class": grouped_quality(cases, predictions, "noise_class"),
        "by_noise_scope": grouped_quality(cases, predictions, "noise_scope"),
        "by_pos": grouped_quality(cases, predictions, "pos"),
        "contract_adjusted": {
            "overall": contract_quality_metrics(cases, predictions),
            "by_noise_class": grouped_contract_quality(
                cases, predictions, "noise_class"
            ),
            "by_noise_scope": grouped_contract_quality(
                cases, predictions, "noise_scope"
            ),
            "by_pos": grouped_contract_quality(cases, predictions, "pos"),
        },
    }
    if matches is not None:
        true_positives = [
            case
            for case in cases
            if case["expected"] and predictions[str(case["id"])]
        ]
        exact = sum(
            any(
                span["byte_start"] == case["gold_byte_start"]
                and span["byte_end"] == case["gold_byte_end"]
                for span in matches[str(case["id"])]
            )
            for case in true_positives
        )
        result["raw_span"] = {
            "overlap_true_positives": len(true_positives),
            "exact_true_positives": exact,
            "exact_rate_percent": round(100 * exact / len(true_positives), 2)
            if true_positives
            else 0.0,
        }
    return result


def evaluate_robustness_smoke(
    *,
    directory: Path,
    explicit_cases: list[dict[str, object]],
    explicit_metadata: dict[str, object],
    untagged_cases: list[dict[str, object]],
    untagged_metadata: dict[str, object],
    runner: Path,
    evaluate_dataset: EvaluateDataset,
    evaluate_boundary_profile: EvaluateBoundaryProfile,
    evaluate_untagged_profile: EvaluateUntaggedProfile,
) -> dict[str, object]:
    explicit_path = directory / "robustness-smoke-cases.jsonl"
    untagged_path = directory / "robustness-untagged-smoke-cases.jsonl"
    explicit_smoke = select_smoke_cases(
        explicit_cases, ("pos", "expected", "noise_scope")
    )
    untagged_smoke = select_smoke_cases(
        untagged_cases, ("pos", "expected", "noise_scope")
    )
    write_cases(explicit_path, explicit_smoke)
    write_cases(untagged_path, untagged_smoke)
    return evaluate_robustness(
        explicit_cases=explicit_smoke,
        explicit_metadata=smoke_metadata(
            explicit_path,
            explicit_smoke,
            explicit_metadata,
            "test-robustness-smoke",
        ),
        explicit_path=explicit_path,
        untagged_cases=untagged_smoke,
        untagged_metadata=smoke_metadata(
            untagged_path,
            untagged_smoke,
            untagged_metadata,
            "test-robustness-untagged-smoke",
        ),
        untagged_path=untagged_path,
        external_baselines_path=None,
        runner=runner,
        runs=1,
        evaluate_dataset=evaluate_dataset,
        evaluate_boundary_profile=evaluate_boundary_profile,
        evaluate_untagged_profile=evaluate_untagged_profile,
    )


def evaluate_robustness(
    *,
    explicit_cases: list[dict[str, object]],
    explicit_metadata: dict[str, object],
    explicit_path: Path,
    untagged_cases: list[dict[str, object]],
    untagged_metadata: dict[str, object],
    untagged_path: Path,
    external_baselines_path: Path | None,
    runner: Path,
    runs: int,
    evaluate_dataset: EvaluateDataset,
    evaluate_boundary_profile: EvaluateBoundaryProfile,
    evaluate_untagged_profile: EvaluateUntaggedProfile,
) -> dict[str, object]:
    explicit = evaluate_dataset(explicit_cases, explicit_path, runner, runs, True)
    external = None
    if external_baselines_path is not None:
        external = load_external_baselines(
            external_baselines_path, explicit_cases, explicit_metadata
        )
        for key in ("versions", "predictions", "matches"):
            explicit[key].update(external[key])

    explicit_quality = {
        backend: robustness_quality(
            explicit_cases,
            predictions,
            explicit["matches"][backend],
        )
        for backend, predictions in explicit["predictions"].items()
    }
    agent_predictions, agent_performance, agent_summary = (
        evaluate_boundary_profile(
            explicit_cases,
            runner,
            "embedded",
            "any",
            explicit_path,
            runs,
        )
    )
    human, _ = evaluate_untagged_profile(
        untagged_cases,
        untagged_path,
        runner,
        "full-pos",
        "smart",
        runs,
    )
    human_predictions = human.get("predictions")
    if not isinstance(human_predictions, dict):
        raise ValueError("robustness untagged evaluation omitted predictions")

    performance = dict(explicit["performance"])
    if external is not None:
        performance.update(external["performance"])
    external_report = None
    if external is not None:
        external_report = {
            "availability": external["availability"],
            "environment": external["environment"],
        }
    return {
        "task": "lemma presence in manually reviewed natural noisy sentences",
        "robustness_mode": "off",
        "scoring_status": "scored",
        "quality_reported": True,
        "datasets": {
            "explicit_pos": explicit_metadata,
            "untagged": untagged_metadata,
        },
        "explicit_pos": {
            "backends": list(explicit["predictions"]),
            "versions": explicit["versions"],
            "quality": explicit_quality,
            "performance": performance,
            "external_baselines": external_report,
        },
        "workflows": {
            "agent-embedded-any-explicit-pos": {
                "input": "explicit-pos",
                "profile": "embedded",
                "boundary": "any",
                "robustness_mode": "off",
                "quality": robustness_quality(
                    explicit_cases, agent_predictions
                ),
                "performance": agent_performance,
                "version": {
                    "backend": agent_summary.get("backend"),
                    "version": agent_summary.get("version"),
                },
            },
            "human-full-pos-smart-untagged": {
                "input": "untagged",
                "profile": "full-pos",
                "boundary": "smart",
                "robustness_mode": "off",
                "quality": robustness_quality(
                    untagged_cases, human_predictions
                ),
                "performance": human["performance"],
            },
        },
    }
