from __future__ import annotations

import hashlib
import json
import runpy
import unittest
from pathlib import Path

from python.validation import load_cases


BENCHMARK = Path(__file__).resolve().parents[1]
REGISTRY = BENCHMARK / "contract-contrast-evidence.jsonl"
AUDIT = runpy.run_path(str(BENCHMARK / "score-ud-audit.py"))
SCORE_CONTRACT = AUDIT["score_contract"]


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
                    [case], {case_id: True}, {case_id: matches}, self.reviews, include_residual=True
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
            {}, include_residual=False,
        )
        self.assertEqual(result["reviewed_ranges"]["unverified_positive_rows"], 1)
        self.assertTrue(result["contract_errors"]["zero_errors_observed"])
        self.assertFalse(result["contract_errors"]["zero_target_verified"])
        self.assertNotIn("residual", result)

    def test_excluded_span_does_not_hide_other_ranges(self) -> None:
        case_id = "spacing"
        case = {
            "id": case_id,
            "text": "첫째 둘째 셋째",
            "expected": False,
            "contract_expected": None,
            "contract_reason": "nonstandard-input",
        }
        excluded = {"byte_start": 0, "byte_end": 6}
        gold = {"byte_start": 7, "byte_end": 13}
        additional = {"byte_start": 14, "byte_end": 20}
        crossing = {"byte_start": 0, "byte_end": 13}
        review = {
            "source_text": case["text"],
            "source_excluded_spans": [excluded],
            "source_spans": [gold],
        }
        for matches, missing, extra in (
            ([excluded, gold], [], []),
            ([excluded, additional, crossing], [(7, 13)], [(0, 13), (14, 20)]),
        ):
            with self.subTest(matches=matches):
                result = SCORE_CONTRACT(
                    [case], {case_id: True}, {case_id: matches}, {case_id: review},
                    include_residual=True,
                )
                self.assertEqual(result["contract"]["excluded_cases"], 1)
                self.assertEqual(result["contract"]["contract_fp"], 0)
                self.assertEqual(result["contract"]["contract_fn"], 0)
                self.assertEqual(result["contract_errors"]["fpc"], int(bool(extra)))
                self.assertEqual(result["contract_errors"]["fnc"], int(bool(missing)))
                self.assertEqual(result["reviewed_ranges"]["verified_excluded_rows"], 1)
                if missing or extra:
                    self.assertEqual(result["residual"][0]["missing"], missing)
                    self.assertEqual(result["residual"][0]["extra"], extra)
                else:
                    self.assertTrue(result["contract_errors"]["zero_target_verified"])
                    self.assertEqual(result["residual"], [])

        pending = SCORE_CONTRACT(
            [case], {case_id: False}, {case_id: []}, {}, include_residual=True,
        )
        self.assertEqual(pending["reviewed_ranges"]["unverified_excluded_rows"], 1)
        self.assertTrue(pending["contract_errors"]["zero_errors_observed"])
        self.assertFalse(pending["contract_errors"]["zero_target_verified"])
        self.assertEqual(pending["residual"][0]["unverified"], [])

    def test_raw_positive_with_another_return_requires_review(self) -> None:
        case_id = "gsd:test-s1:1"
        case = {
            "id": case_id,
            "query": "현대증권",
            "pos": "noun",
            "text": "현대증권 배성영 수석연구원은 말했다.",
            "expected": True,
            "gold_byte_start": 0,
            "gold_byte_end": 12,
        }
        gold = {"byte_start": 0, "byte_end": 12}
        additional = {"byte_start": 13, "byte_end": 22}
        for matches in ([gold], [gold, additional]):
            with self.subTest(matches=matches):
                result = SCORE_CONTRACT(
                    [case], {case_id: True}, {case_id: matches}, {}, include_residual=True
                )
                pending = len(matches) - 1
                self.assertEqual(result["contract"]["contract_tp"], 1)
                self.assertEqual(result["reviewed_ranges"]["unverified_raw_positive_rows"], pending)
                self.assertEqual(result["reviewed_ranges"]["unverified_raw_positive_spans"], pending)
                self.assertTrue(result["contract_errors"]["zero_errors_observed"])
                self.assertEqual(result["contract_errors"]["zero_target_verified"], pending == 0)
                if pending:
                    self.assertEqual(result["residual"], [{
                        "id": case_id,
                        "expected": True,
                        "spans": matches,
                        "false_positive": False,
                        "false_negative": False,
                        "unverified": [(13, 22)],
                    }])
                else:
                    self.assertEqual(result["residual"], [])

    def test_verified_raw_positive_can_return_multiple_correct_ranges(self) -> None:
        case_id = "pud:n01144038:noun:거"
        review = self.reviews[case_id]
        case = {
            "id": case_id,
            "query": "거",
            "pos": "noun",
            "text": review["source_text"],
            "expected": True,
            "gold_byte_start": 131,
            "gold_byte_end": 134,
        }
        result = SCORE_CONTRACT(
            [case], {case_id: True}, {case_id: review["source_spans"]},
            self.reviews, include_residual=True,
        )
        self.assertEqual(result["reviewed_ranges"]["verified_rows"], 1)
        self.assertEqual(result["reviewed_ranges"]["unverified_raw_positive_rows"], 0)
        self.assertTrue(result["contract_errors"]["zero_target_verified"])
        self.assertEqual(result["residual"], [])

    def test_raw_and_corrected_ranges_use_their_own_registered_review(self) -> None:
        case_id = "gsd:test-s95:13"
        source_ranges = AUDIT["source_ranges_for_reviews"]
        raw = source_ranges(REGISTRY, {
            ("tools/morph-compare/raw-gsd-source-corpus-reviews.tsv", case_id),
        })
        corrected = source_ranges(REGISTRY, {
            ("tools/morph-compare/gsd-positive-corpus-reviews.tsv", case_id),
        })
        raw_gold = {"byte_start": 96, "byte_end": 102}
        corrected_gold = {"byte_start": 94, "byte_end": 100}
        self.assertEqual(raw[case_id]["source_spans"], [raw_gold])
        self.assertEqual(corrected[case_id]["source_spans"], [corrected_gold])

        case = {
            "id": case_id,
            "query": "하다",
            "pos": "verb",
            "text": raw[case_id]["source_text"],
            "expected": True,
            "gold_byte_start": 96,
            "gold_byte_end": 102,
        }
        spans = {case_id: [raw_gold, {"byte_start": 11, "byte_end": 14}]}
        result = SCORE_CONTRACT([case], {case_id: True}, spans, raw, include_residual=True)
        self.assertEqual(result["contract"]["contract_tp"], 1)
        self.assertEqual(result["contract_errors"]["fpc"], 1)
        self.assertEqual(result["residual"][0]["extra"], [(11, 14)])
        with self.assertRaisesRegex(ValueError, "source range review identity differs"):
            SCORE_CONTRACT([case], {case_id: True}, spans, corrected, include_residual=True)

    def test_runner_output_is_bound_to_the_input_bytes_and_query_mode(self) -> None:
        case = {"id": "same", "query": "회사", "text": "회사가 있다"}
        checksum = hashlib.sha256(json.dumps(case).encode()).hexdigest()
        result = {
            "fixture_sha256": checksum,
            "profile": "full-pos",
            "boundary": "smart",
            "query_mode": "explicit-pos",
            "results": [{"id": "same"}],
        }
        validate = AUDIT["validate_runner_output"]
        validate(result, checksum, [case])
        for changed in ({**case, "query": "직원"}, {**case, "text": "직원이 있다"}):
            changed_checksum = hashlib.sha256(json.dumps(changed).encode()).hexdigest()
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                validate({**result, "fixture_sha256": changed_checksum}, checksum, [case])
        with self.assertRaisesRegex(ValueError, "명시 품사"):
            validate({**result, "query_mode": "untagged"}, checksum, [case])

    def test_repeated_raw_positives_reuse_only_compatible_annotated_ranges(self) -> None:
        first = {
            "id": "first",
            "query": "길",
            "pos": "noun",
            "text": "그 길 저 길",
            "expected": True,
            "gold_byte_start": 4,
            "gold_byte_end": 7,
        }
        second = {**first, "id": "second", "gold_byte_start": 12, "gold_byte_end": 15}
        matches = [{"byte_start": 4, "byte_end": 7}, {"byte_start": 12, "byte_end": 15}]
        for peer, reusable in (
            (second, True),
            ({**second, "text": "그 길 이 길"}, False),
            ({**second, "query": "다른 질의"}, False),
            ({**second, "pos": "verb"}, False),
            ({
                **second,
                "contract_expected": False,
                "contract_reason": "gold-alignment-error",
            }, False),
        ):
            with self.subTest(peer=peer):
                result = SCORE_CONTRACT(
                    [first, peer], {"first": True, "second": True},
                    {"first": matches, "second": matches[1:]},
                    {}, include_residual=True,
                )
                self.assertEqual(result["reviewed_ranges"]["reused_raw_positive_spans"], int(reusable))
                self.assertEqual(
                    result["reviewed_ranges"]["unverified_raw_positive_rows"], int(not reusable)
                )
                if not reusable:
                    unresolved = next(row for row in result["residual"] if row["id"] == "first")
                    self.assertEqual(unresolved["unverified"], [(12, 15)])
                else:
                    self.assertEqual(result["residual"], [])

    @staticmethod
    def positive_case(case_id: str, text: str) -> dict:
        return {
            "id": case_id,
            "text": text,
            "expected": False,
            "contract_expected": True,
            "contract_reason": "aligned-source-component",
        }
