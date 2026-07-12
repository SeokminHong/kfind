import unittest

from report import (
    BACKENDS,
    KFIND_PROFILES,
    append_local_context_summary,
    classify_primary_cause,
    kfind_profile_comparison,
    quality_metrics,
    shadow_verification_summary,
)


class KfindProfileComparisonTests(unittest.TestCase):
    def test_separates_recovered_still_failing_and_regressed_cases(self) -> None:
        cases = [
            {"id": "recovered", "expected": True},
            {"id": "still-failing", "expected": True},
            {"id": "regressed", "expected": True},
            {"id": "stable", "expected": True},
            {"id": "negative", "expected": False},
        ]
        predictions = {
            "kfind-embedded": {
                "recovered": False,
                "still-failing": False,
                "regressed": True,
                "stable": True,
                "negative": False,
            },
            "kfind-full-pos": {
                "recovered": True,
                "still-failing": False,
                "regressed": False,
                "stable": True,
                "negative": True,
            },
        }
        matches = {
            profile: {case["id"]: [] for case in cases} for profile in predictions
        }

        comparison = kfind_profile_comparison(cases, predictions, matches)

        self.assertEqual(
            ["recovered"],
            [item["case"]["id"] for item in comparison["recovered_with_full_pos"]],
        )
        self.assertEqual(
            ["still-failing"],
            [
                item["case"]["id"]
                for item in comparison["still_failing_with_full_pos"]
            ],
        )
        self.assertEqual(
            ["regressed"],
            [item["case"]["id"] for item in comparison["regressed_with_full_pos"]],
        )


class PrimaryCauseTests(unittest.TestCase):
    def classify(
        self,
        *,
        profile: str = "kfind-embedded",
        embedded: bool = False,
        full_pos: bool = False,
        kiwi: bool = True,
        lindera: bool = True,
        spans: list[dict[str, object]] | None = None,
        auto_analysis: bool = True,
        any_overlap: bool = False,
        anchor_overlap: bool = False,
    ) -> str | None:
        return classify_primary_cause(
            {"id": "case", "expected": True},
            {
                "kfind-embedded": embedded,
                "kfind-full-pos": full_pos,
                "kiwi": kiwi,
                "lindera": lindera,
            },
            profile,
            spans or [],
            {
                "auto_has_expected_pos_analysis": auto_analysis,
                "any_boundary_gold_overlap": any_overlap,
                "gold_anchor_overlap": anchor_overlap,
            },
        )

    def test_cause_priority_is_deterministic(self) -> None:
        self.assertEqual(
            "gold-or-adapter", self.classify(kiwi=False, lindera=False)
        )
        self.assertEqual("lexicon-missing", self.classify(auto_analysis=False))
        self.assertEqual("span-mismatch", self.classify(spans=[{"byte_start": 1}]))
        self.assertEqual("boundary-rejected", self.classify(any_overlap=True))
        self.assertEqual(
            "continuation-rejected", self.classify(anchor_overlap=True)
        )
        self.assertEqual("surface-missing", self.classify())

    def test_profile_prediction_is_classified_independently(self) -> None:
        self.assertIsNone(self.classify(profile="kfind-full-pos", full_pos=True))
        self.assertEqual(
            "surface-missing",
            self.classify(profile="kfind-full-pos", embedded=True),
        )


class QualityMetricsTests(unittest.TestCase):
    def test_hard_negative_precision_counts_true_negatives(self) -> None:
        cases = [
            {"id": "true-negative", "expected": False},
            {"id": "false-positive", "expected": False},
        ]

        metrics = quality_metrics(
            cases,
            {"true-negative": False, "false-positive": True},
        )

        self.assertEqual(50.0, metrics["hard_negative_precision_percent"])
        self.assertEqual(1, metrics["tn"])
        self.assertEqual(1, metrics["fp"])


class ShadowVerificationTests(unittest.TestCase):
    def test_aggregates_counters_and_preserves_case_evidence(self) -> None:
        by_case = {
            "none": {
                "raw_anchor_hits": 0,
                "verified_branch_hits": 0,
                "local_lattice_candidate_hits": 0,
                "unique_analysis_windows": 0,
            },
            "vcp": {
                "raw_anchor_hits": 2,
                "verified_branch_hits": 2,
                "local_lattice_candidate_hits": 2,
                "unique_analysis_windows": 1,
                "lattice": [
                    {"status": "evaluated", "decision": "accept"},
                    {"status": "limit-exceeded", "decision": None},
                ],
            },
        }
        cases = [
            {"id": "none", "expected": False},
            {"id": "vcp", "expected": True, "target_group": "sample/vcp"},
        ]

        summary = shadow_verification_summary(by_case, cases)

        self.assertEqual(2, summary["totals"]["raw_anchor_hits"])
        self.assertEqual(2, summary["totals"]["local_lattice_candidate_hits"])
        self.assertEqual(1, summary["cases_with_local_candidates"])
        self.assertEqual({"accept": 1}, summary["lattice_decisions"])
        self.assertEqual(
            {"accept": 1, "limit-exceeded": 1},
            summary["lattice_outcomes_by_class"]["positive"],
        )
        self.assertEqual(by_case, summary["by_case"])


class LocalContextSummaryTests(unittest.TestCase):
    def test_renders_confusion_matrix_and_shadow_counts(self) -> None:
        metrics = {
            "precision_percent": 75.0,
            "recall_percent": 60.0,
            "f1_percent": 66.67,
            "tp": 3,
            "fp": 1,
            "tn": 4,
            "fn": 2,
        }
        local_context = {
            "dataset": {
                "fixture_sha256": "fixture",
                "cases": 10,
                "positive_cases": 5,
                "negative_cases": 5,
            },
            "quality": {
                backend: {
                    "overall": metrics,
                    "by_target_group": {"sample/vcp": metrics},
                }
                for backend in BACKENDS
            },
            "shadow_verification": {
                profile: {
                    "totals": {
                        "local_lattice_candidate_hits": 2,
                        "unique_analysis_windows": 1,
                    },
                    "cases_with_local_candidates": 1,
                    "lattice_outcomes_by_class": {
                        "positive": {"accept": 1, "reject": 1},
                        "negative": {"reject": 2},
                    },
                    "lattice_outcomes_by_target_group": {
                        "sample/vcp": {"accept": 1, "reject": 2}
                    },
                }
                for profile in KFIND_PROFILES
            },
        }
        lines: list[str] = []

        append_local_context_summary(lines, local_context)

        rendered = "\n".join(lines)
        self.assertIn("## Copula local-context slice", rendered)
        self.assertIn("| sample/vcp | kfind-embedded | 75.0% | 60.0%", rendered)
        self.assertIn("| kfind-embedded | 2 | 1 | 1 |", rendered)
        self.assertIn("| kfind-embedded | positive | 1 | 1 | 0 | 0 |", rendered)
        self.assertIn("| kfind-embedded | sample/vcp | 1 | 2 | 0 | 0 |", rendered)


if __name__ == "__main__":
    unittest.main()
