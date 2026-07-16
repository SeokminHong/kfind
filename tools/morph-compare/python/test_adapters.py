import unittest

from adapters import (
    CandidateSpan,
    candidate,
    candidate_prediction,
    komoran_candidates,
    lindera_candidates,
    mecab_candidates,
)


class AdapterTests(unittest.TestCase):
    def test_predicate_stem_becomes_dictionary_form(self) -> None:
        self.assertEqual(
            candidate("걷", "VV", 3, 12),
            CandidateSpan("걷다", "verb", "VV", 3, 12),
        )

    def test_non_searchable_span_is_excluded(self) -> None:
        self.assertIsNone(candidate("이", "VCP", 3, 3))
        self.assertIsNone(candidate("이", "VCP", -1, 2))

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

    def test_mecab_expression_uses_inflected_surface_span(self) -> None:
        candidates = mecab_candidates(
            "그는 걸었다.",
            "그\tNP,*,F,그,*,*,*,*\n"
            "는\tJX,*,T,는,*,*,*,*\n"
            "걸었다\tVV+EP+EF,*,T,걸었다,Inflect,VV,EF,걷/VV/*+었/EP/*+다/EF/*\n"
            ".\tSF,*,*,*,*,*,*,*\nEOS\n",
        )

        self.assertIn(CandidateSpan("걷다", "verb", "VV", 7, 16), candidates)

    def test_komoran_offsets_are_converted_to_utf8_bytes(self) -> None:
        candidates = komoran_candidates(
            "그는 걸었다.",
            [{"morph": "걷", "pos": "VV", "begin": 3, "end": 5}],
        )

        self.assertIn(CandidateSpan("걷다", "verb", "VV", 7, 13), candidates)


if __name__ == "__main__":
    unittest.main()
