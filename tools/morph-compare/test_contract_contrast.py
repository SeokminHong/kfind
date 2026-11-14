from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

from python.query_matrix_contract import (
    ContractReview,
    apply_contract_reviews,
    load_contract_reviews,
)


class ContractContrastTests(unittest.TestCase):
    def test_span_contract_reviews_match_verify_gold(self) -> None:
        benchmark = Path(__file__).resolve().parent
        fixtures = benchmark.parents[1] / "data" / "fixtures"
        reviewed_spans = []
        for name in ("ud_morphology_cases.tsv", "jiyo_span_cases.tsv"):
            for line in (fixtures / name).read_text(encoding="utf-8").splitlines()[1:]:
                fields = line.split("\t")
                if len(fields) == 8:
                    _, query, pos, expected, start, end, text, reason = fields
                    reviewed_spans.append(
                        (query, pos, expected, int(start), int(end), text, reason)
                    )

        cases = self.read_jsonl(benchmark / "verify-gold-context-cases.jsonl")
        reviews = load_contract_reviews(benchmark / "verify-gold-contract-reviews.tsv")
        apply_contract_reviews(
            cases, reviews, query_mode="explicit-pos", split="verify-gold-context"
        )
        self.assertCountEqual(
            reviewed_spans,
            [
                (
                    case["query"], case["pos"], "no-match", case["gold_byte_start"],
                    case["gold_byte_end"], case["text"], case["contract_reason"],
                )
                for case in cases
                if case.get("contract_expected") is True
            ],
        )

    def test_all_contract_reviews_have_contrast_examples(self) -> None:
        benchmark = Path(__file__).resolve().parent
        repo = benchmark.parents[1]
        examples = self.read_jsonl(benchmark / "contract-contrast-evidence.jsonl")
        by_key = {(row["registry"], row["id"]): row for row in examples}
        self.assertEqual(len(examples), len(by_key))

        for registry in sorted(benchmark.glob("*-reviews.tsv")):
            name = registry.relative_to(repo).as_posix()
            reviews = load_contract_reviews(registry)
            context = registry.with_name(
                registry.name.removesuffix("-contract-reviews.tsv")
                + "-context-cases.jsonl"
            )
            paired = set()
            if context.is_file():
                with self.subTest(context=context.name):
                    paired = self.assert_context_pairs(context, reviews)
            for review in reviews:
                if review.case_id not in paired:
                    key = (name, review.case_id)
                    self.assertIn(key, by_key, f"missing contrast example: {key}")
                    self.assert_contrast(
                        by_key.pop(key), review.text_sha256, review.contract_reason
                    )

        for fixture in (
            "tools/morph-compare/hard-negatives.jsonl",
            "tools/search-baseline/fixture.jsonl",
        ):
            for case in self.read_jsonl(repo / fixture):
                if case.get("contract_reason"):
                    key = (fixture, case["id"])
                    self.assertIn(key, by_key, f"missing contrast example: {key}")
                    self.assert_contrast(
                        by_key.pop(key),
                        hashlib.sha256(case["text"].encode()).hexdigest(),
                        case["contract_reason"],
                    )

        self.assertFalse(by_key, f"contrast examples without a review: {list(by_key)}")

    def assert_contrast(self, row: dict, source_hash: str, reason: str) -> None:
        with self.subTest(registry=row["registry"], case=row["id"]):
            self.assertEqual(
                hashlib.sha256(row["source_text"].encode()).hexdigest(), source_hash
            )
            self.assertNotEqual(row["source_text"], row["contrast_text"])
            self.assertTrue(row["note"].strip())
            for role in ("source", "contrast"):
                self.assertTrue(row[f"{role}_surface"].strip())
                self.assertIn(row[f"{role}_surface"], row[f"{role}_text"])
            if reason in {"same-pos-homograph", "structurally-indistinguishable-homograph"}:
                self.assertEqual(row["source_surface"], row["contrast_surface"])

    def read_jsonl(self, path: Path) -> list[dict]:
        return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]

    def assert_context_pairs(self, path: Path, reviews: list[ContractReview]) -> set[str]:
        cases = self.read_jsonl(path)
        by_id = {case["id"]: case for case in cases}
        self.assertEqual(len(cases), len(by_id))
        reviews = [review for review in reviews if review.case_id in by_id]
        for review in reviews:
            case = by_id[review.case_id]
            self.assertFalse(case["expected"])
            scored_case = case.copy()
            apply_contract_reviews(
                [scored_case], [review], query_mode=review.query_mode, split=review.split
            )
            if "contract_expected" in case:
                self.assertIs(case["contract_expected"], scored_case["contract_expected"])

        for case in cases:
            with self.subTest(case=case["id"]):
                other = by_id[case["paired_case_id"]]
                self.assertEqual(case["id"], other["paired_case_id"])
                self.assertNotEqual(case["text"], other["text"])
                self.assertNotEqual(case["expected"], other["expected"])
                self.assertEqual(
                    (case["query"], case["pos"]), (other["query"], other["pos"])
                )
                surface = case["text"].encode()[case["gold_byte_start"]:case["gold_byte_end"]]
                other_surface = other["text"].encode()[other["gold_byte_start"]:other["gold_byte_end"]]
                self.assertTrue(surface)
                self.assertEqual(surface.decode(), other_surface.decode())
        return {review.case_id for review in reviews}
