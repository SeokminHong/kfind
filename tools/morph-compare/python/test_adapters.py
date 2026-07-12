import unittest

from adapters import CandidateSpan, candidate, candidate_prediction, lindera_candidates


class AdapterTests(unittest.TestCase):
    def test_predicate_stem_becomes_dictionary_form(self) -> None:
        self.assertEqual(
            candidate("걷", "VV", 3, 12),
            CandidateSpan("걷다", "verb", "VV", 3, 12),
        )

    def test_lindera_expression_uses_token_span(self) -> None:
        candidates = lindera_candidates(
            [
                {
                    "surface": "걸었다",
                    "byte_start": 4,
                    "byte_end": 13,
                    "details": [
                        "VV+EP+EF",
                        "*",
                        "*",
                        "*",
                        "Inflect",
                        "VV",
                        "EF",
                        "걷/VV/*+었/EP/*+다/EF/*",
                    ],
                }
            ]
        )
        self.assertIn(CandidateSpan("걷다", "verb", "VV", 4, 13), candidates)

    def test_positive_prediction_requires_gold_span_overlap(self) -> None:
        candidates = {CandidateSpan("가다", "verb", "VV", 20, 29)}
        self.assertFalse(candidate_prediction("가다", "verb", True, 0, 9, candidates))
        self.assertTrue(candidate_prediction("가다", "verb", True, 20, 29, candidates))

    def test_negative_prediction_checks_the_whole_sentence(self) -> None:
        candidates = {CandidateSpan("가다", "verb", "VV", 20, 29)}
        self.assertTrue(candidate_prediction("가다", "verb", False, None, None, candidates))


if __name__ == "__main__":
    unittest.main()
