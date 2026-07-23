import json
import tempfile
import unittest
from pathlib import Path

from external_baselines import EXTERNAL_BACKENDS, load_external_baselines


class ExternalBaselineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cases = [
            {
                "id": "positive",
                "expected": True,
                "gold_byte_start": 3,
                "gold_byte_end": 9,
            },
            {
                "id": "negative",
                "expected": False,
                "gold_byte_start": None,
                "gold_byte_end": None,
            },
        ]
        result = [
            {
                "id": "positive",
                "matching_spans": [{"byte_start": 3, "byte_end": 6}],
            },
            {"id": "negative", "matching_spans": []},
        ]
        self.snapshot = {
            "schema_version": 2,
            "fixture_sha256": "fixture",
            "case_count": 2,
            "environment": {"platform": "test"},
            "backends": {
                backend: {
                    "status": "available",
                    "version": "1.0",
                    "configuration": {},
                    "results": result,
                    "performance": self.performance(),
                }
                for backend in EXTERNAL_BACKENDS
            },
        }

    @staticmethod
    def performance() -> dict[str, object]:
        metrics = {
            "initialization_seconds": 0.1,
            "evaluation_seconds": 0.2,
            "cases_per_second": 10.0,
            "latency_p50_ms": 1.0,
            "latency_p95_ms": 2.0,
            "peak_rss_kib": 1024,
        }
        return {
            "runs": 5,
            "warmup_runs": 1,
            **metrics,
            "run_min": metrics,
            "run_max": metrics,
        }

    def load(
        self, metadata: dict[str, object] | None = None
    ) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "baselines.json"
            path.write_text(json.dumps(self.snapshot), encoding="utf-8")
            return load_external_baselines(
                path,
                self.cases,
                metadata or {"fixture_sha256": "fixture"},
            )

    def test_loads_predictions_from_matching_spans(self) -> None:
        baselines = self.load()

        self.assertTrue(baselines["predictions"]["kiwi"]["positive"])
        self.assertFalse(baselines["predictions"]["kiwi"]["negative"])

    def test_keeps_unavailable_backend_out_of_results(self) -> None:
        self.snapshot["backends"]["komoran"] = {
            "status": "unavailable",
            "reason": "snapshot has not been captured",
        }

        baselines = self.load()

        self.assertNotIn("komoran", baselines["predictions"])
        self.assertEqual(
            "unavailable", baselines["availability"]["komoran"]["status"]
        )

    def test_rejects_stale_fixture(self) -> None:
        self.snapshot["fixture_sha256"] = "stale"

        with self.assertRaisesRegex(ValueError, "refresh-morph-baselines"):
            self.load()

    def test_uses_pre_contract_fixture_identity_for_external_results(self) -> None:
        baselines = self.load(
            {
                "fixture_sha256": "contract-adjusted-fixture",
                "external_baseline_fixture_sha256": "fixture",
            }
        )

        self.assertTrue(baselines["predictions"]["kiwi"]["positive"])

    def test_rejects_missing_performance(self) -> None:
        del self.snapshot["backends"]["kiwi"]["performance"]

        with self.assertRaisesRegex(ValueError, "kiwi has no performance"):
            self.load()


if __name__ == "__main__":
    unittest.main()
