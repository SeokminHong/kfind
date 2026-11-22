#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import re
import sys
from collections.abc import Iterator
from pathlib import Path
from xml.etree.ElementTree import Element

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import STDICT_SHA256, normalize_headword  # noqa: E402


def catalog_records(item: Element) -> Iterator[tuple[str, str, tuple[str, ...], str, str]]:
    word_info = item.find("word_info")
    if word_info is None:
        return
    raw = word_info.findtext("word", "")
    if " " in raw:
        return
    word, homonym = normalize_headword(raw)
    if not re.fullmatch("[가-힣]+", word):
        return
    yield (
        raw.removesuffix(homonym),
        word,
        tuple(entry.text or "" for entry in word_info.findall("pos_info/pos")),
        word_info.findtext("original_language_info/original_language", ""),
        item.findtext("target_code", ""),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="표준국어대사전의 접두사 결합 명사를 추출합니다.")
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    parser.add_argument("--component", action="store_true", help="공통 component 입력 CSV 출력")
    args = parser.parse_args()
    records = load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item", catalog_records, args.cache_dir
    )
    prefix_origins: dict[str, set[str]] = {}
    for written, word, tags, origin, _ in records:
        if written == f"{word}-" and tags == ("접사",) and origin:
            prefix_origins.setdefault(word, set()).add(origin)

    source_ids: dict[str, list[tuple[str, str]]] = {}
    competing_words: set[str] = set()
    for written, word, tags, origin, source_id in records:
        initial, marker, rest = written.partition("-")
        # 붙임표와 원어가 접사 표제어를 뒷받침하고 동형어도 분절이 같아야 한다.
        if (marker and rest and initial + rest == word and tags == ("명사",)
            and any(origin.startswith(value) for value in prefix_origins.get(initial, ()))):
            if not source_id.isascii() or not source_id.isdecimal():
                raise ValueError(f"{word}: 사전 표제어 번호가 필요합니다")
            source_ids.setdefault(word, []).append((initial, source_id))
        else:
            competing_words.add(word)
    output = csv.writer(sys.stdout, lineterminator="\n")
    prefixes = set()
    for word in sorted(source_ids.keys() - competing_words):
        initials = {initial for initial, _ in source_ids[word]}
        if len(initials) != 1:
            continue
        initial = next(iter(initials))
        ids = ",".join(sorted(source for _, source in source_ids[word]))
        if args.component:
            rest = word.removeprefix(initial)
            output.writerow((word, 0, 0, 0, "XPN+XR", f"stdict:{ids}", "F", word,
                             "Compound", "XPN", "XR", f"{initial}/XPN/*+{rest}/XR/*"))
            prefixes.add(initial)
        elif initial == "대":
            print(word, ids, sep="\t")
    for initial in sorted(prefixes):
        output.writerow((initial, 0, 0, 0, "XPN", "*", "F", initial, "*", "*", "*", "*"))


if __name__ == "__main__":
    main()
