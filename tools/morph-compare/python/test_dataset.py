import json
import tempfile
import unittest
from pathlib import Path

from dataset import locate_token_spans, normalize_gold, parse_conllu, sha256
from local_context_dataset import build_local_context_dataset


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

    def test_local_context_dataset_preserves_gold_and_surface_negatives(self) -> None:
        fixture = """# sent_id = positive
# text = 학생이다.
1\t학생이다\t학생+이+다\tVERB\tncn+jp+ef\t_\t0\troot\t_\tSpaceAfter=No
2\t.\t.\tPUNCT\tsf\t_\t1\tpunct\t_\t_

# sent_id = negative
# text = 매일 운동한다.
1\t매일\t매일\tNOUN\tncn\t_\t2\tobl\t_\t_
2\t운동한다\t운동+하+ㄴ다\tVERB\tncn+xsv+ef\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tsf\t_\t2\tpunct\t_\t_

# sent_id = excluded
# text = 있다.
1\t있다\t있+다\tVERB\tjp+ef\t_\t0\troot\t_\tSpaceAfter=No
2\t.\t.\tPUNCT\tsf\t_\t1\tpunct\t_\t_

# sent_id = unaligned
# text = 이상하다.
1\t이상하다\t이상+하+다\tVERB\tncn+ef\t_\t0\troot\t_\tSpaceAfter=No
2\t.\t.\tPUNCT\tsf\t_\t1\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_path = root / "sample.conllu"
            source_path.write_text(fixture, encoding="utf-8")
            manifest = {
                "schema_version": 2,
                "ud_release": "test",
                "local_context": {
                    "seed": "test-seed",
                    "split": "dev",
                    "expected_excluded_candidates": 1,
                    "analyses": [
                        {
                            "source": "sample",
                            "raw_tag": "jp",
                            "raw_lemma": "이",
                            "query": "이다",
                            "pos": "adjective",
                            "negative_surface_cues": ["이", "인", "일"],
                            "positive_cases": 1,
                            "negative_cases": 1,
                        }
                    ],
                },
                "sources": [
                    {
                        "name": "sample",
                        "splits": {
                            "dev": {
                                "data_file": source_path.name,
                                "data_url": "https://example.invalid/sample.conllu",
                                "data_sha256": sha256(source_path),
                            }
                        },
                        "license": "test",
                        "license_file": "LICENSE",
                    }
                ],
            }
            manifest_path = root / "sources.json"
            output = root / "cases.jsonl"
            metadata_path = root / "metadata.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            metadata = build_local_context_dataset(
                manifest_path, root, output, metadata_path
            )
            cases = [json.loads(line) for line in output.read_text().splitlines()]
            fixture_digest = sha256(output)

        self.assertEqual(
            (metadata["positive_cases"], metadata["negative_cases"]), (1, 1)
        )
        self.assertEqual(
            {case["slice"] for case in cases},
            {"gold-copula", "surface-without-gold"},
        )
        self.assertEqual(
            metadata["excluded_candidates"], {"sample:jp:있:있다": 1}
        )
        self.assertEqual(metadata["fixture_sha256"], fixture_digest)


if __name__ == "__main__":
    unittest.main()
