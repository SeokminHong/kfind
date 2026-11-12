import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from audit_ending_runtime import example_candidates


class EndingAlignmentTests(unittest.TestCase):
    def test_explicit_copula_alignment_starts_after_the_nominal_host(self):
        text = "오늘은 휴일이거든요."
        tokens = [SimpleNamespace(tag="VCP", form="이", start=text.index("이"), len=1)]
        examples = [("fixture", "1", text, False)]

        [case] = example_candidates("거든요", examples, {}, {}, lambda _: tokens)

        self.assertEqual(case["query"], "이다")
        self.assertEqual(case["surface"], "이거든요")
        self.assertEqual(
            text.encode()[case["gold_byte_start"]:case["gold_byte_end"]].decode(),
            "이거든요",
        )
