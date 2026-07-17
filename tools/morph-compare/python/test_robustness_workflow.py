from __future__ import annotations

import unittest
from pathlib import Path

from workflows.robustness import evaluate_robustness_candidate_performance


def measured(value: float) -> dict[str, object]:
    return {
        "runs": 5,
        "initialization_seconds": value,
        "evaluation_seconds": value,
        "cases_per_second": value,
        "latency_p50_ms": value,
        "latency_p95_ms": value,
        "peak_rss_kib": value,
        "run_min": {},
        "run_max": {},
    }


class RobustnessCandidateWorkflowTests(unittest.TestCase):
    def test_returns_only_performance_workloads(self) -> None:
        paths: list[tuple[object, ...]] = []

        def evaluate_dataset(*args):
            paths.append(("dataset", *args[1:]))
            return {
                "performance": {
                    "kfind-embedded": measured(1.0),
                    "kfind-full-pos": measured(2.0),
                },
                "quality": {"must": "not leak"},
            }

        def evaluate_boundary(*args):
            paths.append(("boundary", *args[1:]))
            return {}, measured(3.0), {}

        def evaluate_untagged(*args):
            paths.append(("untagged", *args[1:]))
            return {"performance": measured(4.0), "quality": {}}, {}

        result = evaluate_robustness_candidate_performance(
            explicit_cases=[{"id": "explicit"}],
            explicit_metadata={"fixture_sha256": "explicit"},
            explicit_path=Path("explicit.jsonl"),
            untagged_cases=[{"id": "untagged"}],
            untagged_metadata={"fixture_sha256": "untagged"},
            untagged_path=Path("untagged.jsonl"),
            runner=Path("runner"),
            runs=5,
            evaluate_dataset=evaluate_dataset,
            evaluate_boundary_profile=evaluate_boundary,
            evaluate_untagged_profile=evaluate_untagged,
        )

        self.assertFalse(result["quality_reported"])
        self.assertEqual("off", result["robustness_mode"])
        self.assertNotIn("quality", result)
        self.assertEqual(
            [1.0, 2.0, 3.0, 4.0],
            [
                workload["performance"]["initialization_seconds"]
                for workload in result["workloads"].values()
            ],
        )
        self.assertEqual("explicit.jsonl", str(paths[0][1]))
        self.assertIn("any", paths[1])
        self.assertIn("untagged.jsonl", tuple(map(str, paths[2])))


if __name__ == "__main__":
    unittest.main()
