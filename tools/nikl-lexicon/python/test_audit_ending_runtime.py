import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from audit_ending_runtime import example_candidates


class EndingAlignmentTests(unittest.TestCase):
    def test_explicit_copula_alignment_starts_after_the_nominal_host(self):
        for host, surface, ending in [
            ("휴일", "이거든요", "거든요"),
            ("학생", "일는지", "ㄹ는지"),
            ("학생", "입니까", "ㅂ니까"),
            ("잠자리", "였을망정", "을망정"),
        ]:
            with self.subTest(surface=surface):
                text = f"어제는 {host}{surface}."
                start = text.index(host) + len(host)
                tokens = [SimpleNamespace(tag="VCP", form="이", start=start, len=1)]
                examples = [("fixture", "1", text, False)]

                [case] = example_candidates(ending, examples, {}, {}, lambda _: tokens)

                self.assertEqual(case["query"], "이다")
                self.assertEqual(case["surface"], surface)
                self.assertEqual(
                    text.encode()[case["gold_byte_start"]:case["gold_byte_end"]].decode(),
                    surface,
                )

    def test_split_analysis_uses_the_attested_compound_lemma(self):
        text = "행여 눈치챌세라 방문을 열었다."
        tokens = [
            SimpleNamespace(tag="NNG", form="눈치", start=text.index("눈"), len=2),
            SimpleNamespace(tag="VV", form="채", start=text.index("챌"), len=1),
        ]
        examples = [("fixture", "1", text, False)]
        predicates = {"채다": {"verb"}, "눈치채다": {"verb"}}

        [case] = example_candidates("ㄹ세라", examples, predicates, {}, lambda _: tokens)

        self.assertEqual(case["query"], "눈치채다")
        self.assertEqual(case["surface"], "눈치챌세라")
