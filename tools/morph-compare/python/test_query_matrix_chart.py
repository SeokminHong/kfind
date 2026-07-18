from __future__ import annotations

import unittest

from query_matrix_chart import render_query_matrix_quality


class QueryMatrixChartTests(unittest.TestCase):
    def test_renders_raw_and_contract_counts(self) -> None:
        raw = {
            "precision_percent": 99.0,
            "recall_percent": 90.0,
            "tp": 90,
            "fp": 1,
            "tn": 99,
            "fn": 10,
        }
        contract = {
            "cases": 180,
            "contract_precision_percent": 100.0,
            "contract_recall_percent": 100.0,
            "contract_tp": 90,
            "contract_fp": 0,
            "contract_tn": 90,
            "contract_fn": 0,
            "reviewed_cases": 20,
            "excluded_cases": 10,
        }
        report = {
            "query_matrix": {
                "explicit_pos": {
                    "dataset": {
                        "cases": 200,
                        "contract_review": {
                            "reclassified_cases": 10,
                            "excluded_cases": 10,
                        },
                    },
                    "quality": {
                        profile: {
                            "overall": raw,
                            "contract_adjusted": {"overall": contract},
                        }
                        for profile in ("kfind-embedded", "kfind-full-pos")
                    },
                }
            }
        }

        rendered = render_query_matrix_quality(report)

        self.assertIn("Raw   TP 90 · FP 1 · TN 99 · FN 10", rendered)
        self.assertIn("Contract   TPᶜ 90 · FPᶜ 0 · TNᶜ 90 · FNᶜ 0", rendered)
        self.assertIn("they are not aliases of raw values", rendered)


if __name__ == "__main__":
    unittest.main()
