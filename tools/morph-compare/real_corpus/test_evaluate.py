import copy
import hashlib
import unittest
from pathlib import Path

from evaluate import (
    evaluate_profile,
    load_json,
    load_jsonl,
    validate_cases,
    validate_sources,
)
from verify_sources import raw_url, verify_source_bytes


FIXTURE_DIR = Path(__file__).parent


class RealCorpusEvaluationTests(unittest.TestCase):
    def setUp(self):
        self.manifest = load_json(FIXTURE_DIR / "sources.json")
        self.files_by_key = validate_sources(self.manifest)
        self.cases = load_jsonl(FIXTURE_DIR / "cases.jsonl")

    def test_repository_fixture_is_valid(self):
        validate_cases(self.cases, self.files_by_key)

        self.assertEqual(len(self.cases), 25)
        self.assertEqual(sum(case["expected"] for case in self.cases), 21)

    def test_canonical_duplicate_is_rejected(self):
        duplicate = copy.deepcopy(self.cases[0])
        duplicate["id"] = "duplicate"
        duplicate["text"] = f"  {duplicate['text']}  "

        with self.assertRaisesRegex(ValueError, "duplicates canonical text"):
            validate_cases([*self.cases, duplicate], self.files_by_key)

    def test_invalid_gold_span_is_rejected(self):
        cases = copy.deepcopy(self.cases)
        cases[0]["gold_byte_start"] += 1

        with self.assertRaisesRegex(ValueError, "gold span"):
            validate_cases(cases, self.files_by_key)

    def test_profile_metrics_require_gold_overlap(self):
        results = []
        for case in self.cases:
            spans = []
            if case["id"] == "identifier-adjacent-01":
                spans = [
                    {
                        "byte_start": case["gold_byte_start"],
                        "byte_end": case["gold_byte_end"],
                    }
                ]
            elif case["id"] == "homonym-02":
                spans = [{"byte_start": 0, "byte_end": 1}]
            results.append({"id": case["id"], "spans": spans})
        profile = {
            "backend": "fixture",
            "profile": "embedded",
            "boundary": "any",
            "results": results,
        }

        evaluated = evaluate_profile("agent", profile, self.cases)

        self.assertEqual(evaluated["contract"]["query_mode"], "explicit-pos")
        self.assertEqual(evaluated["overall"]["tp"], 1)
        self.assertEqual(evaluated["overall"]["fp"], 1)
        self.assertEqual(evaluated["overall"]["fn"], 20)
        self.assertEqual(evaluated["overall"]["tn"], 3)

    def test_pinned_source_bytes_are_verified(self):
        source_bytes = "한국어 원문\n".encode()
        source_file = {
            "path": "README.md",
            "sha256": hashlib.sha256(source_bytes).hexdigest(),
        }

        lines = verify_source_bytes("fixture", source_file, source_bytes)

        self.assertEqual(lines, ["한국어 원문"])
        self.assertEqual(
            raw_url("https://github.com/example/repo", "a" * 40, "docs/ko.md"),
            f"https://raw.githubusercontent.com/example/repo/{'a' * 40}/docs/ko.md",
        )


if __name__ == "__main__":
    unittest.main()
