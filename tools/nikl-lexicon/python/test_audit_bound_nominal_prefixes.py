import sys
import unittest
from pathlib import Path
from xml.etree.ElementTree import fromstring

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from audit_bound_nominal_prefixes import catalog_records, prefix_candidates


def source_record(word, pos, origin, entry_id, sense_id, definition=""):
    item = fromstring(f"""<item>
      <target_code>{entry_id}</target_code>
      <word_info>
        <word>{word}</word>
        <original_language_info><original_language>{origin}</original_language></original_language_info>
        <pos_info><pos>{pos}</pos><comm_pattern_info><sense_info>
          <sense_code>{sense_id}</sense_code>
          <definition_original><![CDATA[{definition}]]></definition_original>
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
