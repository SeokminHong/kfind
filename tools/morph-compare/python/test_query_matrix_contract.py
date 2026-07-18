from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from query_matrix_contract import apply_contract_reviews, load_contract_reviews


HEADER = (
    "query_mode\tsplit\tcase_id\tquery\tpos\tstrict_expected\ttext_sha256\t"
    "contract_status\tcontract_reason\tnote\n"
)


class QueryMatrixContractReviewTests(unittest.TestCase):
    def test_applies_all_contract_review_statuses(self) -> None:
        cases = [
            self.case("ambiguous", "불과", "noun", False, "불과 수미터"),
            self.case("misaligned", "이", "pronoun", True, "이중 구조"),
            self.case("target", "없다", "adjective", True, "거의 없이"),
            self.case("nonstandard", "옆", "noun", True, "빙원옆에"),
        ]
        rows = [
            self.row(
                cases[0],
                "contract-positive",
                "structurally-indistinguishable-homograph",
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
        case = self.case("ambiguous", "불과", "noun", False, "불과 수미터")
        review = self.load_reviews(
            [
                self.row(
                    case,
                    "contract-positive",
                    "structurally-indistinguishable-homograph",
                )
            ]
        )[0]
        case["text"] = "불과 몇 미터"

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
