from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from query_matrix_contract import apply_contract_reviews, load_contract_reviews


HEADER = (
    "query_mode\tsplit\tcase_id\tquery\tpos\tstrict_expected\ttext_sha256\t"
    "contract_status\tcontract_reason\tnote\n"
)


class QueryMatrixContractReviewTests(unittest.TestCase):
    def test_reviewed_homographs_have_matching_context_pairs(self) -> None:
        root = Path(__file__).resolve().parents[1]
        for prefix, split in (
            ("resultative", "resultative"),
            ("query-matrix", "test"),
            ("trial", "trial"),
            ("copula-conditional", "copula-conditional"),
            ("interrogative", "interrogative"),
            ("prospective", "prospective"),
            ("seoya", "seoya"),
            ("meuro", "meuro"),
            ("mse", "mse"),
            ("dao", "dao"),
            ("jiyo", "jiyo"),
            ("humble", "humble"),
            ("casual-prefinal", "casual-prefinal"),
        ):
            with self.subTest(dataset=prefix):
                self.assert_context_pairs(root, prefix, split)

    def assert_context_pairs(self, root: Path, prefix: str, split: str) -> None:
        cases = [
            json.loads(line)
            for line in (root / f"{prefix}-context-cases.jsonl").read_text(
                encoding="utf-8"
            ).splitlines()
        ]
        by_id = {case["id"]: case for case in cases}
        self.assertEqual(len(cases), len(by_id))
        reviews = [
            review
            for review in load_contract_reviews(root / f"{prefix}-contract-reviews.tsv")
            if review.case_id in by_id
        ]
        self.assertEqual(
            {review.case_id for review in reviews},
            {case["id"] for case in cases if not case["expected"]},
        )
        apply_contract_reviews(
            cases, reviews, query_mode="explicit-pos", split=split
        )
        expected_by_review = {
            review.case_id: (
                None
                if review.contract_status == "excluded"
                else review.contract_reason
                in {"same-pos-homograph", "structurally-indistinguishable-homograph"}
            )
            for review in reviews
        }

        for case in cases:
            with self.subTest(case=case["id"]):
                other = by_id[case["paired_case_id"]]
                self.assertEqual(case["id"], other["paired_case_id"])
                self.assertNotEqual(case["text"], other["text"])
                self.assertNotEqual(case["expected"], other["expected"])
                self.assertEqual((case["query"], case["pos"]), (other["query"], other["pos"]))
                surface = case["text"].encode()[case["gold_byte_start"]:case["gold_byte_end"]]
                other_surface = other["text"].encode()[other["gold_byte_start"]:other["gold_byte_end"]]
                self.assertTrue(surface)
                self.assertEqual(surface.decode(), other_surface.decode())
                if not case["expected"]:
                    self.assertIs(case["contract_expected"], expected_by_review[case["id"]])

    def test_applies_all_contract_review_statuses(self) -> None:
        cases = [
            self.case("ambiguous", "걷다", "verb", False, "친구에게 전화를 걸었다."),
            self.case("misaligned", "이", "pronoun", True, "이중 구조"),
            self.case("target", "없다", "adjective", True, "거의 없이"),
            self.case("nonstandard", "옆", "noun", True, "빙원옆에"),
        ]
        rows = [
            self.row(
                cases[0],
                "contract-positive",
                "same-pos-homograph",
            ),
            self.row(cases[1], "contract-negative", "gold-alignment-error"),
            self.row(cases[2], "confirmed", "implementation-target"),
            self.row(cases[3], "excluded", "nonstandard-input"),
        ]

        reviews = self.load_reviews(rows)
        summary = apply_contract_reviews(
            cases, reviews, query_mode="explicit-pos", split="test"
        )

        self.assertIs(cases[0]["contract_expected"], True)
        self.assertIs(cases[1]["contract_expected"], False)
        self.assertIs(cases[2]["contract_expected"], True)
        self.assertIsNone(cases[3]["contract_expected"])
        self.assertEqual(4, summary["reviewed_cases"])
        self.assertEqual(1, summary["confirmed_cases"])
        self.assertEqual(2, summary["reclassified_cases"])
        self.assertEqual(1, summary["excluded_cases"])

    def test_rejects_changed_case_identity(self) -> None:
        case = self.case("ambiguous", "걷다", "verb", False, "친구에게 전화를 걸었다.")
        review = self.load_reviews(
            [
                self.row(
                    case,
                    "contract-positive",
                    "same-pos-homograph",
                )
            ]
        )[0]
        case["text"] = "친구에게 다시 전화를 걸었다."

        with self.assertRaisesRegex(ValueError, "identity differs"):
            apply_contract_reviews(
                [case], [review], query_mode="explicit-pos", split="test"
            )

    @staticmethod
    def case(
        case_id: str, query: str, pos: str, expected: bool, text: str
    ) -> dict[str, object]:
        return {
            "id": case_id,
            "query": query,
            "pos": pos,
            "expected": expected,
            "text": text,
        }

    def row(
        self,
        case: dict[str, object],
        status: str,
        reason: str,
    ) -> str:
        return "\t".join(
            (
                "explicit-pos",
                "test",
                str(case["id"]),
                str(case["query"]),
                str(case["pos"]),
                str(case["expected"]).lower(),
                hashlib.sha256(str(case["text"]).encode()).hexdigest(),
                status,
                reason,
                "reviewed",
            )
        )

    @staticmethod
    def load_reviews(rows: list[str]):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "reviews.tsv"
            path.write_text(HEADER + "\n".join(rows) + "\n", encoding="utf-8")
            return load_contract_reviews(path)


if __name__ == "__main__":
    unittest.main()
