import sys
import unittest
from pathlib import Path
from xml.etree.ElementTree import fromstring

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from audit_bound_nominal_prefixes import catalog_records, prefix_candidates


def source_record(word, pos, origin, entry_id, sense_id, definition="", examples=()):
    word_type = "한자어" if origin else "고유어"
    examples_xml = "".join(
        f"<example_info><example>{example}</example></example_info>" for example in examples
    )
    item = fromstring(f"""<item>
      <target_code>{entry_id}</target_code>
      <word_info>
        <word>{word}</word>
        <word_type>{word_type}</word_type>
        <original_language_info><original_language>{origin}</original_language></original_language_info>
        <pos_info><pos>{pos}</pos><comm_pattern_info><sense_info>
          <sense_code>{sense_id}</sense_code>
          <definition_original><![CDATA[{definition}]]></definition_original>
          {examples_xml}
        </sense_info></comm_pattern_info></pos_info>
      </word_info>
    </item>""")
    [record] = catalog_records(item)
    return record


class NominalPrefixCatalogTests(unittest.TestCase):
    def test_a_nominal_variant_follows_the_meaning_reference_and_origin(self):
        prefix = source_record("토-07", "접사", "土", 342755, 400813)
        saturday = source_record("토-요일", "명사", "土曜日", 349235, 364487)
        wall = source_record("토-담", "명사", "土", 496179, 1)
        records = [prefix, saturday, wall]

        for origin, target, expected in [
            ("土", 364487, {"토담"}),
            ("吐", 364487, {"토요일", "토담"}),
            ("土", 999999, {"토요일", "토담"}),
        ]:
            with self.subTest(origin=origin, target=target):
                abbreviation = source_record(
                    "토04", "명사", origin, 342753, 406762,
                    f"‘<sense_no>{target}</sense_no>토요일’을 이르는 말.",
                )
                self.assertEqual(set(prefix_candidates([*records, abbreviation])), expected)

    def test_native_prefix_requires_an_attested_word_without_a_homonym(self):
        prefix = source_record(
            "날-05", "접사", "", 59519, 1, examples=("날것.", "날짜.", "날짜를 적었다.")
        )
        raw_food = source_record("날-것", "명사", "", 59795, 2)
        novice = source_record("날-짜02", "명사", "", 60044, 3)
        date = source_record("날-짜01", "명사", "", 407607, 4)
        unlisted = source_record("날-고기", "명사", "", 404641, 5)

        self.assertEqual(
            prefix_candidates([prefix, raw_food, novice, date, unlisted]),
            {"날것": [("날", "59795"), ("날", "59519")]},
        )
