from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from validation import sha256, validate_robustness_dataset, validate_split_isolation


class SplitIsolationTests(unittest.TestCase):
    def test_repeated_queries_within_a_split_are_allowed(self) -> None:
        validate_split_isolation(
            [{"id": "dev-1", "text": "학교에 갔다."}, {"id": "dev-2", "text": "학교에 갔다."}],
            [{"id": "test-1", "text": "병원에 갔다."}],
        )

    def test_duplicate_sentence_with_different_ids_and_normalization_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, r"share 1 normalized sentences .*dev-1 and test-1"):
            validate_split_isolation(
                [{"id": "dev-1", "text": "학교에 갔다."}],
                [{"id": "test-1", "text": "학교에\t갔다."}],
            )


class RobustnessValidationTests(unittest.TestCase):
    def fixture(
        self, directory: Path
    ) -> tuple[Path, list[dict[str, object]], dict[str, object]]:
        cases = [
            {
                "id": "positive",
                "expected": True,
                "noise_origin": "natural",
                "noise_class": "hangul-typo",
                "noise_scope": "target-span",
            },
            {
                "id": "negative",
                "expected": False,
                "noise_origin": "natural",
                "noise_class": "hangul-typo",
                "noise_scope": "context-only",
            },
        ]
        path = directory / "cases.jsonl"
        path.write_text(
            "".join(json.dumps(case) + "\n" for case in cases),
            encoding="utf-8",
        )
        metadata = {
            "source_set": "robustness",
            "scoring_status": "scored",
            "fixture_type": "robustness",
            "query_mode": "explicit-pos",
            "fixture_sha256": sha256(path),
            "cases": 2,
            "positive_cases": 1,
            "negative_cases": 1,
            "case_review": {"status": "reviewed"},
        }
        return path, cases, metadata

    def test_accepts_balanced_reviewed_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, cases, metadata = self.fixture(Path(directory))

            validate_robustness_dataset(path, cases, metadata, "explicit-pos")

    def test_rejects_unreviewed_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, cases, metadata = self.fixture(Path(directory))
            metadata["case_review"] = {"status": "draft"}

            with self.assertRaisesRegex(ValueError, "reviewed query-level gold"):
                validate_robustness_dataset(path, cases, metadata, "explicit-pos")


if __name__ == "__main__":
    unittest.main()
