from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from dataset import parse_conllu, positive_case
from query_matrix import build_query_matrix, query_matrix_metrics
from validation import load_cases, validate_query_matrix_dataset


CONLLU = """# sent_id = selected
# text = 새 새가 난다.
1\t새\t새\tDET\tMM\t_\t2\tdet\t_\t_
2\t새가\t새+가\tNOUN\tNNG+JKS\t_\t3\tnsubj\t_\t_
3\t난다\t날+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
4\t.\t.\tPUNCT\tSF\t_\t3\tpunct\t_\t_

# sent_id = pool
# text = 헌 구름이 온다.
1\t헌\t헌\tDET\tMM\t_\t2\tdet\t_\t_
2\t구름이\t구름+이\tNOUN\tNNG+JKS\t_\t3\tnsubj\t_\t_
3\t온다\t오+ㄴ다\tVERB\tVV+EF\t_\t0\troot\t_\tSpaceAfter=No
4\t.\t.\tPUNCT\tSF\t_\t3\tpunct\t_\t_

"""


class QueryMatrixTests(unittest.TestCase):
    def fixture(self, directory: Path) -> tuple[Path, Path, Path]:
        sources = directory / "sources"
        sources.mkdir()
        conllu = sources / "sample.conllu"
        conllu.write_text(CONLLU, encoding="utf-8")
        digest = hashlib.sha256(conllu.read_bytes()).hexdigest()
        manifest = {
            "schema_version": 4,
            "ud_release": "test",
            "seed": "query-matrix-test",
            "source_sets": {
                "canonical": {
                    "sources": ["sample"],
                    "positive_quotas_per_source": {"noun": 1},
                    "scoring_status": "scored",
                }
            },
            "sources": [
                {
                    "name": "sample",
                    "description": "Sample",
                    "splits": {
                        "test": {
                            "data_file": "sample.conllu",
                            "data_url": "https://example.com/sample.conllu",
                            "data_sha256": digest,
                        }
                    },
                    "license": "CC0",
                    "license_file": "LICENSE",
                }
            ],
        }
        manifest_path = directory / "sources.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        sentences, _ = parse_conllu("sample", conllu)
        noun = next(
            candidate
            for candidate in sentences[0].candidates
            if candidate.query == "새" and candidate.pos == "noun"
        )
        canonical_path = directory / "canonical.jsonl"
        canonical_path.write_text(
            json.dumps(positive_case(noun).__dict__, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        return manifest_path, sources, canonical_path

    def build(self, directory: Path, query_mode: str):
        manifest, sources, canonical = self.fixture(directory)
        output = directory / f"{query_mode}.jsonl"
        metadata_path = directory / f"{query_mode}.json"
        contract_reviews_path = directory / "contract-reviews.tsv"
        contract_reviews_path.write_text(
            "query_mode\tsplit\tcase_id\tquery\tpos\tstrict_expected\t"
            "text_sha256\tcontract_status\tcontract_reason\tnote\n",
            encoding="utf-8",
        )
        metadata = build_query_matrix(
            manifest_path=manifest,
            sources_dir=sources,
            canonical_cases_path=canonical,
            output=output,
            metadata_path=metadata_path,
            contract_reviews_path=contract_reviews_path,
            split_name="test",
            query_mode=query_mode,
        )
        cases = load_cases(output)
        validate_query_matrix_dataset(output, cases, metadata, query_mode)
        return cases, metadata

    def test_expands_one_sentence_to_balanced_pos_diverse_queries(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cases, metadata = self.build(Path(directory), "explicit-pos")

        positives = [case for case in cases if case["expected"]]
        negatives = [case for case in cases if not case["expected"]]
        self.assertEqual(len(positives), 3)
        self.assertEqual(len(negatives), 3)
        self.assertEqual({case["pos"] for case in positives}, {"determiner", "noun", "verb"})
        self.assertEqual(
            sorted(case["pos"] for case in positives),
            sorted(case["pos"] for case in negatives),
        )
        self.assertEqual(metadata["canonical_positive_coverage"], 1)
        self.assertEqual(metadata["present_queries_per_sentence"], {"3": 1})
        self.assertEqual(metadata["contract_review"]["reviewed_cases"], 0)
        selected_pairs = {(case["query"], case["pos"]) for case in positives}
        self.assertTrue(
            selected_pairs.isdisjoint(
                {(case["query"], case["pos"]) for case in negatives}
            )
        )

    def test_untagged_negatives_exclude_every_present_lemma(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cases, _ = self.build(Path(directory), "untagged")

        present_queries = {str(case["query"]) for case in cases if case["expected"]}
        absent_queries = {str(case["query"]) for case in cases if not case["expected"]}
        self.assertTrue(present_queries.isdisjoint(absent_queries))
        self.assertTrue(
            all(str(case["id"]).startswith("untagged:matrix:") for case in cases)
        )

    def test_sentence_metrics_preserve_clustered_query_outcomes(self) -> None:
        cases = [
            {"id": "a-1", "expected": True, "matrix_group_id": "a"},
            {"id": "a-2", "expected": True, "matrix_group_id": "a"},
            {"id": "b-1", "expected": True, "matrix_group_id": "b"},
            {
                "id": "b-n",
                "expected": False,
                "contract_expected": True,
                "contract_reason": "same-pos-homograph",
                "matrix_group_id": "b",
            },
        ]
        predictions = {"a-1": True, "a-2": False, "b-1": True, "b-n": False}
        strict = query_matrix_metrics(
            cases,
            predictions,
            "fixed-seed",
        )
        contract_adjusted = query_matrix_metrics(
            cases,
            predictions,
            "fixed-seed",
            contract_adjusted=True,
        )

        self.assertEqual(strict["sentences"], 2)
        self.assertEqual(strict["all_present_queries_recovered"], 1)
        self.assertEqual(strict["all_present_queries_recovered_percent"], 50.0)
        self.assertEqual(
            strict["recovered_query_distribution"], {"1/1": 1, "1/2": 1}
        )
        self.assertEqual(strict["bootstrap_resamples"], 10_000)
        self.assertEqual(contract_adjusted["sentences"], 2)
        self.assertEqual(contract_adjusted["all_present_queries_recovered"], 0)
        self.assertEqual(
            contract_adjusted["recovered_query_distribution"], {"1/2": 2}
        )


if __name__ == "__main__":
    unittest.main()
