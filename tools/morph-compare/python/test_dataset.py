import json
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
    sha256,
)
from local_context_dataset import (
    build_local_context_dataset,
    validate_disjoint_sources,
)
from validation import validate_local_context_dataset


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
                "schema_version": 3,
                "ud_release": "test",
                "benchmark_sources": ["sample"],
                "local_context": {
                    "seed": "test-seed",
                    "split": "dev",
                    "metadata_split": "dev-local-context",
                    "sort_scope": "local-context-order",
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
                        "license_url": "https://example.invalid/LICENSE",
                        "license_sha256": "unused",
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
            validate_local_context_dataset(output, cases, metadata)
            metadata["group_counts"].append(
                {
                    "source": "sample",
                    "raw_tag": "vcn",
                    "positive_cases": 0,
                    "negative_cases": 0,
                }
            )
            validate_local_context_dataset(output, cases, metadata)

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

    def test_pud_adapter_preserves_source_copula_exclusions(self) -> None:
        fixture = """# sent_id = positive
# text = 학생 이다.
1\t학생\t학생\tNOUN\tNNG\t_\t0\troot\t_\t_
2\t이다\t이\tAUX\tVC\tMood=Ind\t1\tcop\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t1\tpunct\t_\t_

# sent_id = excluded
# text = 기능 이다.
1\t기능\t기능\tNOUN\tNNG\t_\t0\troot\t_\t_
2\t이다\t_\tAUX\tVC\tMood=Ind\t1\tcop\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t1\tpunct\t_\t_

# sent_id = negative
# text = 매일 운동한다.
1\t매일\t매일\tNOUN\tNNG\t_\t2\tobl\t_\t_
2\t운동한다\t운동하다\tVERB\tVV\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_path = root / "sample.conllu"
            source_path.write_text(fixture, encoding="utf-8")
            manifest = {
                "schema_version": 3,
                "ud_release": "test",
                "unseen_local_context": {
                    "seed": "test-seed",
                    "split": "test",
                    "metadata_split": "unseen-local-context",
                    "sort_scope": "unseen-context-order",
                    "expected_excluded_candidates": 1,
                    "analyses": [
                        {
                            "source": "sample",
                            "raw_tag": "vc",
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
                        "adapter": "pud-copula",
                        "splits": {
                            "test": {
                                "data_file": source_path.name,
                                "data_url": "https://example.invalid/sample.conllu",
                                "data_sha256": sha256(source_path),
                            }
                        },
                        "license": "test",
                        "license_file": "LICENSE",
                        "license_url": "https://example.invalid/LICENSE",
                        "license_sha256": "unused",
                    }
                ],
            }
            manifest_path = root / "sources.json"
            output = root / "cases.jsonl"
            metadata_path = root / "metadata.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            metadata = build_local_context_dataset(
                manifest_path,
                root,
                output,
                metadata_path,
                "unseen_local_context",
            )
            cases = [json.loads(line) for line in output.read_text().splitlines()]
            validate_local_context_dataset(
                output, cases, metadata, "unseen-local-context"
            )

        positive = next(case for case in cases if case["expected"])
        negative = next(case for case in cases if not case["expected"])
        self.assertEqual(positive["id"], "pos:sample:positive:2:0")
        self.assertEqual(positive["gold_raw_tag"], "VC")
        self.assertEqual(
            (positive["gold_byte_start"], positive["gold_byte_end"]), (7, 13)
        )
        self.assertEqual(positive["target_group"], "sample/vc")
        self.assertEqual(negative["sent_id"], "negative")
        self.assertEqual(metadata["excluded_candidates"], {"sample:vc:_:이다": 1})
        self.assertEqual(metadata["sources"][0]["parsing"]["source_copula_tokens"], 2)
        self.assertEqual(
            metadata["sources"][0]["parsing"]["source_copula_missing_lemma"], 1
        )

    def test_blind_context_rejects_normalized_sentence_overlap(self) -> None:
        fixture = """# sent_id = overlap
# text = 학생이다.
1\t학생이다\t학생+이+다\tVERB\tncn+jp+ef\t_\t0\troot\t_\tSpaceAfter=No
2\t.\t.\tPUNCT\tsf\t_\t1\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference_path = root / "reference.conllu"
            reference_path.write_text(fixture, encoding="utf-8")
            target_sentences, _ = parse_conllu("blind", reference_path)
            sources = {
                "reference": {
                    "name": "reference",
                    "splits": {
                        "dev": {
                            "data_file": reference_path.name,
                            "data_sha256": sha256(reference_path),
                        }
                    },
                }
            }
            config = {
                "disjoint_from": [{"source": "reference", "split": "dev"}]
            }
            with self.assertRaisesRegex(ValueError, "overlaps reference/dev"):
                validate_disjoint_sources(
                    config, sources, root, {"blind": target_sentences}
                )


if __name__ == "__main__":
    unittest.main()
