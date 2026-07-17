from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

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


def evaluate_robustness_candidate_smoke(
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
    explicit_path = directory / "robustness-candidate-smoke-cases.jsonl"
    untagged_path = directory / "robustness-candidate-untagged-smoke-cases.jsonl"
    explicit_smoke = select_smoke_cases(explicit_cases)
    untagged_smoke = select_smoke_cases(untagged_cases)
    write_cases(explicit_path, explicit_smoke)
    write_cases(untagged_path, untagged_smoke)
    return evaluate_robustness_candidate_performance(
        explicit_cases=explicit_smoke,
        explicit_metadata=smoke_metadata(
            explicit_path,
            explicit_smoke,
            explicit_metadata,
            "test-robustness-candidate-smoke",
        ),
        explicit_path=explicit_path,
        untagged_cases=untagged_smoke,
        untagged_metadata=smoke_metadata(
            untagged_path,
            untagged_smoke,
            untagged_metadata,
            "test-robustness-candidate-untagged-smoke",
        ),
        untagged_path=untagged_path,
        runner=runner,
        runs=1,
        evaluate_dataset=evaluate_dataset,
        evaluate_boundary_profile=evaluate_boundary_profile,
        evaluate_untagged_profile=evaluate_untagged_profile,
    )


def evaluate_robustness_candidate_performance(
    *,
    explicit_cases: list[dict[str, object]],
    explicit_metadata: dict[str, object],
    explicit_path: Path,
    untagged_cases: list[dict[str, object]],
    untagged_metadata: dict[str, object],
    untagged_path: Path,
    runner: Path,
    runs: int,
    evaluate_dataset: EvaluateDataset,
    evaluate_boundary_profile: EvaluateBoundaryProfile,
    evaluate_untagged_profile: EvaluateUntaggedProfile,
) -> dict[str, object]:
    explicit = evaluate_dataset(
        explicit_cases, explicit_path, runner, runs, True
    )
    _, agent_performance, _ = evaluate_boundary_profile(
        explicit_cases,
        runner,
        "embedded",
        "any",
        explicit_path,
        runs,
    )
    human, _ = evaluate_untagged_profile(
        untagged_cases,
        untagged_path,
        runner,
        "full-pos",
        "smart",
        runs,
    )
    return {
        "task": "annotation-required noisy-text performance",
        "robustness_mode": "off",
        "scoring_status": "annotation-required",
        "quality_reported": False,
        "datasets": {
            "explicit_pos": explicit_metadata,
            "untagged": untagged_metadata,
        },
        "workloads": {
            "embedded-smart-explicit-pos": {
                "input": "explicit-pos",
                "profile": "embedded",
                "boundary": "smart",
                "performance": explicit["performance"]["kfind-embedded"],
            },
            "full-pos-smart-explicit-pos": {
                "input": "explicit-pos",
                "profile": "full-pos",
                "boundary": "smart",
                "performance": explicit["performance"]["kfind-full-pos"],
            },
            "agent-embedded-any-explicit-pos": {
                "input": "explicit-pos",
                "profile": "embedded",
                "boundary": "any",
                "performance": agent_performance,
            },
            "human-full-pos-smart-untagged": {
                "input": "untagged",
                "profile": "full-pos",
                "boundary": "smart",
                "performance": human["performance"],
            },
        },
    }
