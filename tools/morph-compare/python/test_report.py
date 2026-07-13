import unittest

from report import (
    BACKENDS,
    KFIND_PROFILES,
    append_boundary_comparison,
    append_component_shadow_table,
    append_component_startup,
    append_human_untagged,
    append_local_context_summary,
    classify_component_paths,
    classify_primary_cause,
    kfind_profile_comparison,
    quality_metrics,
    shadow_verification_summary,
    untagged_plan_metrics,
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


class ComponentStartupTests(unittest.TestCase):
    def test_renders_resource_less_and_explicit_component_profiles(self) -> None:
        metric = {
            "runs": 3,
            "base_initialization_seconds": 0.01,
            "component_initialization_seconds": None,
            "initialization_seconds": 0.01,
            "base_peak_rss_kib": 10240,
            "peak_rss_kib": 10240,
            "run_min": {
                "base_initialization_seconds": 0.009,
                "component_initialization_seconds": None,
                "initialization_seconds": 0.009,
                "base_peak_rss_kib": 10240,
                "peak_rss_kib": 10240,
            },
            "run_max": {
                "base_initialization_seconds": 0.011,
                "component_initialization_seconds": None,
                "initialization_seconds": 0.011,
                "base_peak_rss_kib": 10240,
                "peak_rss_kib": 10240,
            },
        }
        lines: list[str] = []

        append_component_startup(lines, {"embedded": metric})

        rendered = "\n".join(lines)
        self.assertIn("## Optional component startup", rendered)
        self.assertIn("| embedded | 3 | 0.0100s [0.0090, 0.0110] | n/a |", rendered)


class BoundaryComparisonTests(unittest.TestCase):
    def test_renders_quality_and_performance_for_each_policy(self) -> None:
        performance = {
            "runs": 5,
            "initialization_seconds": 0.1,
            "cases_per_second": 1000.0,
            "latency_p95_ms": 0.5,
            "peak_rss_kib": 10240,
            "run_min": {"cases_per_second": 900.0, "latency_p95_ms": 0.4},
            "run_max": {"cases_per_second": 1100.0, "latency_p95_ms": 0.6},
        }
        quality = {
            "precision_percent": 99.0,
            "recall_percent": 80.0,
            "f1_percent": 88.49,
        }
        comparison = {
            "boundaries": ["smart", "token", "any"],
            "profiles": {
                profile: {
                    boundary: {"quality": quality, "performance": performance}
                    for boundary in ("smart", "token", "any")
                }
                for profile in ("embedded", "full-pos")
            },
        }
        lines: list[str] = []

        append_boundary_comparison(lines, comparison)

        rendered = "\n".join(lines)
        self.assertIn("## Boundary policy comparison", rendered)
        self.assertIn(
            "| full-pos | any | 99.0% | 80.0% | 88.49% | 0.1000s |",
            rendered,
        )


class HumanUntaggedTests(unittest.TestCase):
    def test_aggregates_positive_plan_usability(self) -> None:
        cases = [
            {"id": "positive-a", "expected": True},
            {"id": "positive-b", "expected": True},
            {"id": "negative", "expected": False},
        ]
        diagnostics = {
            "positive-a": {
                "expected_pos_present": True,
                "multi_coarse_pos": True,
                "literal_fallback": False,
            },
            "positive-b": {
                "expected_pos_present": False,
                "multi_coarse_pos": False,
                "literal_fallback": True,
            },
            "negative": {
                "expected_pos_present": False,
                "multi_coarse_pos": False,
                "literal_fallback": False,
            },
        }

        metrics = untagged_plan_metrics(cases, diagnostics)

        self.assertEqual(50.0, metrics["expected_pos_present_percent"])
        self.assertEqual(50.0, metrics["multi_coarse_pos_percent"])
        self.assertEqual(50.0, metrics["literal_fallback_percent"])

    def test_renders_quality_performance_and_plan_metrics(self) -> None:
        performance = {
            "initialization_seconds": 0.01,
            "cases_per_second": 1200.0,
            "latency_p95_ms": 0.4,
            "peak_rss_kib": 10240,
        }
        quality = {
            "precision_percent": 90.0,
            "recall_percent": 80.0,
            "f1_percent": 84.71,
            "tp": 8,
            "fp": 1,
            "tn": 9,
            "fn": 2,
        }
        human = {
            "dataset": {
                "fixture_sha256": "untagged-fixture",
                "cases": 20,
                "positive_cases": 10,
                "negative_cases": 10,
            },
            "boundaries": ["smart", "any"],
            "profiles": {
                "embedded": {
                    "plan": {
                        "positive_cases": 10,
                        "expected_pos_present": 8,
                        "expected_pos_present_percent": 80.0,
                        "multi_coarse_pos": 3,
                        "multi_coarse_pos_percent": 30.0,
                        "literal_fallback": 2,
                        "literal_fallback_percent": 20.0,
                    },
                    "boundaries": {
                        boundary: {
                            "quality": quality,
                            "performance": performance,
                        }
                        for boundary in ("smart", "any")
                    },
                }
            },
        }
        lines: list[str] = []

        append_human_untagged(lines, human)

        rendered = "\n".join(lines)
        self.assertIn("## Human untagged search", rendered)
        self.assertIn("| embedded | any | 90.0% | 80.0% | 84.71% |", rendered)
        self.assertIn("| embedded | 10 | 80.0% (8) | 30.0% (3) |", rendered)


class ShadowVerificationTests(unittest.TestCase):
    def test_aggregates_counters_and_preserves_case_evidence(self) -> None:
        by_case = {
            "none": {
                "raw_anchor_hits": 0,
                "verified_branch_hits": 0,
                "local_lattice_candidate_hits": 0,
                "unique_analysis_windows": 0,
                "nominal_component_candidate_hits": 0,
                "unique_component_windows": 0,
            },
            "vcp": {
                "raw_anchor_hits": 2,
                "verified_branch_hits": 2,
                "local_lattice_candidate_hits": 2,
                "unique_analysis_windows": 1,
                "nominal_component_candidate_hits": 1,
                "unique_component_windows": 1,
                "component_projection_comparisons": 1,
                "component_projection_mismatches": 0,
                "lattice": [
                    {"status": "evaluated", "decision": "accept"},
                    {"status": "limit-exceeded", "decision": None},
                ],
                "component": [
                    {"status": "evaluated", "decision": "accept"},
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
        self.assertEqual(1, summary["cases_with_component_candidates"])
        self.assertEqual({"accept": 1}, summary["lattice_decisions"])
        self.assertEqual(
            {"accept": 1, "limit-exceeded": 1},
            summary["lattice_outcomes_by_class"]["positive"],
        )
        self.assertEqual({"accept": 1}, summary["component_decisions"])
        self.assertEqual({"accept": 1}, summary["component_cases_by_decision"])
        self.assertEqual(
            {"accept": 1}, summary["component_outcomes_by_class"]["positive"]
        )
        self.assertEqual(
            {"comparisons": 1, "mismatches": 0},
            summary["component_projection_equivalence"],
        )
        self.assertEqual(by_case, summary["by_case"])

    def test_classifies_lowest_cost_component_paths(self) -> None:
        target = {"byte_start": 3, "byte_end": 9}
        window = {"raw": {"byte_start": 3, "byte_end": 12}}
        query_node = {
            "original": target,
            "pos": "NNG",
            "unknown": False,
        }
        suffix_node = {
            "original": {"byte_start": 9, "byte_end": 12},
            "pos": "XSV",
            "unknown": False,
        }
        unknown_node = {
            "original": {"byte_start": 3, "byte_end": 12},
            "pos": "UNKNOWN",
            "unknown": True,
        }
        by_case = {
            "accept": {
                "component": [
                    {
                        "decision": "accept",
                        "include_cost": 10,
                        "target": target,
                        "window": window,
                        "paths": [
                            {
                                "cost": 10,
                                "includes_query": True,
                                "nodes": [query_node, suffix_node],
                            }
                        ],
                    },
                    {
                        "decision": "accept",
                        "include_cost": 20,
                        "target": target,
                        "window": window,
                        "paths": [
                            {
                                "cost": 20,
                                "includes_query": True,
                                "nodes": [query_node],
                            }
                        ],
                    },
                ]
            },
            "reject": {
                "component": [
                    {
                        "decision": "reject",
                        "exclude_cost": 5,
                        "target": target,
                        "window": window,
                        "paths": [
                            {
                                "cost": 5,
                                "includes_query": False,
                                "nodes": [unknown_node],
                            }
                        ],
                    }
                ]
            },
        }
        metadata = {
            "accept": {"expected": True},
            "reject": {"expected": True},
        }

        classification = classify_component_paths(by_case, metadata)

        self.assertEqual(
            {"derivational-continuation": 1},
            classification["path_types_by_class"]["positive"]["accept"],
        )
        self.assertEqual(
            {"unknown": 1},
            classification["path_types_by_class"]["positive"]["reject"],
        )
        self.assertEqual(
            {"derivational-continuation": 1},
            classification["p1_rule_candidates_by_class"]["positive"],
        )
        self.assertEqual(
            "prefix",
            classification["by_case"]["accept"]["decisions"]["accept"][
                "target_position"
            ],
        )

    def test_diagnoses_only_gold_aligned_copula_candidates(self) -> None:
        gold_span = {"byte_start": 3, "byte_end": 9}
        include_path = {
            "cost": 20,
            "includes_query": True,
            "nodes": [
                {
                    "original": {"byte_start": 6, "byte_end": 9},
                    "pos": "VCP+ETM",
                    "unknown": False,
                }
            ],
        }
        exclude_path = {
            "cost": 10,
            "includes_query": False,
            "nodes": [
                {
                    "original": {"byte_start": 3, "byte_end": 9},
                    "pos": "NNG",
                    "unknown": False,
                }
            ],
        }
        rejected = {
            "status": "evaluated",
            "decision": "reject",
            "target": {"byte_start": 6, "byte_end": 9},
            "window": {"raw": gold_span, "normalized": "격인"},
            "include_cost": 20,
            "exclude_cost": 10,
            "cost_margin": 10,
            "paths": [exclude_path, include_path],
        }
        unrelated = rejected | {
            "target": {"byte_start": 12, "byte_end": 15},
            "window": {
                "raw": {"byte_start": 12, "byte_end": 18},
                "normalized": "제일",
            },
        }
        by_case = {
            "vcp": {
                "raw_anchor_hits": 2,
                "verified_branch_hits": 2,
                "local_lattice_candidate_hits": 2,
                "unique_analysis_windows": 2,
                "nominal_component_candidate_hits": 0,
                "unique_component_windows": 0,
                "lattice": [rejected, unrelated],
            }
        }
        cases = [
            {
                "id": "vcp",
                "source": "sample",
                "sent_id": "sentence",
                "text": "격인 제일",
                "expected": True,
                "slice": "gold-copula",
                "target_group": "sample/vcp",
                "target_raw_tag": "vcp",
                "gold_byte_start": 3,
                "gold_byte_end": 9,
            }
        ]

        diagnosis = shadow_verification_summary(by_case, cases)[
            "copula_gold_diagnosis"
        ]

        self.assertEqual({"reject": 1}, diagnosis["gold_candidate_outcomes"])
        self.assertEqual(
            {"whole-window-competitor": 1}, diagnosis["failures_by_cause"]
        )
        self.assertEqual(1, len(diagnosis["failures"]))
        self.assertEqual("vcp", diagnosis["failures"][0]["target_raw_tag"])

    def test_renders_component_case_decisions(self) -> None:
        shadow = {
            profile: {
                "cases_with_component_candidates": 5,
                "component_cases_by_decision": {"accept": 3, "reject": 2},
            }
            for profile in KFIND_PROFILES
        }
        lines: list[str] = []

        append_component_shadow_table(lines, shadow)

        self.assertIn("| kfind-embedded | 5 | 3 | 2 |", "\n".join(lines))


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
                    "copula_gold_diagnosis": {
                        "gold_candidate_outcomes": {"accept": 3, "reject": 1},
                        "failures_by_target_group": {
                            "sample/vcp": {"whole-window-competitor": 1}
                        },
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
        self.assertIn("| kfind-embedded | 3 | 1 | 0 | 0 |", rendered)
        self.assertIn(
            "| kfind-embedded | sample/vcp | whole-window-competitor | 1 |",
            rendered,
        )


if __name__ == "__main__":
    unittest.main()
