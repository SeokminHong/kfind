from __future__ import annotations

import csv
from collections import defaultdict
from pathlib import Path
from typing import Iterable

from nikl_lexemes import LexemeRecord


CATALOG_SOURCES = ("krdict", "stdict", "opendict")
REQUIRED_SOURCES = frozenset({"krdict", "stdict"})
CATALOG_HEADER = ("surface", "headwords", "krdict_ids", "stdict_ids", "opendict_ids")


def attached_nominal_suffixes(
    records: Iterable[LexemeRecord], requested: frozenset[str]
) -> tuple[LexemeRecord, ...]:
    return tuple(
        sorted(
            record
            for record in records
            if record.lemma in requested
            and record.pos == "접사"
            and "일반어" in record.statuses
            and record.headword.startswith("-")
        )
    )


def write_catalog(
    path: Path, requested: Iterable[str], records: Iterable[LexemeRecord]
) -> None:
    requested = tuple(sorted(set(requested)))
    by_surface: dict[str, list[LexemeRecord]] = defaultdict(list)
    for record in records:
        by_surface[record.lemma].append(record)

    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(CATALOG_HEADER)
        for surface in requested:
            surface_records = by_surface[surface]
            writer.writerow(
                [
                    surface,
                    "|".join(sorted({record.headword for record in surface_records})),
                    *[
                        "|".join(
                            sorted(
                                {
                                    record.source_id
                                    for record in surface_records
                                    if record.source == source
                                }
                            )
                        )
                        or "-"
                        for source in CATALOG_SOURCES
                    ],
                ]
            )


def validate_catalog(path: Path, requested: Iterable[str]) -> None:
    expected_surfaces = tuple(sorted(set(requested)))
    with path.open(encoding="utf-8", newline="") as source:
        rows = list(csv.reader(source, delimiter="\t"))

    if not rows or tuple(rows[0]) != CATALOG_HEADER:
        raise ValueError("attached nominal suffix catalog has an invalid schema")

    body = rows[1:]
    for line_number, row in enumerate(body, start=2):
        if len(row) != len(CATALOG_HEADER):
            raise ValueError(f"line {line_number}: expected {len(CATALOG_HEADER)} fields")

    actual_surfaces = tuple(row[0] for row in body)
    if actual_surfaces != expected_surfaces:
        raise ValueError(
            "attached nominal suffix surfaces must equal the sorted reviewed set"
        )

    for row in body:
        surface, headwords, *source_ids = row
        parsed_headwords = _catalog_values(headwords)
        if not parsed_headwords or any(not headword.startswith("-") for headword in parsed_headwords):
            raise ValueError(f"{surface}: suffix headwords must start with '-'")

        missing = [
            source
            for source, values in zip(CATALOG_SOURCES, source_ids, strict=True)
            if source in REQUIRED_SOURCES and not _catalog_values(values)
        ]
        if missing:
            raise ValueError(
                f"{surface}: missing modern suffix evidence from {', '.join(missing)}"
            )


def _catalog_values(field: str) -> tuple[str, ...]:
    if field == "-":
        return ()
    values = tuple(field.split("|"))
    if not values or any(not value for value in values):
        raise ValueError("catalog values must be non-empty")
    if values != tuple(sorted(set(values))):
        raise ValueError("catalog values must be sorted and unique")
    return values
