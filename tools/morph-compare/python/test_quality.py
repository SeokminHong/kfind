import unittest

from quality import contract_expected, contract_quality_metrics, quality_metrics


class ContractQualityTests(unittest.TestCase):
    def test_preserves_strict_fp_while_reclassifying_contract_positive(self) -> None:
        cases = [
            {"id": "strict-positive", "expected": True},
            {
                "id": "intended-match",
                "expected": False,
                "contract_expected": True,
                "contract_reason": "same-pos-homograph",
            },
            {"id": "actual-fp", "expected": False},
            {"id": "strict-negative", "expected": False},
        ]
        predictions = {
            "strict-positive": True,
            "intended-match": True,
            "actual-fp": True,
            "strict-negative": False,
        }

        strict = quality_metrics(cases, predictions)
        adjusted = contract_quality_metrics(cases, predictions)

        self.assertEqual((1, 2, 1, 0), _strict_counts(strict))
        self.assertEqual((2, 1, 1, 0), _contract_counts(adjusted))
        self.assertEqual(1, adjusted["reclassified_cases"])
        self.assertEqual(
            {"same-pos-homograph": 1}, adjusted["reclassified_by_reason"]
        )

    def test_contract_positive_miss_is_contract_fn_not_strict_fn(self) -> None:
        case = {
            "id": "ambiguous-miss",
            "expected": False,
            "contract_expected": True,
            "contract_reason": "same-pos-homograph",
        }

        strict = quality_metrics([case], {"ambiguous-miss": False})
        adjusted = contract_quality_metrics([case], {"ambiguous-miss": False})

        self.assertEqual((0, 0, 1, 0), _strict_counts(strict))
        self.assertEqual((0, 0, 0, 1), _contract_counts(adjusted))

    def test_rejects_annotation_without_a_review_reason(self) -> None:
        case = {"id": "unreviewed", "expected": False, "contract_expected": True}

        with self.assertRaisesRegex(ValueError, "unsupported contract_reason"):
            contract_expected(case)


def _strict_counts(metrics: dict[str, object]) -> tuple[object, ...]:
    return tuple(metrics[key] for key in ("tp", "fp", "tn", "fn"))


def _contract_counts(metrics: dict[str, object]) -> tuple[object, ...]:
    return tuple(
        metrics[key]
        for key in ("contract_tp", "contract_fp", "contract_tn", "contract_fn")
    )


if __name__ == "__main__":
    unittest.main()
