import tempfile
import unittest
from pathlib import Path

from dataset import (
    locate_token_spans,
    normalize_gold,
    parse_conllu,
    positive_case,
    select_untagged_negative,
    select_manifest_sources,
)


class DatasetTests(unittest.TestCase):
    def test_gold_tag_normalization_supports_both_treebanks(self) -> None:
        self.assertEqual(normalize_gold("가", "VV"), ("가다", "verb"))
        self.assertEqual(normalize_gold("가", "pvg"), ("가다", "verb"))
        self.assertEqual(normalize_gold("사람", "NNG"), ("사람", "noun"))
        self.assertIsNone(normalize_gold("을", "JKO"))

    def test_token_alignment_uses_utf8_byte_offsets(self) -> None:
        self.assertEqual(
            locate_token_spans("가 나.", ["가", "나", "."]),
            [(0, 3), (4, 7), (7, 8)],
        )

    def test_conllu_parser_preserves_eojeol_span(self) -> None:
        fixture = """# sent_id = test-1
# text = 오후에 갔어요.
1\t오후에\t오후+에\tADV\tNNG+JKB\t_\t2\tobl\t_\t_
2\t갔어요\t가+았+어요\tVERB\tVV+EP+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.conllu"
            path.write_text(fixture, encoding="utf-8")
            sentences, stats = parse_conllu("sample", path)
        self.assertEqual(stats["sentences"], 1)
        verb = next(item for item in sentences[0].candidates if item.pos == "verb")
        self.assertEqual((verb.query, verb.byte_start, verb.byte_end), ("가다", 10, 19))

    def test_orig_lemma_recovers_aligned_auxiliary(self) -> None:
        fixture = """# sent_id = test-2
# text = 있는 것이다.
1\t있는\t있\tAUX\tpx+etm\t_\t2\taux\t_\tOrigLemma=있+는
2\t것이다\t것+이+다\tVERB\tnbn+jp+ef\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tsf\t_\t2\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.conllu"
            path.write_text(fixture, encoding="utf-8")
            sentences, stats = parse_conllu("sample", path)
        self.assertEqual(stats["orig_lemma_tokens"], 1)
        self.assertIn(
            ("있다", "verb"),
            {(item.query, item.pos) for item in sentences[0].candidates},
        )

    def test_untagged_negative_excludes_the_query_under_every_pos(self) -> None:
        fixture = """# sent_id = positive
# text = 새가 난다.
1\t새가\t새+가\tNOUN\tNNG+JKS\t_\t2\tnsubj\t_\t_
2\t난다\t날+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

# sent_id = other-pos
# text = 새 기능이다.
1\t새\t새\tDET\tMM\t_\t2\tdet\t_\t_
2\t기능이다\t기능+이+다\tVERB\tNNG+VCP+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

# sent_id = absent
# text = 헌 기능이다.
1\t헌\t헌\tDET\tMM\t_\t2\tdet\t_\t_
2\t기능이다\t기능+이+다\tVERB\tNNG+VCP+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.conllu"
            path.write_text(fixture, encoding="utf-8")
            sentences, _ = parse_conllu("sample", path)

        noun = next(
            candidate
            for candidate in sentences[0].candidates
            if candidate.query == "새" and candidate.pos == "noun"
        )
        negative = select_untagged_negative(
            positive_case(noun), sentences, "test-seed"
        )

        self.assertEqual("absent", negative.sent_id)

    def test_manifest_source_selection_is_explicit(self) -> None:
        manifest = {
            "schema_version": 3,
            "sources": [{"name": "development"}, {"name": "blind"}],
        }
        self.assertEqual(
            [
                source["name"]
                for source in select_manifest_sources(manifest, ["blind"])
            ],
            ["blind"],
        )


if __name__ == "__main__":
    unittest.main()
