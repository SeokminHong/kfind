from __future__ import annotations

import unittest

from site_snapshot import profile_comparison_summary


def metrics(value: int) -> dict[str, int]:
    return {
        "cases": 2,
        "tp": value,
        "fp": 0,
        "tn": 1,
        "fn": 0,
    }


def contract_metrics(value: int) -> dict[str, int]:
    return {
        "cases": 2,
        "contract_tp": value,
        "contract_fp": 0,
        "contract_tn": 1,
        "contract_fn": 0,
    }


class SiteSnapshotTests(unittest.TestCase):
    def test_lists_four_kfind_profiles_before_external_backends(self) -> None:
        boundary_profiles = {}
        value = 1
        for resource_profile in ("embedded", "full-pos"):
            boundary_profiles[resource_profile] = {}
            for boundary in ("any", "smart"):
                boundary_profiles[resource_profile][boundary] = {
                    "quality": metrics(value),
                    "contract_adjusted_quality": contract_metrics(value),
                    "performance": {"cases_per_second": value},
                }
                value += 1
        evaluation = {
            "backends": ["kfind-embedded", "kfind-full-pos", "kiwi"],
            "boundary_comparison": {"profiles": boundary_profiles},
            "quality": {
                "kiwi": {
                    "overall": metrics(5),
                    "contract_adjusted": {
                        "overall": contract_metrics(5)
                    },
                }
            },
        }

        summary = profile_comparison_summary(
            evaluation, {"kiwi": {"cases_per_second": 5}}
        )

        self.assertEqual(
            [
                "kfind-embedded-any",
                "kfind-embedded-smart",
                "kfind-full-pos-any",
                "kfind-full-pos-smart",
                "kiwi",
            ],
            summary["profiles"],
        )
        self.assertEqual(
            3,
            summary["quality"]["kfind-full-pos-any"]["overall"]["tp"],
        )
        self.assertEqual(
            4,
            summary["quality"]["kfind-full-pos-smart"][
                "contract_adjusted"
            ]["overall"]["contract_tp"],
        )
        self.assertEqual(
            5, summary["performance"]["kiwi"]["cases_per_second"]
        )


if __name__ == "__main__":
    unittest.main()
