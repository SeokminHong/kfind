#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import re
import sys
from collections import Counter, defaultdict
from collections.abc import Iterator
from pathlib import Path
from typing import NamedTuple
from xml.etree.ElementTree import Element

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import STDICT_SHA256, normalize_headword  # noqa: E402


class CatalogRecord(NamedTuple):
    written: str
    word: str
    tags: tuple[str, ...]
    origin: str
    source_id: str
    sense_ids: tuple[str, ...]
    nominal_variant_targets: tuple[str, ...]
    native_prefix_examples: tuple[str, ...]


NOMINAL_VARIANT = re.compile(r"‘<sense_no>(\d+)</sense_no>[^’]+’(?:을|를) 이르는 말\.")


def catalog_records(item: Element) -> Iterator[CatalogRecord]:
    word_info = item.find("word_info")
    if word_info is None:
        return
    raw = word_info.findtext("word", "")
    if " " in raw:
        return
    word, homonym = normalize_headword(raw)
    if not re.fullmatch("[가-힣]+", word):
        return
    written = raw.removesuffix(homonym)
    tags = tuple(entry.text or "" for entry in word_info.findall("pos_info/pos"))
    senses = word_info.findall("pos_info/comm_pattern_info/sense_info") if tags == ("명사",) else ()
    native_prefix_examples = ()
    if written == f"{word}-" and tags == ("접사",) and word_info.findtext("word_type") == "고유어":
        native_prefix_examples = tuple(
            example.text.removesuffix(".")
            for example in word_info.findall("pos_info/comm_pattern_info/sense_info/example_info/example")
            if example.text and re.fullmatch(r"[가-힣]+\.?", example.text)
        )
    yield CatalogRecord(
        written,
        word,
        tags,
        word_info.findtext("original_language_info/original_language", ""),
        item.findtext("target_code", ""),
        tuple(sense.findtext("sense_code", "") for sense in senses) if written != word else (),
        tuple(
            match.group(1)
            for sense in senses
            if written == word
            and (match := NOMINAL_VARIANT.fullmatch(sense.findtext("definition_original", "")))
        ),
        native_prefix_examples,
    )


def prefix_candidates(records: list[CatalogRecord]) -> dict[str, list[tuple[str, str]]]:
    prefix_origins: dict[str, set[str]] = {}
    nominal_variant_origins: dict[tuple[str, str], set[str]] = {}
    native_example_ids: dict[tuple[str, str], set[str]] = defaultdict(set)
    for record in records:
        if record.written == f"{record.word}-" and record.tags == ("접사",) and record.origin:
            prefix_origins.setdefault(record.word, set()).add(record.origin)
        if record.origin:
            for sense_id in record.nominal_variant_targets:
                nominal_variant_origins.setdefault((record.word, sense_id), set()).add(record.origin)
        for example in record.native_prefix_examples:
            native_example_ids[example, record.word].add(record.source_id)

    example_words = {word for word, _ in native_example_ids}
    example_counts = Counter(record.word for record in records if record.word in example_words)
    source_ids: dict[str, list[tuple[str, str]]] = {}
    competing_words: set[str] = set()
    for record in records:
        initial, marker, rest = record.written.partition("-")
        # 고유어 용례에 표제어 번호가 없으므로 동형어가 있으면 용례의 뜻을 고를 수 없다.
        native_sources = (
            native_example_ids.get((record.word, initial), ())
            if not record.origin and example_counts[record.word] == 1
            else ()
        )
        if not (
            marker and rest and initial + rest == record.word and record.tags == ("명사",)
            and (
                native_sources
                or any(record.origin.startswith(origin) for origin in prefix_origins.get(initial, ()))
            )
        ):
            competing_words.add(record.word)
            continue
        # 같은 원어의 독립 명사가 이 뜻풀이를 직접 참조하면 접사로 확정할 수 없다.
        if any(
            record.origin.startswith(origin)
            for sense_id in record.sense_ids
            for origin in nominal_variant_origins.get((initial, sense_id), ())
        ):
            competing_words.add(record.word)
            continue
        entry_ids = (record.source_id, *sorted(native_sources))
        if not all(source.isascii() and source.isdecimal() for source in entry_ids):
            raise ValueError(f"{record.word}: 사전 표제어 번호가 필요합니다")
        source_ids.setdefault(record.word, []).extend((initial, source) for source in entry_ids)
    return {word: ids for word, ids in source_ids.items() if word not in competing_words}


def main() -> None:
    parser = argparse.ArgumentParser(description="표준국어대사전의 접두사 결합 명사를 추출합니다.")
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    parser.add_argument("--component", action="store_true", help="공통 component 입력 CSV 출력")
    args = parser.parse_args()
    records = load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item", catalog_records, args.cache_dir
    )
    source_ids = prefix_candidates(records)
    output = csv.writer(sys.stdout, lineterminator="\n")
    prefixes = set()
    for word in sorted(source_ids):
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
