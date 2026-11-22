from __future__ import annotations

import runpy
import unittest
from pathlib import Path

from python.validation import load_cases


BENCHMARK = Path(__file__).resolve().parents[1]
REGISTRY = BENCHMARK / "contract-contrast-evidence.jsonl"
SCORE_CONTRACT = runpy.run_path(str(BENCHMARK / "score-ud-audit.py"))["score_contract"]


class UDAuditScoringTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        ids = {"gsd:test-s129:pronoun:이", "pud:n01144038:noun:거"}
        cls.reviews = {row["id"]: row for row in load_cases(REGISTRY) if row["id"] in ids}

    def test_exact_and_additional_ranges_are_scored_independently(self) -> None:
        single = "gsd:test-s129:pronoun:이"
        multiple = "pud:n01144038:noun:거"
        for case_id, spans, expected_fpc, expected_fnc, missing, extra in (
            (single, [(0, 9), (10, 13)], 1, 0, [], [(10, 13)]),
            (multiple, [(131, 134), (0, 3)], 1, 1, [(171, 174)], [(0, 3)]),
            (single, [(0, 9)], 0, 0, [], []),
            (multiple, [(131, 134), (171, 174)], 0, 0, [], []),
        ):
            with self.subTest(case=case_id, spans=spans):
                case = self.positive_case(case_id, self.reviews[case_id]["source_text"])
                matches = [{"byte_start": start, "byte_end": end} for start, end in spans]
                result = SCORE_CONTRACT(
                    [case], {case_id: True}, {case_id: matches}, REGISTRY, include_residual=True
                )
                self.assertEqual(result["contract"]["contract_fp"], 0)
                self.assertEqual(result["contract"]["contract_fn"], expected_fnc)
                self.assertEqual(result["contract_errors"]["fpc"], expected_fpc)
                self.assertEqual(result["contract_errors"]["fnc"], expected_fnc)
                self.assertEqual(
                    result["contract_errors"]["zero_target_verified"], expected_fpc == expected_fnc == 0
                )
                if missing or extra:
                    self.assertEqual(result["residual"], [{
                        "id": case_id,
                        "expected": True,
                        "spans": matches,
                        "false_positive": bool(extra),
                        "false_negative": bool(missing),
                        "missing": missing,
                        "extra": extra,
                    }])
                else:
                    self.assertEqual(result["residual"], [])

    def test_unverified_positive_ranges_remain_visible(self) -> None:
        case = self.positive_case("unverified", "보이는 것")
        result = SCORE_CONTRACT(
            [case], {"unverified": True}, {"unverified": [{"byte_start": 10, "byte_end": 13}]},
            REGISTRY, include_residual=False,
        )
        self.assertEqual(result["reviewed_ranges"]["unverified_positive_rows"], 1)
        self.assertTrue(result["contract_errors"]["zero_errors_observed"])
        self.assertFalse(result["contract_errors"]["zero_target_verified"])
        self.assertNotIn("residual", result)

    @staticmethod
    def positive_case(case_id: str, text: str) -> dict:
        return {
            "id": case_id,
            "text": text,
            "expected": False,
            "contract_expected": True,
            "contract_reason": "aligned-source-component",
        }
