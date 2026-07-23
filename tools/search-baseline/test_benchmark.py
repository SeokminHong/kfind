from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


DIRECTORY = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "search_baseline_benchmark",
    DIRECTORY / "benchmark.py",
)
assert SPEC is not None and SPEC.loader is not None
benchmark = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(benchmark)


class SearchBaselineBenchmarkTest(unittest.TestCase):
    def setUp(self) -> None:
        self.queries = benchmark.load_patterns(DIRECTORY / "patterns.json")
        self.cases = benchmark.load_fixture(
            DIRECTORY / "fixture.jsonl",
            self.queries,
        )

    def test_fixture_contract_is_balanced_and_complete(self) -> None:
        self.assertEqual(len(self.queries), 7)
        self.assertEqual(len(self.cases), 112)
        self.assertEqual(sum(case["expected"] for case in self.cases), 56)
        self.assertEqual(
            sum(case["contract_expected"] for case in self.cases),
            62,
        )
        self.assertEqual(
            sum(
                case["expected"] != case["contract_expected"]
                for case in self.cases
            ),
            6,
        )

    def test_metrics_preserve_tp_tn_fp_fn_order(self) -> None:
        predictions = {
            int(case["line"]) for case in self.cases if case["expected"]
        }
        raw = benchmark.metrics(self.cases, predictions, False)
        contract = benchmark.metrics(self.cases, predictions, True)

        self.assertEqual(
            {key: raw[key] for key in ("tp", "tn", "fp", "fn")},
            {"tp": 56, "tn": 56, "fp": 0, "fn": 0},
        )
        self.assertEqual(
            {key: contract[key] for key in ("tp", "tn", "fp", "fn")},
            {"tp": 56, "tn": 50, "fp": 0, "fn": 6},
        )

    def test_commands_keep_quality_and_tool_variants_distinct(self) -> None:
        query = self.queries[0]
        corpus = Path("/tmp/corpus.txt")
        kfind = Path("/tmp/kfind")
        data_dir = Path("/tmp/data")

        kfind_command = benchmark.command_for(
            "kfind_any",
            query,
            corpus,
            kfind,
            data_dir,
            "rg",
            "grep",
            True,
        )
        rg_command = benchmark.command_for(
            "rg_enumerated",
            query,
            corpus,
            kfind,
            data_dir,
            "rg",
            "grep",
            True,
        )

        self.assertIn("v:걷다", kfind_command)
        self.assertIn(str(data_dir), kfind_command)
        self.assertEqual(
            "any", kfind_command[kfind_command.index("--boundary") + 1]
        )
        self.assertIn(query["enumerated"], rg_command)
        self.assertNotIn(query["stem"], rg_command)


if __name__ == "__main__":
    unittest.main()
