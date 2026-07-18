from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

from fnc_dispositions import (
    LEDGER_FIELDS,
    load_ledger,
    validate_disposition_ledger,
)


class DispositionLedgerTests(unittest.TestCase):
    def test_validates_exact_raw_false_negative_set(self) -> None:
        ledger = [self.row()]

        summary = validate_disposition_ledger(self.report(), ledger, "kfind-full-pos")

        self.assertEqual(summary.raw_false_negatives, 1)
        self.assertEqual(summary.unclassified_raw_false_negatives, 0)
        self.assertEqual(summary.disposition_counts, {"out-of-contract": 1})

    def test_rejects_a_stale_case(self) -> None:
        ledger = [self.row(case_id="matrix:stale")]

        with self.assertRaisesRegex(ValueError, "does not match current raw FN set"):
            validate_disposition_ledger(self.report(), ledger, "kfind-full-pos")

    def test_rejects_report_identity_drift(self) -> None:
        ledger = [self.row(gold_surface="다른 표면")]

        with self.assertRaisesRegex(ValueError, "identity differs from report"):
            validate_disposition_ledger(self.report(), ledger, "kfind-full-pos")

    def test_load_ledger_requires_the_exact_schema(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ledger.tsv"
            with path.open("w", encoding="utf-8", newline="") as ledger_file:
                writer = csv.DictWriter(
                    ledger_file, fieldnames=LEDGER_FIELDS[:-1], delimiter="\t"
                )
                writer.writeheader()

            with self.assertRaisesRegex(ValueError, "required schema"):
                load_ledger(path)

    @staticmethod
    def report() -> dict[str, object]:
        return {
            "query_matrix": {
                "explicit_pos": {
                    "dataset": {"fixture_sha256": "fixture-sha"},
                    "failures": [
                        {
                            "case": {
                                "id": "matrix:fn",
                                "expected": True,
                                "contract_expected": None,
                                "contract_reason": "out-of-contract",
                                "query": "없다",
                                "pos": "adjective",
                                "text": "거의 없이",
                                "gold_byte_start": 7,
                                "gold_byte_end": 13,
                            },
                            "predictions": {"kfind-full-pos": False},
                            "primary_cause": "gold-or-adapter",
                            "profile_causes": {
                                "kfind-full-pos": "gold-or-adapter"
                            },
                        }
                    ],
                }
            }
        }

    @staticmethod
    def row(**overrides: str) -> dict[str, str]:
        row = {
            "fixture_sha256": "fixture-sha",
            "backend": "kfind-full-pos",
            "case_id": "matrix:fn",
            "query": "없다",
            "pos": "adjective",
            "gold_surface": "없이",
            "failure_cause": "gold-or-adapter",
            "disposition": "out-of-contract",
            "rationale": "파생 부사 표면은 활용 검색 범위가 아니다.",
            "dictionary_evidence": "두 기본 사전은 별도 표제어로 기록한다.",
        }
        row.update(overrides)
        return row


if __name__ == "__main__":
    unittest.main()
