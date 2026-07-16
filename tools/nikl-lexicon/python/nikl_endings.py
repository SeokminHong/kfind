from __future__ import annotations

import csv
import unicodedata
import xml.etree.ElementTree as ET
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from nikl_import import HOMONYM_SUFFIX, direct_feat


@dataclass(frozen=True, order=True)
class EndingRecord:
    source: str
    source_id: str
    headword: str
    surface: str
    categories: tuple[str, ...]


def normalize_ending(raw: str) -> tuple[str, str]:
    headword = unicodedata.normalize("NFC", raw.strip())
    surface = headword.replace("-", "").replace("^", "").replace(" ", "")
    if match := HOMONYM_SUFFIX.fullmatch(surface):
        surface = match.group("lemma")
    return headword, surface


def ending_categories(definitions: Iterable[str]) -> tuple[str, ...]:
    categories = set()
    for definition in definitions:
        if "선어말 어미" in definition:
            categories.add("prefinal")
        if "종결 어미" in definition:
            categories.add("final")
        if "연결 어미" in definition:
            categories.add("connective")
        if "관형사형" in definition or "관형형" in definition:
            categories.add("adnominal")
        if "명사형" in definition:
            categories.add("nominalizer")
    return tuple(sorted(categories or {"unclassified"}))


def krdict_endings(entry: ET.Element) -> tuple[EndingRecord, ...]:
    if direct_feat(entry, "partOfSpeech") != "어미":
        return ()
    lemma = entry.find("./Lemma")
    raw = direct_feat(lemma, "writtenForm") if lemma is not None else None
    if not raw:
        return ()
    headword, surface = normalize_ending(raw)
    if not surface:
        return ()
    definitions = tuple(
        value
        for sense in entry.findall("./Sense")
        if (value := direct_feat(sense, "definition"))
    )
    return (
        EndingRecord(
            source="krdict",
            source_id=entry.get("val") or entry.get("id") or "",
            headword=headword,
            surface=surface,
            categories=ending_categories(definitions),
        ),
    )


def stdict_endings(item: ET.Element) -> tuple[EndingRecord, ...]:
    word_info = item.find("./word_info")
    if word_info is None:
        return ()
    raw = word_info.findtext("./word")
    if not raw:
        return ()
    headword, surface = normalize_ending(raw)
    records = []
    target = item.findtext("./target_code", "").strip()
    for pos_info in word_info.findall("./pos_info"):
        if pos_info.findtext("./pos") != "어미":
            continue
        definitions = tuple(
            definition.text.strip()
            for definition in pos_info.findall(".//sense_info/definition")
            if definition.text and definition.text.strip()
        )
        pos_code = pos_info.findtext("./pos_code", "").strip()
        records.append(
            EndingRecord(
                source="stdict",
                source_id=f"{target}:{pos_code}" if pos_code else target,
                headword=headword,
                surface=surface,
                categories=ending_categories(definitions),
            )
        )
    return tuple(records)


def opendict_endings(item: ET.Element) -> tuple[EndingRecord, ...]:
    word_info = item.find("./wordInfo")
    raw = word_info.findtext("./word") if word_info is not None else None
    if not raw:
        return ()
    headword, surface = normalize_ending(raw)
    target = item.findtext("./target_code", "").strip()
    definitions = []
    matched = False
    for sense in item.findall("./senseInfo"):
        if sense.findtext("./pos") != "어미":
            continue
        matched = True
        definition = sense.findtext("./definition")
        if definition and definition.strip():
            definitions.append(definition.strip())
    if not matched:
        return ()
    return (
        EndingRecord(
            source="opendict",
            source_id=target,
            headword=headword,
            surface=surface,
            categories=ending_categories(definitions),
        ),
    )


def write_catalog(path: Path, records: Iterable[EndingRecord]) -> None:
    grouped: dict[str, list[EndingRecord]] = defaultdict(list)
    for record in records:
        grouped[record.surface].append(record)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(
            ("surface", "headwords", "categories", "krdict_ids", "stdict_ids", "opendict_ids")
        )
        for surface, values in sorted(grouped.items()):
            if not any(value.source in {"krdict", "stdict"} for value in values):
                continue
            writer.writerow(
                (
                    surface,
                    "|".join(sorted({value.headword for value in values})),
                    "|".join(sorted({category for value in values for category in value.categories})),
                    ids(values, "krdict"),
                    ids(values, "stdict"),
                    ids(values, "opendict"),
                )
            )


def ids(records: Iterable[EndingRecord], source: str) -> str:
    values = sorted({record.source_id for record in records if record.source == source})
    return "|".join(values) if values else "-"
