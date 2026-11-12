#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterator
from pathlib import Path
from xml.etree.ElementTree import Element

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import STDICT_SHA256, normalize_headword  # noqa: E402


# 표준국어대사전 접미사 뜻풀이 번호와 같은 뜻의 독립 명사 표제어 번호
NOUNS_BY_SUFFIX_SENSE = {
    "104787": "47678",
    "1358873": "46361",
    "1395994": "116095",
    "182141": "164762",
    "193793": "184902",
    "25442": "490427",
    "340337": "20859",
    "342337": "49642",
    "357040": "246486",
    "47186": "208368",
    "524539": "399536",
    "584424": "334741",
    "598286": "437671",
    "599034": "498206",
    "700730": "60397",
}


def catalog_rows(item: Element) -> Iterator[tuple[str, str, str, str, str]]:
    word_info = item.find("./word_info")
    if word_info is None:
        return
    raw = word_info.findtext("./word", "")
    suffix, _ = normalize_headword(raw)
    if not raw.startswith("-") or len(suffix) != 1:
        return
    suffix_id = item.findtext("./target_code", "")
    for pos in word_info.findall("./pos_info"):
        if pos.findtext("./pos") != "접사":
            continue
        for sense in pos.findall(".//sense_info"):
            sense_id = sense.findtext("./sense_code", "")
            noun_id = NOUNS_BY_SUFFIX_SENSE.get(sense_id)
            if noun_id is None:
                continue
            for example in sense.findall(".//example"):
                word = (example.text or "").strip().removesuffix(".")
                if re.fullmatch("[가-힣]{2,}", word) and word.endswith(suffix):
                    yield word, suffix, suffix_id, sense_id, noun_id


def main() -> None:
    parser = argparse.ArgumentParser(
        description="표준국어대사전 접미사 용례의 명사 성분 목록을 출력합니다."
    )
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    args = parser.parse_args()
    rows = set(
        load_catalog_records(
            "stdict", args.stdict, STDICT_SHA256, "item", catalog_rows, args.cache_dir
        )
    )
    if {row[3] for row in rows} != NOUNS_BY_SUFFIX_SENSE.keys():
        raise ValueError("등록한 접미사 뜻풀이에 대응하는 단어 용례가 빠졌습니다.")
    print("// 접미사 표제어:뜻풀이, 독립 명사 표제어")
    print("const STDICT_ATTACHED_NOMINAL_USAGES: &[(&str, &str)] = &[")
    for suffix, word, suffix_id, sense_id, noun_id in sorted(
        (suffix, word, suffix_id, sense_id, noun_id)
        for word, suffix, suffix_id, sense_id, noun_id in rows
    ):
        print(f'    ("{suffix}", "{word}"), // {suffix_id}:{sense_id}, {noun_id}')
    print("];")


if __name__ == "__main__":
    main()
