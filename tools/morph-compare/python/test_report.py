import unittest

from report import classify_primary_cause, kfind_profile_comparison, quality_metrics


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
                "kfind-embedded": False,
                "kfind-full-pos": False,
                "kiwi": kiwi,
                "lindera": lindera,
            },
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


if __name__ == "__main__":
    unittest.main()
