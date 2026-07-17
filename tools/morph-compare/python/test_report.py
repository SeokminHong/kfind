import unittest

from report import (
    BACKENDS,
    KFIND_PROFILES,
    append_boundary_comparison,
    append_contract_quality,
    append_structural_shadow_table,
    append_component_startup,
    append_development_failure_diagnostics,
    append_external_baselines,
    append_human_untagged,
    append_product_workflows,
    append_product_use_cases,
    append_query_matrix,
    append_robustness,
    build_report,
    classify_lattice_paths,
    classify_primary_cause,
    kfind_profile_comparison,
    product_persona_comparison,
    product_workflows,
    quality_metrics,
    shadow_verification_summary,
    strict_subspan_position,
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


class DevelopmentFailureDiagnosticTests(unittest.TestCase):
    @staticmethod
    def failure(
        case_id: str,
        query: str,
        pos: str,
        cause: str,
        rule_path: list[str],
        *,
        text: str | None = None,
        gold_span: tuple[int, int] | None = None,
        token_span: tuple[int, int] | None = None,
    ) -> dict[str, object]:
        text = text or (
            query.removesuffix("다") + "었다" if pos == "verb" else query
        )
        gold_start, gold_end = gold_span or (0, len(text.encode("utf-8")))
        match: dict[str, object] = {
            "origins": [{"analysis_index": 0, "rule_path": rule_path}]
        }
        if token_span is not None:
            match["token"] = {
                "byte_start": token_span[0],
                "byte_end": token_span[1],
            }
        return {
            "case": {
                "id": case_id,
                "query": query,
                "pos": pos,
                "text": text,
                "expected": True,
                "gold_byte_start": gold_start,
                "gold_byte_end": gold_end,
            },
            "predictions": {"kfind-full-pos": False},
            "profile_causes": {"kfind-full-pos": cause},
            "profile_cause_evidence": {
                "kfind-full-pos": {
                    "any_boundary_gold_matches": [match]
                }
            },
        }

    def test_renders_full_pos_cause_pos_counts_and_predicate_rule_paths(self) -> None:
        development = {
            "failures": [
                self.failure(
                    "verb-case", "먹다", "verb", "boundary-rejected", ["ending.past"]
                ),
                self.failure(
                    "adjective-case",
                    "예쁘다",
                    "adjective",
                    "boundary-rejected",
                    ["ending.adnominal"],
                ),
                self.failure("noun-case", "학교", "noun", "surface-missing", []),
            ]
        }
        lines: list[str] = []

        append_development_failure_diagnostics(lines, development)

        rendered = "\n".join(lines)
        self.assertIn("| boundary-rejected | adjective | 1 |", rendered)
        self.assertIn("| boundary-rejected | verb | 1 |", rendered)
        self.assertIn("| surface-missing | noun | 1 |", rendered)
        self.assertIn(
            "| verb-case | 먹다/verb | 먹었다 | ending.past |", rendered
        )
        self.assertIn(
            "| adjective-case | 예쁘다/adjective | 예쁘다 | ending.adnominal |",
            rendered,
        )
        self.assertNotIn("| noun-case |", rendered)

    def test_renders_connective_ji_strict_subspan_positions(self) -> None:
        development = {
            "failures": [
                self.failure(
                    "left-case",
                    "없다",
                    "adjective",
                    "boundary-rejected",
                    ["ending.connective-ji"],
                    text="없지는",
                    token_span=(0, 6),
                ),
                self.failure(
                    "right-case",
                    "주다",
                    "verb",
                    "boundary-rejected",
                    ["ending.connective-ji"],
                    text="심어주지",
                    token_span=(6, 12),
                ),
            ]
        }
        lines: list[str] = []

        append_development_failure_diagnostics(lines, development)

        rendered = "\n".join(lines)
        self.assertIn("| left-edge | 1 |", rendered)
        self.assertIn("| right-edge | 1 |", rendered)
        self.assertIn("| internal | 0 |", rendered)
        self.assertIn(
            "| left-case | 없다/adjective | 없지는 | 없지 | left-edge |",
            rendered,
        )
        self.assertIn(
            "| right-case | 주다/verb | 심어주지 | 주지 | right-edge |",
            rendered,
        )

    def test_classifies_only_strict_gold_subspans(self) -> None:
        self.assertEqual("left-edge", strict_subspan_position(0, 9, 0, 6))
        self.assertEqual("right-edge", strict_subspan_position(0, 9, 3, 9))
        self.assertEqual("internal", strict_subspan_position(0, 12, 3, 9))
        with self.assertRaises(ValueError):
            strict_subspan_position(0, 9, 0, 9)
        with self.assertRaises(ValueError):
            strict_subspan_position(3, 9, 0, 6)


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

    def test_renders_contract_counts_separately(self) -> None:
        report = {
            "backends": ["kfind-embedded"],
            "quality": {
                "kfind-embedded": {
                    "contract_adjusted": {
                        "overall": {
                            "contract_precision_percent": 80.0,
                            "contract_recall_percent": 100.0,
                            "contract_f1_percent": 88.89,
                            "contract_tp": 4,
                            "contract_fp": 1,
                            "contract_tn": 2,
                            "contract_fn": 0,
                            "reclassified_cases": 3,
                        }
                    }
                }
            },
        }
        lines: list[str] = []

        append_contract_quality(lines, report)

        self.assertIn("TPᶜ", "\n".join(lines))
        self.assertIn(
            "| kfind-embedded | 80.0% | 100.0% | 88.89% | 4 | 1 | 2 | 0 | 3 |",
            "\n".join(lines),
        )


class ProductUseCaseTests(unittest.TestCase):
    def test_renders_fresh_process_cli_metrics(self) -> None:
        performance = {
            "runs": 3,
            "wall_seconds": 0.2,
            "throughput_mib_s": 500.0,
            "peak_rss_kib": 20480,
            "run_min": {
                "wall_seconds": 0.1,
                "throughput_mib_s": 450.0,
                "peak_rss_kib": 19000,
            },
            "run_max": {
                "wall_seconds": 0.3,
                "throughput_mib_s": 550.0,
                "peak_rss_kib": 21000,
            },
        }
        use_cases = {
            "profile": "standard",
            "cache": "warm cache",
            "corpus": {"bytes": 104857600, "files": 1000, "sha256": "abc"},
            "workflows": {
                name: {
                    "output": output,
                    "command": f"kfind {name}",
                    "matching_lines": 1,
                    "performance": performance,
                }
                for name, output in (("agent", "JSON Lines"), ("human", "default text"))
            },
        }
        lines: list[str] = []

        append_product_use_cases(lines, use_cases)

        rendered = "\n".join(lines)
        self.assertIn("## Product CLI use cases", rendered)
        self.assertIn("| agent | JSON Lines | 0.2000s [0.1000, 0.3000]", rendered)
        self.assertIn("20.0 MiB", rendered)


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


class ProductWorkflowTests(unittest.TestCase):
    def test_selects_agent_and_human_product_profiles(self) -> None:
        agent = {
            "quality": {
                "precision_percent": 60.0,
                "recall_percent": 95.0,
                "f1_percent": 73.55,
                "fp": 20,
            },
            "contract_adjusted_quality": {"contract_precision_percent": 80.0},
            "sentence_coverage": {"all_present_queries_recovered_percent": 70.0},
            "contract_adjusted_sentence_coverage": {
                "all_present_queries_recovered_percent": 65.0
            },
            "performance": {"cases_per_second": 5000.0},
        }
        human = {
            "quality": {
                "precision_percent": 90.0,
                "recall_percent": 85.0,
                "f1_percent": 87.43,
                "fp": 5,
            },
            "contract_adjusted_quality": {"contract_precision_percent": 95.0},
            "sentence_coverage": {"all_present_queries_recovered_percent": 80.0},
            "contract_adjusted_sentence_coverage": {
                "all_present_queries_recovered_percent": 75.0
            },
            "performance": {"cases_per_second": 1000.0},
        }
        plan = {"expected_pos_present_percent": 92.0}
        boundary_comparison = {
            "profiles": {
                "embedded": {"any": agent},
            }
        }
        human_untagged = {
            "profiles": {
                "full-pos": {
                    "plan": plan,
                    "boundaries": {"smart": human},
                }
            }
        }

        workflows = product_workflows(boundary_comparison, human_untagged)

        self.assertEqual("explicit POS", workflows["agent"]["input"])
        self.assertIs(agent["quality"], workflows["agent"]["quality"])
        self.assertIs(
            agent["contract_adjusted_quality"],
            workflows["agent"]["contract_adjusted_quality"],
        )
        self.assertIs(
            agent["contract_adjusted_sentence_coverage"],
            workflows["agent"]["contract_adjusted_sentence_coverage"],
        )
        self.assertEqual("untagged", workflows["human"]["input"])
        self.assertIs(human["quality"], workflows["human"]["quality"])
        self.assertEqual(
            "embedded engine without optional resources",
            workflows["library"]["default"],
        )

        lines: list[str] = []
        append_product_workflows(lines, {"product_workflows": workflows})
        rendered = "\n".join(lines)
        self.assertIn("| agent | explicit POS | embedded | any |", rendered)
        self.assertIn("| human | untagged | full-pos | smart |", rendered)
        self.assertIn("workflows are not combined into one score", rendered)

    def test_builds_persona_comparison_from_same_fixture(self) -> None:
        agent = {
            "quality": {"precision_percent": 97.0},
            "contract_adjusted_quality": {"contract_precision_percent": 98.0},
            "performance": {"cases_per_second": 14000.0},
        }
        user = {
            "quality": {"precision_percent": 99.0},
            "contract_adjusted_quality": {"contract_precision_percent": 99.5},
            "performance": {"cases_per_second": 7000.0},
        }
        comparison = product_persona_comparison(
            {"profiles": {"embedded": {"any": agent}}},
            user,
            {"expected_pos_present_percent": 96.0},
            {"fixture_sha256": "fixture"},
        )

        self.assertEqual("explicit POS", comparison["rows"]["agent"]["input"])
        self.assertEqual("POS omitted", comparison["rows"]["user"]["input"])
        self.assertIs(agent["quality"], comparison["rows"]["agent"]["quality"])
        self.assertIs(
            agent["contract_adjusted_quality"],
            comparison["rows"]["agent"]["contract_adjusted_quality"],
        )
        self.assertIs(user["quality"], comparison["rows"]["user"]["quality"])
        self.assertEqual("fixture", comparison["dataset"]["fixture_sha256"])

    def test_renders_external_quality_and_performance_snapshots(self) -> None:
        lines: list[str] = []
        performance = {
            "runs": 5,
            "initialization_seconds": 0.25,
            "cases_per_second": 2500.0,
            "latency_p95_ms": 0.5,
            "peak_rss_kib": 20480,
            "run_min": {
                "initialization_seconds": 0.2,
                "cases_per_second": 2400.0,
                "latency_p95_ms": 0.4,
                "peak_rss_kib": 19000,
            },
            "run_max": {
                "initialization_seconds": 0.3,
                "cases_per_second": 2600.0,
                "latency_p95_ms": 0.6,
                "peak_rss_kib": 21000,
            },
        }
        quality = {
            "precision_percent": 100.0,
            "recall_percent": 80.0,
            "f1_percent": 88.89,
        }

        append_external_baselines(
            lines,
            {
                "product_persona_comparison": {
                    "rows": {
                        "agent": {
                            "label": "Agent",
                            "quality": quality,
                            "performance": performance,
                        },
                        "user": {
                            "label": "User",
                            "quality": quality,
                            "performance": performance,
                        },
                    }
                },
                "quality": {"kiwi": {"overall": quality}},
                "external_baselines": {
                    "environment": {
                        "platform": "Linux-aarch64",
                        "logical_cpus": 10,
                        "python": "3.12.13",
                    },
                    "performance": {"kiwi": performance},
                    "availability": {
                        "kiwi": {"status": "available"},
                        "komoran": {
                            "status": "unavailable",
                            "reason": "not captured",
                        },
                    }
                }
            },
        )

        rendered = "\n".join(lines)
        self.assertIn("## Product persona and external comparison", rendered)
        self.assertIn("| Agent | 100.0% | 80.0% |", rendered)
        self.assertIn("| User | 100.0% | 80.0% |", rendered)
        self.assertIn("| kiwi | 100.0% | 80.0% |", rendered)
        self.assertIn("| kiwi | available | 5 | 0.2500s", rendered)
        self.assertIn(
            "| komoran | unavailable: not captured | n/a |", rendered
        )


class QueryMatrixReportTests(unittest.TestCase):
    def test_groups_strict_and_contract_quality_by_matrix_slot(self) -> None:
        cases = [
            {
                "id": "present",
                "source": "sample",
                "pos": "noun",
                "expected": True,
                "matrix_slot": "present-1",
            },
            {
                "id": "absent",
                "source": "sample",
                "pos": "noun",
                "expected": False,
                "contract_expected": True,
                "contract_reason": "same-pos-homograph",
                "matrix_slot": "absent-1",
            },
        ]
        predictions = {
            profile: {"present": True, "absent": True} for profile in KFIND_PROFILES
        }
        matches = {
            profile: {"present": [], "absent": []} for profile in KFIND_PROFILES
        }
        empty_counters = {
            "raw_anchor_hits": 0,
            "verified_program_hits": 0,
            "structural_candidate_hits": 0,
            "unique_structural_windows": 0,
        }
        report = build_report(
            cases,
            {"fixture_sha256": "fixture"},
            {profile: {} for profile in KFIND_PROFILES},
            predictions,
            matches,
            {},
            {
                profile: {case["id"]: None for case in cases}
                for profile in KFIND_PROFILES
            },
            {
                profile: {case["id"]: empty_counters for case in cases}
                for profile in KFIND_PROFILES
            },
        )

        quality = report["quality"]["kfind-embedded"]
        self.assertEqual(1, quality["by_matrix_slot"]["absent-1"]["fp"])
        self.assertEqual(
            1,
            quality["contract_adjusted"]["by_matrix_slot"]["absent-1"][
                "contract_tp"
            ],
        )

    def test_renders_query_and_sentence_level_metrics_separately(self) -> None:
        quality = {
            "precision_percent": 99.0,
            "recall_percent": 90.0,
            "f1_percent": 94.26,
            "tp": 90,
            "fp": 1,
            "tn": 99,
            "fn": 10,
        }
        coverage = {
            "all_present_queries_recovered_percent": 75.0,
            "recall_sentence_cluster_bootstrap_95_percent": [84.0, 94.0],
        }
        contract_quality = {
            "contract_precision_percent": 99.5,
            "contract_recall_percent": 91.0,
            "contract_f1_percent": 95.06,
            "contract_tp": 91,
            "contract_fp": 1,
            "contract_tn": 98,
            "contract_fn": 9,
            "reclassified_cases": 1,
        }
        contract_coverage = {
            "all_present_queries_recovered_percent": 72.5,
            "recall_sentence_cluster_bootstrap_95_percent": [85.0, 95.0],
        }
        performance = {
            "cases_per_second": 1000.0,
            "latency_p95_ms": 0.5,
            "peak_rss_kib": 10240,
        }
        explicit = {
            "dataset": {
                "fixture_sha256": "matrix-fixture",
                "cases": 200,
                "positive_cases": 100,
                "negative_cases": 100,
                "sentences": 40,
                "canonical_positive_coverage": 30,
                "canonical_positive_cases": 30,
            },
            "backends": ["kfind-embedded"],
            "quality": {
                "kfind-embedded": {
                    "overall": quality,
                    "contract_adjusted": {"overall": contract_quality},
                }
            },
            "sentence_coverage": {"kfind-embedded": coverage},
            "contract_adjusted_sentence_coverage": {
                "kfind-embedded": contract_coverage
            },
        }
        report = {
            "explicit_pos": explicit,
            "product_workflows": {
                name: {
                    "quality": quality,
                    "contract_adjusted_quality": contract_quality,
                    "sentence_coverage": coverage,
                    "contract_adjusted_sentence_coverage": contract_coverage,
                    "performance": performance,
                }
                for name in ("agent", "human")
            },
            "development": None,
        }
        lines: list[str] = []

        append_query_matrix(lines, report)

        rendered = "\n".join(lines)
        self.assertIn("## Query matrix", rendered)
        self.assertIn("100 same-sentence negative", rendered)
        self.assertIn("| kfind-embedded | 99.0% | 90.0% |", rendered)
        self.assertIn(
            "| kfind-embedded | 99.5% | 91.0% | 95.06% | 91 | 1 | 98 | 9 | 72.5% |",
            rendered,
        )
        self.assertIn("| agent | 99.0% | 90.0% |", rendered)
        self.assertIn("| agent | 99.5% | 91.0% | 95.06% |", rendered)


class RobustnessReportTests(unittest.TestCase):
    def test_renders_quality_and_performance(self) -> None:
        performance = {
            "runs": 5,
            "initialization_seconds": 0.1,
            "cases_per_second": 1200.0,
            "latency_p50_ms": 0.2,
            "latency_p95_ms": 0.5,
            "peak_rss_kib": 10240,
            "run_min": {
                "initialization_seconds": 0.09,
                "cases_per_second": 1100.0,
                "latency_p50_ms": 0.18,
                "latency_p95_ms": 0.45,
                "peak_rss_kib": 9216,
            },
            "run_max": {
                "initialization_seconds": 0.11,
                "cases_per_second": 1300.0,
                "latency_p50_ms": 0.22,
                "latency_p95_ms": 0.55,
                "peak_rss_kib": 11264,
            },
        }
        robustness = {
            "scoring_status": "scored",
            "robustness_mode": "off",
            "datasets": {
                "explicit_pos": {"fixture_sha256": "explicit", "cases": 500},
                "untagged": {"fixture_sha256": "untagged", "cases": 500},
            },
            "explicit_pos": {
                "backends": ["kfind-embedded"],
                "quality": {
                    "kfind-embedded": {
                        "overall": {
                            "precision_percent": 98.0,
                            "recall_percent": 90.0,
                            "f1_percent": 93.83,
                            "tp": 225,
                            "fp": 5,
                            "tn": 245,
                            "fn": 25,
                        },
                        "by_noise_scope": {
                            scope: {
                                "cases": cases,
                                "precision_percent": 98.0,
                                "recall_percent": 90.0,
                                "f1_percent": 93.83,
                            }
                            for scope, cases in (
                                ("target-span", 100),
                                ("context-only", 400),
                            )
                        },
                        "by_noise_class": {
                            "hangul-typo": {
                                "cases": 500,
                                "precision_percent": 98.0,
                                "recall_percent": 90.0,
                                "f1_percent": 93.83,
                            }
                        },
                        "raw_span": {
                            "exact_true_positives": 225,
                            "overlap_true_positives": 225,
                        },
                    }
                },
                "performance": {"kfind-embedded": performance},
            },
            "workflows": {
                "agent-embedded-any-explicit-pos": {
                    "input": "explicit-pos",
                    "quality": {
                        "overall": {
                            "precision_percent": 96.0,
                            "recall_percent": 94.0,
                            "f1_percent": 94.99,
                            "fp": 10,
                        },
                        "by_noise_scope": {
                            "target-span": {"recall_percent": 91.0},
                            "context-only": {"recall_percent": 96.0},
                        },
                    },
                    "performance": performance,
                },
                "human-full-pos-smart-untagged": {
                    "input": "untagged",
                    "quality": {
                        "overall": {
                            "precision_percent": 97.0,
                            "recall_percent": 92.0,
                            "f1_percent": 94.43,
                        }
                    },
                    "performance": performance,
                }
            },
        }
        lines: list[str] = []

        append_robustness(lines, robustness)

        rendered = "\n".join(lines)
        self.assertIn("## Robustness quality and performance", rendered)
        self.assertIn("manually reviewed natural noisy sentences", rendered)
        self.assertIn(
            "| kfind Agent | 96.0% | 94.0% | 94.99% | 10 | 91.0% | 96.0% |",
            rendered,
        )
        self.assertIn("| kfind-embedded | 98.0% | 90.0% | 93.83% |", rendered)
        self.assertIn("| target-span | kfind-embedded | 100 |", rendered)
        self.assertIn("| human-full-pos-smart-untagged | untagged |", rendered)
        self.assertIn("| kfind-embedded | 5 |", rendered)
        self.assertIn("1200.0 [1100.0, 1300.0]", rendered)


class ShadowVerificationTests(unittest.TestCase):
    def test_aggregates_counters_and_preserves_case_evidence(self) -> None:
        by_case = {
            "none": {
                "raw_anchor_hits": 0,
                "verified_program_hits": 0,
                "structural_candidate_hits": 0,
                "unique_structural_windows": 0,
            },
            "component": {
                "raw_anchor_hits": 2,
                "verified_program_hits": 2,
                "structural_candidate_hits": 1,
                "unique_structural_windows": 1,
                "diagnostic_lattice": [
                    {"status": "evaluated", "decision": "accept"},
                ],
            },
        }
        cases = [
            {"id": "none", "expected": False},
            {"id": "component", "expected": True},
        ]

        summary = shadow_verification_summary(by_case, cases)

        self.assertEqual(2, summary["totals"]["raw_anchor_hits"])
        self.assertEqual(1, summary["cases_with_structural_candidates"])
        self.assertEqual({"accept": 1}, summary["lattice_decisions"])
        self.assertEqual({"accept": 1}, summary["lattice_cases_by_decision"])
        self.assertEqual(
            {"accept": 1}, summary["lattice_outcomes_by_class"]["positive"]
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
                "diagnostic_lattice": [
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
                "diagnostic_lattice": [
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

        classification = classify_lattice_paths(by_case, metadata)

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

    def test_renders_component_case_decisions(self) -> None:
        shadow = {
            profile: {
                "cases_with_structural_candidates": 5,
                "lattice_cases_by_decision": {"accept": 3, "reject": 2},
            }
            for profile in KFIND_PROFILES
        }
        lines: list[str] = []

        append_structural_shadow_table(lines, shadow)

        self.assertIn("| kfind-embedded | 5 | 3 | 2 |", "\n".join(lines))


if __name__ == "__main__":
    unittest.main()
