import unittest
from pathlib import Path

from task_search import matches_from_json, score


class TaskSearchTests(unittest.TestCase):
    def test_native_and_ripgrep_json_describe_the_same_match(self):
        native = b'{"type":"match","path":"/a","line":2,"text":"abc","spans":[{"token":{"start":0,"end":3}}]}'
        rg = b'{"type":"match","data":{"path":{"text":"/a"},"line_number":2,"lines":{"text":"abc\\n"},"submatches":[{"start":0,"end":3}]}}'
        self.assertEqual(matches_from_json(native), matches_from_json(rg, True))

    def test_other_lines_do_not_make_the_gold_span_a_true_positive(self):
        cache = Path("/corpus")
        case = {
            "source_id": "source",
            "source_path": "a.txt",
            "source_line_start": 2,
            "source_line_end": 2,
            "expected": True,
            "gold_byte_start": 0,
            "gold_byte_end": 6,
        }
        matches = [
            {
                "path": "/corpus/source/a.txt",
                "line": 1,
                "text": "한글",
                "spans": [{"byte_start": 0, "byte_end": 6}],
            }
        ]
        result = score(case, matches, cache, ["한글", "한글"])
        self.assertTrue(result["target_file_found"])
        self.assertEqual(result["classification"], "fn")
        self.assertEqual(result["candidate_text_bytes"], 6)
        matches[0]["line"] = 2
        self.assertEqual(
            score(case, matches, cache, ["한글", "한글"])["classification"], "tp"
        )
        case["expected"] = False
        self.assertEqual(
            score(case, matches, cache, ["한글", "한글"])["classification"], "fp"
        )


if __name__ == "__main__":
    unittest.main()
