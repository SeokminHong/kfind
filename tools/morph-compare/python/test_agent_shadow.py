import unittest

from python.agent_shadow import (
    build_agent_shadow_report,
    classify_exact_analysis,
    classify_path_presence,
)


class AgentShadowTest(unittest.TestCase):
    def test_classifies_path_presence_without_using_cost_order(self):
        self.assertEqual(
            classify_path_presence([lattice(10, None)]), "include-only"
        )
        self.assertEqual(
            classify_path_presence([lattice(50, 10)]), "include-and-exclude"
        )
        self.assertEqual(
            classify_path_presence([lattice(None, 10)]), "exclude-only"
        )

    def test_classifies_exact_predicate_analysis(self):
        self.assertEqual(classify_exact_analysis([]), "none")
        self.assertEqual(
            classify_exact_analysis([{"pos": "NNG+JX"}]),
            "non-predicate-only",
        )
        self.assertEqual(
            classify_exact_analysis([{"pos": "NNG+XSV+EF"}]),
            "predicate-or-mixed",
        )

    def test_builds_cost_independent_projection(self):
        cases = [
            case("positive", True, 0, 3),
            case("negative", False, None, None),
        ]
        raw = {
            "profile": "embedded",
            "boundary": "any",
            "morphology_artifact_sha256": "abc",
            "results": [
                result("positive", matched(0, 3, lattice(50, None))),
                result("negative", matched(0, 3, lattice(50, 10))),
            ],
        }

        report = build_agent_shadow_report(cases, raw)

        self.assertEqual(report["projections"]["all"]["false_positive"], 1)
        include_only = report["projections"]["include-only"]
        self.assertEqual(include_only["true_positive"], 1)
        self.assertEqual(include_only["false_positive"], 0)


def lattice(include, exclude):
    return {
        "status": "evaluated",
        "include_cost": include,
        "exclude_cost": exclude,
    }


def case(case_id, expected, start, end):
    return {
        "id": case_id,
        "query": "학교",
        "pos": "noun",
        "expected": expected,
        "gold_byte_start": start,
        "gold_byte_end": end,
    }


def result(case_id, item):
    return {"id": case_id, "matches": [item]}


def matched(start, end, evidence):
    token = {"byte_start": start, "byte_end": end}
    return {
        "token": token,
        "whole_token": token,
        "lattice": [evidence],
        "exact_whole_token_analyses": [],
    }


if __name__ == "__main__":
    unittest.main()
