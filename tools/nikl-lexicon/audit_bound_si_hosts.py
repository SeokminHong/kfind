#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import sys
from collections import defaultdict
from collections.abc import Iterator
from pathlib import Path
from xml.etree.ElementTree import Element

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import STDICT_SHA256, normalize_headword  # noqa: E402


def headwords(item: Element) -> Iterator[tuple[str, str, str, str, set[str]]]:
    raw = item.findtext("./word_info/word", "")
    word, _ = normalize_headword(raw)
    if not word.endswith(("시", "시하다")):
        return
    word_info = item.find("./word_info")
    if word_info is None:
        return
    spelling = raw.rstrip("0123456789")
    origin = "".join(
        language.text or ""
        for language in word_info.findall("./original_language_info/original_language")
    )
    positions = {node.text or "" for node in word_info.findall("./pos_info/pos")}
    yield word, spelling, item.findtext("./target_code", ""), origin, positions


def main() -> None:
    parser = argparse.ArgumentParser(
        description="표준국어대사전에서 접미사 시(視)가 붙은 표제어를 추출합니다."
    )
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    parser.add_argument("--format", choices=("rust", "fixture"), default="rust")
    args = parser.parse_args()
    words = defaultdict(list)
    for word, spelling, source_id, origin, positions in load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item", headwords, args.cache_dir
    ):
        words[word].append((spelling, source_id, origin, positions))
    independent_origins = {
        origin for _, _, origin, positions in words["시"] if "명사" in positions
    }
    if "視" in independent_origins:
        raise ValueError("시(視)의 독립 명사 등재 여부를 다시 확인해야 합니다.")

    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    if args.format == "rust":
        print("// 표준국어대사전 표제어와 명사 성분의 부재 및 하다 결합")
        print("const STDICT_BOUND_SI_HOSTS: &[(&str, bool, bool)] = &[")
    else:
        writer.writerow(("id", "query", "pos", "expected", "byte_start", "byte_end", "text"))
    for word, entries in sorted(words.items()):
        identifiers = [
            source_id
            for spelling, source_id, origin, _ in entries
            if len(word) > 1 and spelling.endswith("-시") and origin.endswith("視")
        ]
        if not identifiers:
            continue
        no_noun_component = not any(
            spelling.endswith("-시")
            and "명사" in positions
            and any(origin.endswith(noun) for noun in independent_origins if noun)
            for spelling, _, origin, positions in entries
        )
        verbs = words.get(word + "하다", [])
        has_hada = bool(verbs) and all(
            origin.endswith("視하다") and "동사" in positions
            for _, _, origin, positions in verbs
        )
        source_id = "|".join(sorted(identifiers))
        if args.format == "rust":
            print(
                f'    ("{word}", {str(no_noun_component).lower()}, '
                f'{str(has_hada).lower()}), // {source_id}'
            )
        else:
            for form, enabled, text in (
                ("noun", no_noun_component, f"그는 {word}도 언급했다."),
                ("verb", has_hada, f"그는 그 사실을 {word}했다."),
            ):
                if enabled:
                    start = len(text[: text.index(word) + len(word) - 1].encode())
                    writer.writerow(
                        (
                            f"stdict.{source_id}.{form}",
                            "시",
                            "noun",
                            "no-match",
                            start,
                            start + 3,
                            text,
                        )
                    )
    if args.format == "rust":
        print("];")


if __name__ == "__main__":
    main()
