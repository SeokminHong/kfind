from __future__ import annotations

import tempfile
import unittest
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path
from unittest.mock import patch

from nikl_import import (
    KrDictEntry,
    import_snapshot,
    krdict_record,
    krdict_related_adverbs,
    normalize_headword,
    opendict_record,
    stdict_record,
)
from nikl_endings import ending_categories, krdict_endings, normalize_ending, stdict_endings
from nikl_particles import (
    generated_surfaces,
    krdict_particles,
    normalize_particle,
    opendict_particles,
    stdict_particles,
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

    def test_accepts_only_bidirectional_predicate_adverb_relations(self) -> None:
        entries = {
            "1": KrDictEntry(
                "상관없다",
                "형용사",
                (("파생어", "2", "상관없이"), ("파생어", "3", "일방향으로")),
            ),
            "2": KrDictEntry(
                "상관없이",
                "부사",
                (("☞(가 보라)", "1", "상관없다"),),
            ),
            "3": KrDictEntry("일방향으로", "부사", ()),
        }

        self.assertEqual(krdict_related_adverbs(entries), {"1": ("상관없이",)})

    def test_extracts_and_classifies_ending_headwords(self) -> None:
        krdict = ET.fromstring(
            """<LexicalEntry val="1"><feat att="partOfSpeech" val="어미"/>
            <Lemma><feat att="writtenForm" val="-더니"/></Lemma>
            <Sense><feat att="definition" val="회상을 나타내는 연결 어미."/></Sense>
            </LexicalEntry>"""
        )
        stdict = ET.fromstring(
            """<item><target_code>2</target_code><word_info><word>-더니</word>
            <pos_info><pos_code>3</pos_code><pos>어미</pos><comm_pattern_info>
            <sense_info><definition>회상을 나타내는 연결 어미.</definition></sense_info>
            </comm_pattern_info></pos_info></word_info></item>"""
        )

        self.assertEqual(normalize_ending(" -으시- "), ("-으시-", "으시"))
        self.assertEqual(ending_categories(["명사형 전성 어미."]), ("nominalizer",))
        self.assertEqual(krdict_endings(krdict)[0].surface, "더니")
        self.assertEqual(stdict_endings(stdict)[0].source_id, "2:3")

    def test_extracts_particle_headwords_without_definition_text(self) -> None:
        krdict = ET.fromstring(
            """<LexicalEntry val="1"><feat att="partOfSpeech" val="조사"/>
            <Lemma><feat att="writtenForm" val="까지"/></Lemma></LexicalEntry>"""
        )
        stdict = ET.fromstring(
            """<item><target_code>2</target_code><word_info><word>도07</word>
            <pos_info><pos_code>3</pos_code><pos>조사</pos><comm_pattern_info>
            <sense_info><type>일반어</type><definition>복사하지 않을 내용</definition></sense_info>
            </comm_pattern_info></pos_info></word_info></item>"""
        )
        opendict = ET.fromstring(
            """<item><target_code>4</target_code><wordInfo><word>까지</word></wordInfo>
            <senseInfo><pos>조사</pos><type>일반어</type></senseInfo></item>"""
        )

        self.assertEqual(normalize_particle(" -까지01 "), ("-까지01", "까지"))
        self.assertEqual(krdict_particles(krdict)[0].surface, "까지")
        self.assertEqual(stdict_particles(stdict)[0].source_id, "2:3")
        self.assertEqual(stdict_particles(stdict)[0].statuses, ("일반어",))
        self.assertEqual(opendict_particles(opendict)[0].source_id, "4:1")

    def test_generates_particle_catalog_surfaces_from_bounded_transitions(self) -> None:
        generated = generated_surfaces(
            {
                "까지": ["particle.limit"],
                "도": ["particle.additive"],
                "만": ["particle.only"],
            },
            {
                "particle.limit": ["particle.additive", "particle.only"],
                "particle.additive": [],
                "particle.only": [],
            },
            max_rules=2,
        )

        self.assertIn("까지도", generated)
        self.assertIn("까지만", generated)
        self.assertNotIn("도까지", generated)

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
