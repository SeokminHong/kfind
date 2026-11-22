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


def catalog_records(item: Element) -> Iterator[tuple[str, str, bool]]:
    word_info = item.find("word_info")
    if word_info is None:
        return
    raw = word_info.findtext("word", "")
    if not raw.startswith("대") or " " in raw:
        return
    word, homonym = normalize_headword(raw)
    if not re.fullmatch("[가-힣]{2,}", word):
        return
    # 붙임표는 직접 구성 성분을 표시한다. 동형 전문 용어의 붙여쓰기도 대조한다.
    prefix_boundary = raw.removesuffix(homonym) == f"대-{word[1:]}"
    nominal = [entry.text for entry in word_info.findall("pos_info/pos")] == ["명사"]
    original = word_info.findtext("original_language_info/original_language", "")
    yield (
        word,
        item.findtext("target_code", ""),
        prefix_boundary and nominal and original.startswith(("大", "對")),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="표준국어대사전의 대- 결합 명사를 추출합니다.")
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    args = parser.parse_args()
    records = load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item", catalog_records, args.cache_dir
    )
    source_ids: dict[str, list[str]] = {}
    competing_words: set[str] = set()
    for word, source_id, supports_prefix in records:
        if supports_prefix:
            if not source_id.isascii() or not source_id.isdecimal():
                raise ValueError(f"{word}: 사전 표제어 번호가 필요합니다")
            source_ids.setdefault(word, []).append(source_id)
        else:
            competing_words.add(word)
    for word in sorted(source_ids.keys() - competing_words):
        print(word, ",".join(sorted(source_ids[word])), sep="\t")


if __name__ == "__main__":
    main()
