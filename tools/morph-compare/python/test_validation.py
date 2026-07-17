from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from validation import sha256, validate_robustness_candidate_dataset


class RobustnessCandidateValidationTests(unittest.TestCase):
    def fixture(
        self, directory: Path
    ) -> tuple[Path, list[dict[str, object]], dict[str, object]]:
        cases = [
            {"id": "positive", "expected": True},
            {"id": "negative", "expected": False},
        ]
        path = directory / "cases.jsonl"
        path.write_text(
            "".join(json.dumps(case) + "\n" for case in cases),
            encoding="utf-8",
        )
        metadata = {
            "source_set": "robustness-candidate",
            "scoring_status": "annotation-required",
            "query_mode": "explicit-pos",
            "fixture_sha256": sha256(path),
            "cases": 2,
            "positive_cases": 1,
            "negative_cases": 1,
        }
        return path, cases, metadata

    def test_accepts_balanced_annotation_required_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, cases, metadata = self.fixture(Path(directory))

            validate_robustness_candidate_dataset(
                path, cases, metadata, "explicit-pos"
            )

    def test_rejects_scored_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, cases, metadata = self.fixture(Path(directory))
            metadata["scoring_status"] = "scored"

            with self.assertRaisesRegex(ValueError, "annotation-required"):
                validate_robustness_candidate_dataset(
                    path, cases, metadata, "explicit-pos"
                )


if __name__ == "__main__":
    unittest.main()
