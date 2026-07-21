import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from apply_fixture_contract import apply_fixture_contract
from validation import load_cases, sha256, validate_contract_review_metadata


class ApplyFixtureContractTests(unittest.TestCase):
    def test_applies_scoped_reviews_and_updates_metadata_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cases_path = root / "cases.jsonl"
            metadata_path = root / "metadata.json"
            reviews_path = root / "reviews.tsv"
            text = "붙여쓴 입력"
            case = {
                "id": "case-1",
                "query": "입력",
                "pos": "noun",
                "expected": True,
                "text": text,
            }
            cases_path.write_text(
                json.dumps(case, ensure_ascii=False) + "\n", encoding="utf-8"
            )
            metadata_path.write_text(
                json.dumps(
                    {
                        "split": "dev",
                        "query_mode": "explicit-pos",
                        "fixture_sha256": sha256(cases_path),
                    }
                ),
                encoding="utf-8",
            )
            reviews_path.write_text(
                "\t".join(
                    [
                        "query_mode",
                        "split",
                        "case_id",
                        "query",
                        "pos",
                        "strict_expected",
                        "text_sha256",
                        "contract_status",
                        "contract_reason",
                        "note",
                    ]
                )
                + "\n"
                + "\t".join(
                    [
                        "explicit-pos",
                        "dev",
                        "case-1",
                        "입력",
                        "noun",
                        "true",
                        hashlib.sha256(text.encode()).hexdigest(),
                        "excluded",
                        "nonstandard-input",
                        "표준문 계약 밖의 붙여쓰기",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            metadata = apply_fixture_contract(
                cases_path=cases_path,
                metadata_path=metadata_path,
                reviews_path=reviews_path,
            )

            reviewed = load_cases(cases_path)[0]
            self.assertIsNone(reviewed["contract_expected"])
            self.assertEqual("nonstandard-input", reviewed["contract_reason"])
            self.assertEqual(1, metadata["contract_review"]["excluded_cases"])
            self.assertEqual(sha256(cases_path), metadata["fixture_sha256"])
            validate_contract_review_metadata([reviewed], metadata)

            metadata["contract_review"]["excluded_cases"] = 0
            with self.assertRaisesRegex(ValueError, "differs from cases"):
                validate_contract_review_metadata([reviewed], metadata)


if __name__ == "__main__":
    unittest.main()
