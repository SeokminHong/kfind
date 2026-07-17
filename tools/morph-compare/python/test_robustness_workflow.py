from __future__ import annotations

import unittest
from pathlib import Path

from workflows.robustness import evaluate_robustness


def measured(value: float) -> dict[str, object]:
    return {
        "runs": 5,
        "warmup_runs": 1,
        "initialization_seconds": value,
        "evaluation_seconds": value,
        "cases_per_second": value,
        "latency_p50_ms": value,
        "latency_p95_ms": value,
        "peak_rss_kib": value,
        "run_min": {},
        "run_max": {},
    }


class RobustnessWorkflowTests(unittest.TestCase):
    def test_reports_quality_and_performance_separately(self) -> None:
        explicit_cases = [
            {
                "id": "positive",
                "expected": True,
                "gold_byte_start": 0,
                "gold_byte_end": 3,
                "noise_class": "hangul-typo",
                "noise_scope": "target-span",
                "pos": "noun",
            },
            {
                "id": "negative",
                "expected": False,
                "gold_byte_start": None,
                "gold_byte_end": None,
                "noise_class": "hangul-typo",
                "noise_scope": "context-only",
                "pos": "noun",
            },
        ]
        untagged_cases = [
            {**case, "id": f"untagged:{case['id']}"}
            for case in explicit_cases
        ]

        def evaluate_dataset(*_args):
            predictions = {"positive": True, "negative": False}
            matches = {
                "positive": [{"byte_start": 0, "byte_end": 3}],
                "negative": [],
            }
            return {
                "versions": {
                    "kfind-embedded": {"version": "test"},
                    "kfind-full-pos": {"version": "test"},
                },
                "predictions": {
                    "kfind-embedded": predictions,
                    "kfind-full-pos": predictions,
                },
                "matches": {
                    "kfind-embedded": matches,
                    "kfind-full-pos": matches,
                },
                "performance": {
                    "kfind-embedded": measured(1.0),
                    "kfind-full-pos": measured(2.0),
                },
            }

        def evaluate_boundary(*_args):
            return (
                {"positive": True, "negative": False},
                measured(3.0),
                {"backend": "kfind", "version": "test"},
            )

        def evaluate_untagged(*_args):
            return (
                {
                    "performance": measured(4.0),
                    "predictions": {
                        "untagged:positive": True,
                        "untagged:negative": False,
                    },
                },
                {},
            )

        result = evaluate_robustness(
            explicit_cases=explicit_cases,
            explicit_metadata={"fixture_sha256": "explicit"},
            explicit_path=Path("explicit.jsonl"),
            untagged_cases=untagged_cases,
            untagged_metadata={"fixture_sha256": "untagged"},
            untagged_path=Path("untagged.jsonl"),
            external_baselines_path=None,
            runner=Path("runner"),
            runs=5,
            evaluate_dataset=evaluate_dataset,
            evaluate_boundary_profile=evaluate_boundary,
            evaluate_untagged_profile=evaluate_untagged,
        )

        self.assertTrue(result["quality_reported"])
        self.assertEqual("scored", result["scoring_status"])
        self.assertEqual(
            100.0,
            result["explicit_pos"]["quality"]["kfind-embedded"]["overall"][
                "f1_percent"
            ],
        )
        self.assertEqual(
            100.0,
            result["explicit_pos"]["quality"]["kfind-embedded"]["raw_span"][
                "exact_rate_percent"
            ],
        )
        self.assertEqual(
            100.0,
            result["workflows"]["human-full-pos-smart-untagged"]["quality"][
                "overall"
            ]["f1_percent"],
        )


if __name__ == "__main__":
    unittest.main()
