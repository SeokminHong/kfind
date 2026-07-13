from __future__ import annotations

import unittest

from unseen_benchmark import project_case


def span(start: int, end: int) -> dict[str, int]:
    return {"byte_start": start, "byte_end": end}


def origin(analysis_index: int, *rules: str) -> dict[str, object]:
    return {"analysis_index": analysis_index, "rule_path": list(rules)}


def candidate(
    start: int, end: int, *origins: dict[str, object]
) -> dict[str, object]:
    return {
        "atom_index": 0,
        "core": span(start, end),
        "token": span(start, end),
        "origins": list(origins),
    }


def evidence(
    start: int,
    end: int,
    analysis_index: int,
    decision: str | None,
    *rules: str,
) -> dict[str, object]:
    return {
        "status": "evaluated" if decision is not None else "limit-exceeded",
        "atom_index": 0,
        "analysis_index": analysis_index,
        "rule_path": list(rules),
        "target": span(start, end),
        "decision": decision,
        "include_cost": 10 if decision is not None else None,
        "exclude_cost": 10 if decision is not None else None,
        "cost_margin": 0 if decision is not None else None,
    }


class CopulaProjectionTests(unittest.TestCase):
    def test_removes_only_reject_and_preserves_other_outcomes(self) -> None:
        case = {"id": "negative", "expected": False}
        candidates = [
            candidate(0, 1, origin(0, "accept")),
            candidate(2, 3, origin(0, "reject")),
            candidate(4, 5, origin(0, "ambiguous")),
            candidate(6, 7, origin(0, "unresolved")),
        ]
        shadow = {
            "lattice": [
                evidence(0, 1, 0, "accept", "accept"),
                evidence(2, 3, 0, "reject", "reject"),
                evidence(4, 5, 0, "ambiguous", "ambiguous"),
                evidence(6, 7, 0, None, "unresolved"),
            ]
        }

        projected, outcomes = project_case(
            case,
            True,
            [span(0, 1), span(2, 3), span(4, 5), span(6, 7)],
            candidates,
            shadow,
        )

        self.assertEqual(
            [span(0, 1), span(4, 5), span(6, 7)],
            projected["projected_spans"],
        )
        self.assertEqual(
            {"accept": 1, "reject": 1, "ambiguous": 1, "unresolved": 1},
            dict(outcomes),
        )

    def test_non_contextual_origin_keeps_rejected_span(self) -> None:
        case = {"id": "negative", "expected": False}
        candidates = [candidate(0, 1, origin(0, "vcp"), origin(1, "literal"))]
        shadow = {"lattice": [evidence(0, 1, 0, "reject", "vcp")]}

        projected, _ = project_case(
            case, True, [span(0, 1)], candidates, shadow
        )

        self.assertEqual([span(0, 1)], projected["projected_spans"])
        self.assertEqual(
            ["reject", "not-contextual"],
            [item["outcome"] for item in projected["origins"]],
        )

    def test_reselects_later_candidate_after_reject(self) -> None:
        case = {"id": "negative", "expected": False}
        candidates = [
            candidate(0, 2, origin(0, "first")),
            candidate(1, 3, origin(0, "later")),
        ]
        shadow = {
            "lattice": [
                evidence(0, 2, 0, "reject", "first"),
                evidence(1, 3, 0, "accept", "later"),
            ]
        }

        projected, _ = project_case(
            case, True, [span(0, 2)], candidates, shadow
        )

        self.assertEqual([span(1, 3)], projected["projected_spans"])

    def test_requires_every_lattice_origin_to_map_to_a_candidate(self) -> None:
        case = {"id": "negative", "expected": False}

        with self.assertRaisesRegex(ValueError, "does not map"):
            project_case(
                case,
                False,
                [],
                [],
                {"lattice": [evidence(0, 1, 0, "reject", "missing")]},
            )


if __name__ == "__main__":
    unittest.main()
