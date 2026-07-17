from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable

from nikl_import import direct_feat, normalize_headword, normalize_text


@dataclass(frozen=True, order=True)
class LexicalRelation:
    relation_type: str
    target_headword: str
    target_id: str
    unit: str


@dataclass(frozen=True, order=True)
class LexemeRecord:
    source: str
    source_id: str
    headword: str
    lemma: str
    pos: str
    statuses: tuple[str, ...]
    relations: tuple[LexicalRelation, ...]


def krdict_lexemes(
    entry: ET.Element, requested: frozenset[str]
) -> tuple[LexemeRecord, ...]:
    lemma_element = entry.find("./Lemma")
    raw = direct_feat(lemma_element, "writtenForm") if lemma_element is not None else None
    if not raw:
        return ()
    lemma, _ = normalize_headword(raw)
    if lemma not in requested:
        return ()
    relations = tuple(
        sorted(
            LexicalRelation(
                relation_type=direct_feat(related, "type") or "",
                target_headword=normalize_text(direct_feat(related, "writtenForm") or ""),
                target_id=direct_feat(related, "id") or "",
                unit="어휘",
            )
            for related in entry.findall("./RelatedForm")
        )
    )
    return (
        LexemeRecord(
            source="krdict",
            source_id=entry.get("val") or entry.get("id") or "",
            headword=normalize_text(raw),
            lemma=lemma,
            pos=direct_feat(entry, "partOfSpeech") or "",
            statuses=("일반어",),
            relations=relations,
        ),
    )


def stdict_lexemes(
    item: ET.Element, requested: frozenset[str]
) -> tuple[LexemeRecord, ...]:
    word_info = item.find("./word_info")
    if word_info is None:
        return ()
    raw = normalize_text(word_info.findtext("./word", ""))
    lemma, _ = normalize_headword(raw)
    if not raw or lemma not in requested:
        return ()
    target = normalize_text(item.findtext("./target_code", ""))
    word_relations = lexical_relations(word_info.findall("./lexical_info"))
    records = []
    for pos_info in word_info.findall("./pos_info"):
        pos_code = normalize_text(pos_info.findtext("./pos_code", ""))
        statuses = texts(pos_info.findall(".//sense_info/type")) or ("unknown",)
        relations = tuple(
            sorted(
                set(word_relations)
                | set(lexical_relations(pos_info.findall(".//lexical_info")))
            )
        )
        records.append(
            LexemeRecord(
                source="stdict",
                source_id=f"{target}:{pos_code}" if pos_code else target,
                headword=raw,
                lemma=lemma,
                pos=normalize_text(pos_info.findtext("./pos", "")),
                statuses=statuses,
                relations=relations,
            )
        )
    return tuple(records)


def opendict_lexemes(
    item: ET.Element, requested: frozenset[str]
) -> tuple[LexemeRecord, ...]:
    word_info = item.find("./wordInfo")
    if word_info is None:
        return ()
    raw = normalize_text(word_info.findtext("./word", ""))
    lemma, _ = normalize_headword(raw)
    if not raw or lemma not in requested:
        return ()
    target = normalize_text(item.findtext("./target_code", ""))
    relations = relation_info(item.findall(".//relation_info"))
    records = []
    for index, sense in enumerate(item.findall("./senseInfo"), start=1):
        records.append(
            LexemeRecord(
                source="opendict",
                source_id=f"{target}:{index}",
                headword=raw,
                lemma=lemma,
                pos=normalize_text(sense.findtext("./pos", "")),
                statuses=texts(sense.findall("./type")) or ("unknown",),
                relations=relations,
            )
        )
    return tuple(records)


def lexical_relations(elements: Iterable[ET.Element]) -> tuple[LexicalRelation, ...]:
    return tuple(
        sorted(
            {
                LexicalRelation(
                    relation_type=normalize_text(element.findtext("./type", "")),
                    target_headword=normalize_text(element.findtext("./word", "")),
                    target_id=normalize_text(element.findtext("./link_target_code", "")),
                    unit=normalize_text(element.findtext("./unit", "")),
                )
                for element in elements
            }
        )
    )


def relation_info(elements: Iterable[ET.Element]) -> tuple[LexicalRelation, ...]:
    return tuple(
        sorted(
            {
                LexicalRelation(
                    relation_type=normalize_text(element.findtext("./type", "")),
                    target_headword=normalize_text(element.findtext("./word", "")),
                    target_id=normalize_text(element.findtext("./link_target_code", "")),
                    unit="관계",
                )
                for element in elements
            }
        )
    )


def texts(elements: Iterable[ET.Element]) -> tuple[str, ...]:
    return tuple(
        sorted(
            {
                normalize_text(element.text)
                for element in elements
                if element.text and normalize_text(element.text)
            }
        )
    )


def write_report(
    path: Path,
    queries: Iterable[str],
    records: Iterable[LexemeRecord],
    snapshots: dict[str, str],
) -> None:
    normalized_queries = sorted({normalize_headword(query)[0] for query in queries})
    sorted_records = sorted(records)
    by_source = {
        source: sorted(
            set(normalized_queries)
            - {record.lemma for record in sorted_records if record.source == source}
        )
        for source in snapshots
    }
    payload = {
        "schema_version": 1,
        "snapshots": snapshots,
        "queries": normalized_queries,
        "missing_by_source": by_source,
        "records": [asdict(record) for record in sorted_records],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
