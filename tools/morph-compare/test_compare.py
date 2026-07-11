import unittest

from compare import Candidate, candidate, candidate_matches, lindera_candidates


class NormalizationTests(unittest.TestCase):
    def test_predicate_stems_become_dictionary_forms(self) -> None:
        self.assertEqual(candidate("걷", "VV"), Candidate("걷다", "verb", "VV"))
        self.assertEqual(candidate("아니", "VCN"), Candidate("아니다", "adjective", "VCN"))

    def test_lindera_expression_exposes_inflected_stem(self) -> None:
        candidates = lindera_candidates(
            [
                {
                    "surface": "걸었다",
                    "part_of_speech_tag": "VV+EP+EF",
                    "expression": "걷/VV/*+었/EP/*+다/EF/*",
                }
            ]
        )
        self.assertTrue(candidate_matches("걷다", "verb", candidates))

    def test_productive_hada_is_reconstructed(self) -> None:
        candidates = lindera_candidates(
            [
                {
                    "surface": "검증했다",
                    "part_of_speech_tag": "NNG+XSV+EP+EF",
                    "expression": "검증/NNG/*+하/XSV/*+었/EP/*+다/EF/*",
                }
            ]
        )
        self.assertTrue(candidate_matches("검증하다", "verb", candidates))


if __name__ == "__main__":
    unittest.main()
