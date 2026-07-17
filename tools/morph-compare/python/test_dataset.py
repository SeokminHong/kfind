import tempfile
import unittest
from pathlib import Path

from dataset import (
    GoldCandidate,
    Sentence,
    apply_sentence_review,
    locate_token_spans,
    normalize_gold,
    parse_conllu,
    positive_case,
    review_pool_cases,
    review_pool_rows,
    review_pool_sha256,
    resolve_source_set,
    select_positives,
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

    def test_sentence_review_filters_a_pinned_pre_review_pool(self) -> None:
        fixture = """# sent_id = positive
# text = 새가 난다.
1\t새가\t새+가\tNOUN\tNNG+JKS\t_\t2\tnsubj\t_\t_
2\t난다\t날+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

# sent_id = negative
# text = 구름이 온다.
1\t구름이\t구름+이\tNOUN\tNNG+JKS\t_\t2\tnsubj\t_\t_
2\t온다\t오+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.conllu"
            path.write_text(fixture, encoding="utf-8")
            sentences, _ = parse_conllu("sample", path)

        quotas = {"noun": 1}
        rows = review_pool_rows(review_pool_cases(sentences, quotas, "seed"))
        rejected_id = rows[0]["sent_id"]
        reviews = {
            "schema_version": 1,
            "review_policy": "test-policy",
            "splits": {
                "test": {
                    "sample": {
                        "positive_quotas_per_source": quotas,
                        "pool_sentences": len(rows),
                        "pool_sha256": review_pool_sha256(rows),
                        "rejected": [
                            {
                                "sent_id": rejected_id,
                                "reason_class": "hangul-typo",
                                "annotation": "test rejection",
                            }
                        ],
                    }
                }
            },
        }
        accepted, summary = apply_sentence_review(
            sentences=sentences,
            source_name="sample",
            split_name="test",
            seed="seed",
            reviews=reviews,
            review_file="reviews.json",
        )

        self.assertEqual(len(accepted), len(rows) - 1)
        self.assertNotIn(rejected_id, {sentence.sent_id for sentence in accepted})
        self.assertEqual(summary["rejected_sentences"], 1)
        self.assertEqual(summary["pool_sha256"], review_pool_sha256(rows))

    def test_positive_selection_caps_each_sentence(self) -> None:
        def candidate(sent_id: str, query: str, index: int) -> GoldCandidate:
            return GoldCandidate(
                source="sample",
                sent_id=sent_id,
                text=f"{sent_id} text",
                token_id=str(index),
                morph_index=0,
                query=query,
                pos="noun",
                byte_start=index,
                byte_end=index + 1,
                raw_lemma=query,
                raw_tag="NNG",
            )

        crowded = Sentence(
            "sample",
            "crowded",
            "crowded text",
            tuple(candidate("crowded", f"명사{index}", index) for index in range(4)),
            True,
        )
        fallback = Sentence(
            "sample",
            "fallback",
            "fallback text",
            (candidate("fallback", "대체", 5),),
            True,
        )

        selected = select_positives(
            [crowded, fallback],
            {"noun": 4},
            "seed",
            max_per_sentence=3,
        )

        self.assertEqual(len(selected), 4)
        self.assertLessEqual(
            sum(case.sent_id == "crowded" for case in selected),
            3,
        )

    def test_manifest_source_selection_is_explicit(self) -> None:
        manifest = {
            "schema_version": 4,
            "sources": [{"name": "development"}, {"name": "blind"}],
        }
        self.assertEqual(
            [
                source["name"]
                for source in select_manifest_sources(manifest, ["blind"])
            ],
            ["blind"],
        )

    def test_source_sets_keep_scored_and_annotation_required_corpora_separate(self) -> None:
        manifest = {
            "schema_version": 4,
            "source_sets": {
                "canonical": {
                    "sources": ["edited"],
                    "positive_quotas_per_source": {"noun": 2},
                    "scoring_status": "scored",
                },
                "robustness-candidate": {
                    "sources": ["learner"],
                    "positive_quotas_per_source": {"noun": 1},
                    "scoring_status": "annotation-required",
                },
            },
            "sources": [{"name": "edited"}, {"name": "learner"}],
        }

        canonical, canonical_quotas, canonical_status = resolve_source_set(
            manifest, "canonical"
        )
        robustness, robustness_quotas, robustness_status = resolve_source_set(
            manifest, "robustness-candidate"
        )

        self.assertEqual([source["name"] for source in canonical], ["edited"])
        self.assertEqual(canonical_quotas, {"noun": 2})
        self.assertEqual(canonical_status, "scored")
        self.assertEqual([source["name"] for source in robustness], ["learner"])
        self.assertEqual(robustness_quotas, {"noun": 1})
        self.assertEqual(robustness_status, "annotation-required")


if __name__ == "__main__":
    unittest.main()
