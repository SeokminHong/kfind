from __future__ import annotations

import json
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

from nikl_lexemes import krdict_lexemes, opendict_lexemes, stdict_lexemes, write_report
from nikl_nominal_suffixes import attached_nominal_suffixes, validate_catalog, write_catalog


class NiklLexemeTest(unittest.TestCase):
    def test_krdict_preserves_structured_related_form(self) -> None:
        entry = ET.fromstring(
            """
            <LexicalEntry val="62078">
              <feat att="partOfSpeech" val="대명사" />
              <Lemma><feat att="writtenForm" val="누구" /></Lemma>
              <RelatedForm>
                <feat att="type" val="파생어" />
                <feat att="id" val="7" />
                <feat att="writtenForm" val="누군가" />
              </RelatedForm>
            </LexicalEntry>
            """
        )

        records = krdict_lexemes(entry, frozenset({"누구"}))

        self.assertEqual(records[0].lemma, "누구")
        self.assertEqual(records[0].pos, "대명사")
        self.assertEqual(records[0].relations[0].relation_type, "파생어")
        self.assertEqual(records[0].relations[0].target_headword, "누군가")

    def test_stdict_normalizes_homonym_and_keeps_lexical_relations(self) -> None:
        item = ET.fromstring(
            """
            <item>
              <target_code>424010</target_code>
              <word_info>
                <word>무어01</word>
                <lexical_info>
                  <word>뭐</word><unit>어휘</unit><type>준말</type>
                  <link_target_code>428865</link_target_code>
                </lexical_info>
                <pos_info>
                  <pos_code>424010001</pos_code><pos>대명사</pos>
                  <sense_info>
                    <type>일반어</type>
                    <lexical_info>
                      <word>무엇</word><unit>의미</unit><type>동의어</type>
                      <link_target_code>5656</link_target_code>
                    </lexical_info>
                  </sense_info>
                </pos_info>
              </word_info>
            </item>
            """
        )

        records = stdict_lexemes(item, frozenset({"무어"}))

        self.assertEqual(records[0].lemma, "무어")
        self.assertEqual(
            [(relation.relation_type, relation.target_headword) for relation in records[0].relations],
            [("동의어", "무엇"), ("준말", "뭐")],
        )

    def test_opendict_keeps_relation_info_separate(self) -> None:
        item = ET.fromstring(
            """
            <item>
              <target_code>3</target_code>
              <wordInfo><word>후</word></wordInfo>
              <relation_info>
                <word>이후</word><type>관련어</type><link_target_code>4</link_target_code>
              </relation_info>
              <senseInfo><pos>명사</pos><type>일반어</type></senseInfo>
            </item>
            """
        )

        records = opendict_lexemes(item, frozenset({"후"}))

        self.assertEqual(records[0].relations[0].unit, "관계")
        self.assertEqual(records[0].relations[0].target_headword, "이후")

    def test_report_lists_exact_missing_headwords_by_source(self) -> None:
        entry = ET.fromstring(
            """
            <LexicalEntry val="1">
              <feat att="partOfSpeech" val="명사" />
              <Lemma><feat att="writtenForm" val="후" /></Lemma>
            </LexicalEntry>
            """
        )
        records = krdict_lexemes(entry, frozenset({"후"}))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report.json"
            write_report(
                output,
                ["후", "누구"],
                records,
                {"krdict": "a", "stdict": "b"},
            )
            report = json.loads(output.read_text(encoding="utf-8"))

        self.assertEqual(report["missing_by_source"]["krdict"], ["누구"])
        self.assertEqual(report["missing_by_source"]["stdict"], ["누구", "후"])

    def test_attached_nominal_suffix_catalog_keeps_reviewed_modern_suffixes(self) -> None:
        requested = frozenset({"하"})
        krdict = ET.fromstring(
            """
            <LexicalEntry val="88469">
              <feat att="partOfSpeech" val="접사" />
              <Lemma><feat att="writtenForm" val="-하" /></Lemma>
            </LexicalEntry>
            """
        )
        stdict = ET.fromstring(
            """
            <item><target_code>362661</target_code><word_info><word>-하12</word>
              <pos_info><pos_code>362661001</pos_code><pos>접사</pos>
                <sense_info><type>일반어</type></sense_info>
              </pos_info>
            </word_info></item>
            """
        )
        records = (*krdict_lexemes(krdict, requested), *stdict_lexemes(stdict, requested))

        suffixes = attached_nominal_suffixes(records, requested)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "suffixes.tsv"
            write_catalog(output, requested, suffixes)
            validate_catalog(output, requested)
            lines = output.read_text(encoding="utf-8").splitlines()

        self.assertEqual(
            lines,
            [
                "surface\theadwords\tkrdict_ids\tstdict_ids\topendict_ids",
                "하\t-하|-하12\t88469\t362661:362661001\t-",
            ],
        )

    def test_attached_nominal_suffix_validation_is_separate_from_generation(self) -> None:
        requested = frozenset({"하"})
        krdict = ET.fromstring(
            """
            <LexicalEntry val="88469">
              <feat att="partOfSpeech" val="접사" />
              <Lemma><feat att="writtenForm" val="-하" /></Lemma>
            </LexicalEntry>
            """
        )
        records = krdict_lexemes(krdict, requested)

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "suffixes.tsv"
            write_catalog(output, requested, records)
            with self.assertRaisesRegex(ValueError, "missing modern suffix evidence from stdict"):
                validate_catalog(output, requested)


if __name__ == "__main__":
    unittest.main()
