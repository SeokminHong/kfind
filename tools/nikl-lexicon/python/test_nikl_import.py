from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

from nikl_import import (
    import_snapshot,
    krdict_record,
    normalize_headword,
    opendict_record,
    stdict_record,
)


class NiklImportTest(unittest.TestCase):
    def test_normalizes_hyphens_and_preserves_homonym_suffix(self) -> None:
        self.assertEqual(normalize_headword("다르다01"), ("다르다", "01"))
        self.assertEqual(normalize_headword("푸르-다"), ("푸르다", ""))

    def test_imports_krdict_conjugations_by_source_record(self) -> None:
        source = b"""<?xml version="1.0" encoding="UTF-8"?>
<LexicalResource><Lexicon><LexicalEntry val="26824">
  <feat att="homonym_number" val="0"/>
  <feat att="partOfSpeech" val="\xed\x98\x95\xec\x9a\xa9\xec\x82\xac"/>
  <Lemma><feat att="writtenForm" val="\xeb\x8b\xa4\xeb\xa5\xb4\xeb\x8b\xa4"/></Lemma>
  <WordForm><feat att="type" val="\xed\x99\x9c\xec\x9a\xa9"/><feat att="writtenForm" val="\xeb\x8b\xac\xeb\x9d\xbc"/></WordForm>
</LexicalEntry></Lexicon></LexicalResource>"""
        records, stats = self.import_fixture("krdict", "LexicalEntry", krdict_record, source)

        self.assertEqual(len(records), 1)
        self.assertEqual(records[0].source_id, "26824")
        self.assertEqual(records[0].lemma, "다르다")
        self.assertEqual(records[0].pos, "VA")
        self.assertEqual(records[0].conjugations, ("달라",))
        self.assertEqual(stats.item_count, 1)

    def test_imports_stdict_pos_and_marks_redirects(self) -> None:
        source = """<?xml version="1.0" encoding="UTF-8"?>
<channel><item><target_code>72197</target_code><word_info>
  <word>다르다02</word>
  <pos_info><pos_code>72197001</pos_code><pos>동사</pos>
    <comm_pattern_info><sense_info><type>일반어</type><definition>→ 다루다.</definition></sense_info></comm_pattern_info>
  </pos_info>
</word_info></item></channel>""".encode()
        records, _ = self.import_fixture("stdict", "item", stdict_record, source)

        self.assertEqual(len(records), 1)
        self.assertEqual(records[0].source_id, "72197:72197001")
        self.assertEqual(records[0].raw_homonym, "02")
        self.assertEqual(records[0].lexical_status, "redirect")

    def test_imports_opendict_status_without_promoting_it(self) -> None:
        source = """<?xml version="1.0" encoding="UTF-8"?>
<channel><item><target_code>1</target_code>
  <wordInfo><word>푸르다001</word><conju_info><conjugation_info><conjugation>푸르러</conjugation></conjugation_info></conju_info></wordInfo>
  <senseInfo><pos>형용사</pos><type>방언</type><definition>합성 fixture</definition></senseInfo>
</item></channel>""".encode()
        records, _ = self.import_fixture("opendict", "item", opendict_record, source)

        self.assertEqual(len(records), 1)
        self.assertEqual(records[0].lemma, "푸르다")
        self.assertEqual(records[0].lexical_status, "방언")
        self.assertEqual(records[0].conjugations, ("푸르러",))

    def test_rejects_unexpected_invalid_xml_byte_count(self) -> None:
        source = b"<LexicalResource>\x08</LexicalResource>"
        with self.assertRaisesRegex(ValueError, "expected 0 invalid XML bytes, found 1"):
            self.import_fixture("krdict", "LexicalEntry", krdict_record, source)

    def test_records_allowlisted_invalid_xml_byte_location(self) -> None:
        source = b"<LexicalResource>\x08</LexicalResource>"
        _, stats = self.import_fixture(
            "krdict",
            "LexicalEntry",
            krdict_record,
            source,
            expected_invalid_bytes=1,
        )

        self.assertEqual(stats.sanitized_byte_count, 1)
        self.assertEqual(stats.sanitized_locations, ("fixture.xml:17",))

    def test_reuses_sha_keyed_extracted_snapshot(self) -> None:
        source = b"<LexicalResource><Lexicon/></LexicalResource>"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "fixture.zip"
            cache = root / "cache"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("fixture.xml", source)
            first, _ = import_snapshot(
                "krdict",
                path,
                "LexicalEntry",
                krdict_record,
                cache_directory=cache,
            )
            with patch("nikl_import.extract_snapshot", side_effect=AssertionError("cache miss")):
                second, _ = import_snapshot(
                    "krdict",
                    path,
                    "LexicalEntry",
                    krdict_record,
                    cache_directory=cache,
                )

            self.assertEqual(first, second)
            self.assertEqual(len(list(cache.glob("krdict/*/.complete"))), 1)

    def import_fixture(
        self,
        source_name: str,
        tag: str,
        adapter,
        source: bytes,
        expected_invalid_bytes: int = 0,
    ):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.zip"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("fixture.xml", source)
            return import_snapshot(
                source_name,
                path,
                tag,
                adapter,
                expected_invalid_bytes,
            )


if __name__ == "__main__":
    unittest.main()
