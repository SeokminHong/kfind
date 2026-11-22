import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from test_audit_bound_nominal_prefixes import source_record

from audit_bound_nominal_suffixes import aligned_mecab_nouns, noun_suffix_candidates


class NominalSuffixCatalogTests(unittest.TestCase):
    def test_attested_suffix_preserves_other_origins_and_unaligned_analyses(self):
        records = [
            source_record("-자16", "접사", "者", 281809, 1),
            source_record("자14", "의존 명사", "者", 281505, 2),
            source_record("자05", "명사", "子", 281206, 3),
            source_record("근로-자", "명사", "勤勞者", 10, 4),
            source_record("책임-자", "명사", "責任者", 11, 5),
            source_record("배우-자01", "명사", "配偶子", 140388, 6),
            source_record("배우-자02", "명사", "配偶者", 140389, 7),
        ]
        suffix, candidates = noun_suffix_candidates(records, "281809")
        self.assertEqual((suffix, candidates), ("자", {"근로자": {"10"}, "책임자": {"11"}}))

        with TemporaryDirectory() as directory:
            path = Path(directory) / "NNG.csv"
            path.write_text(
                "근로자,0,0,0,NNG,*,F,근로자,Compound,*,*,근로/NNG/*+자/NNG/*\n"
                "책임자,0,0,0,NNG,*,F,책임자,Compound,*,*,책/NNG/*+자/NNG/*\n"
                "배우자,0,0,0,NNG,*,F,배우자,Compound,*,*,배우/NNG/*+자/NNG/*\n",
                encoding="utf-8",
            )
            self.assertEqual(aligned_mecab_nouns(path, suffix, candidates), {"근로자"})

    def test_rejects_suffix_with_an_independent_noun_of_the_same_origin(self):
        records = [
            source_record("-자16", "접사", "者", 281809, 1),
            source_record("자14", "명사", "者", 281505, 2),
        ]
        with self.assertRaisesRegex(ValueError, "의존 명사"):
            noun_suffix_candidates(records, "281809")
