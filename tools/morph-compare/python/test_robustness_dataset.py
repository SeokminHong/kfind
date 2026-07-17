import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from robustness_dataset import (
    build_robustness_dataset,
    case_review_sha256,
    parse_source_signals,
    source_signal_rows,
)
from dataset import review_pool_sha256


FIXTURE = """# sent_id = noisy-positive
# text = 사람믈 본다.
1\t사람믈\t사람+을\tNOUN\tNNG+JKO\tTypo=Yes\t2\tobj\t_\t_
2\t본다\t보+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

# sent_id = noisy-negative
# text = 구르미 온다.
1\t구르미\t구름+이\tNOUN\tNNG+JKS\tTypo=Yes\t2\tnsubj\t_\t_
2\t온다\t오+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
3\t.\t.\tPUNCT\tSF\t_\t2\tpunct\t_\t_

"""


class RobustnessDatasetTests(unittest.TestCase):
    def test_source_signals_include_typo_rows(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.conllu"
            path.write_text(FIXTURE, encoding="utf-8")
            signals = parse_source_signals(path)

        self.assertEqual(set(signals), {"noisy-positive", "noisy-negative"})
        self.assertEqual(signals["noisy-positive"].typo_forms, ("사람믈",))

    def test_case_review_hash_changes_with_gold(self) -> None:
        case = {
            "id": "case",
            "sent_id": "sentence",
            "text": "사람믈 본다.",
            "query": "사람",
            "pos": "noun",
            "expected": True,
            "gold_byte_start": 0,
            "gold_byte_end": 9,
            "noise_class": "hangul-typo",
            "noise_scope": "target-span",
        }

        original = case_review_sha256([case])
        changed = case_review_sha256([{**case, "query": "구름"}])

        self.assertNotEqual(original, changed)

    def test_builds_balanced_scored_fixture_from_reviewed_noise(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = root / "corpora"
            sources.mkdir()
            source_path = sources / "sample.conllu"
            source_path.write_text(FIXTURE, encoding="utf-8")
            signals = parse_source_signals(source_path)
            review = {
                "schema_version": 1,
                "review_policy": "test-policy",
                "source": "learner",
                "split": "test",
                "source_signal_pool": {
                    "signals": ["Typo=Yes"],
                    "pool_sentences": len(signals),
                    "pool_sha256": review_pool_sha256(
                        source_signal_rows("learner", signals)
                    ),
                },
                "excluded": [],
                "supplements": [],
                "class_overrides": [],
                "candidate_corrections": [],
                "candidate_rejections": [],
                "negative_rejections": {
                    "explicit-pos": [],
                    "untagged": [],
                },
                "case_reviews": {},
            }
            (root / "reviews.json").write_text(
                json.dumps(review, ensure_ascii=False), encoding="utf-8"
            )
            manifest = {
                "schema_version": 4,
                "ud_release": "test",
                "seed": "test-seed",
                "source_sets": {
                    "robustness": {
                        "sources": ["learner"],
                        "sentence_review_file": "reviews.json",
                        "positive_quotas_per_source": {"noun": 1},
                        "target_positive_quotas_per_source": {"noun": 1},
                        "scoring_status": "scored",
                    }
                },
                "sources": [
                    {
                        "name": "learner",
                        "description": "test source",
                        "splits": {
                            "test": {
                                "data_file": "sample.conllu",
                                "data_url": "https://example.invalid/sample.conllu",
                                "data_sha256": hashlib.sha256(
                                    FIXTURE.encode("utf-8")
                                ).hexdigest(),
                            }
                        },
                        "license": "test",
                        "license_file": "LICENSE",
                    }
                ],
            }
            manifest_path = root / "sources.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            output = root / "cases.jsonl"
            metadata_path = root / "metadata.json"

            metadata = build_robustness_dataset(
                manifest_path=manifest_path,
                sources_dir=sources,
                output=output,
                metadata_path=metadata_path,
                query_mode="explicit-pos",
                allow_draft_case_review=True,
            )

        self.assertEqual(metadata["cases"], 2)
        self.assertEqual(metadata["positive_cases"], 1)
        self.assertEqual(metadata["negative_cases"], 1)
        self.assertEqual(metadata["noise_scope_counts"], {
            "context-only": 1,
            "target-span": 1,
        })
        self.assertEqual(metadata["case_review"]["status"], "draft")


if __name__ == "__main__":
    unittest.main()
