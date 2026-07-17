from __future__ import annotations

import csv
import json
import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import xml.etree.ElementTree as ET

from nikl_import import direct_feat, descendant_texts, normalize_headword, normalize_text


@dataclass(frozen=True, order=True)
class ParticleRecord:
    source: str
    source_id: str
    headword: str
    surface: str
    statuses: tuple[str, ...]


def normalize_particle(raw: str) -> tuple[str, str]:
    headword = normalize_text(raw)
    surface, _ = normalize_headword(headword)
    return headword, surface


def krdict_particles(entry: ET.Element) -> tuple[ParticleRecord, ...]:
    if direct_feat(entry, "partOfSpeech") != "조사":
        return ()
    lemma = entry.find("./Lemma")
    raw = direct_feat(lemma, "writtenForm") if lemma is not None else None
    if not raw:
        return ()
    headword, surface = normalize_particle(raw)
    if not surface:
        return ()
    return (
        ParticleRecord(
            source="krdict",
            source_id=entry.get("val") or entry.get("id") or "",
            headword=headword,
            surface=surface,
            statuses=("일반어",),
        ),
    )


def stdict_particles(item: ET.Element) -> tuple[ParticleRecord, ...]:
    word_info = item.find("./word_info")
    if word_info is None:
        return ()
    raw = word_info.findtext("./word")
    if not raw:
        return ()
    headword, surface = normalize_particle(raw)
    if not surface:
        return ()
    target = normalize_text(item.findtext("./target_code", ""))
    records = []
    for pos_info in word_info.findall("./pos_info"):
        if normalize_text(pos_info.findtext("./pos", "")) != "조사":
            continue
        pos_code = normalize_text(pos_info.findtext("./pos_code", ""))
        statuses = descendant_texts(pos_info, (".//sense_info/type",)) or ("unknown",)
        records.append(
            ParticleRecord(
                source="stdict",
                source_id=f"{target}:{pos_code}" if pos_code else target,
                headword=headword,
                surface=surface,
                statuses=statuses,
            )
        )
    return tuple(records)


def opendict_particles(item: ET.Element) -> tuple[ParticleRecord, ...]:
    word_info = item.find("./wordInfo")
    raw = word_info.findtext("./word") if word_info is not None else None
    if not raw:
        return ()
    headword, surface = normalize_particle(raw)
    if not surface:
        return ()
    target = normalize_text(item.findtext("./target_code", ""))
    records = []
    for index, sense in enumerate(item.findall("./senseInfo"), start=1):
        if normalize_text(sense.findtext("./pos", "")) != "조사":
            continue
        statuses = descendant_texts(sense, ("./type",)) or ("unknown",)
        records.append(
            ParticleRecord(
                source="opendict",
                source_id=f"{target}:{index}",
                headword=headword,
                surface=surface,
                statuses=statuses,
            )
        )
    return tuple(records)


def write_catalog(path: Path, records: Iterable[ParticleRecord]) -> None:
    grouped = group_by_surface(records)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(
            (
                "surface",
                "headwords",
                "krdict_ids",
                "stdict_ids",
                "opendict_ids",
                "stdict_statuses",
                "opendict_statuses",
            )
        )
        for surface, values in sorted(grouped.items()):
            if not any(value.source in {"krdict", "stdict"} for value in values):
                continue
            writer.writerow(
                (
                    surface,
                    "|".join(sorted({value.headword for value in values})),
                    source_values(values, "krdict", "source_id"),
                    source_values(values, "stdict", "source_id"),
                    source_values(values, "opendict", "source_id"),
                    source_statuses(values, "stdict"),
                    source_statuses(values, "opendict"),
                )
            )


def write_coverage_report(
    path: Path, records: Iterable[ParticleRecord], rules_path: Path
) -> None:
    grouped = group_by_surface(records)
    krdict = source_surfaces(grouped, "krdict")
    stdict = source_surfaces(grouped, "stdict")
    stdict_general = {
        surface
        for surface, values in grouped.items()
        if any(value.source == "stdict" and "일반어" in value.statuses for value in values)
    }
    opendict = source_surfaces(grouped, "opendict")
    primary_modern = krdict | stdict_general
    consensus = krdict & stdict_general
    runtime, transitions = runtime_model(rules_path)
    runtime_surfaces = set(runtime)
    generated = generated_surfaces(runtime, transitions, max_rules=4)
    graph_covered_modern = primary_modern & set(generated)
    graph_covered_consensus = consensus & set(generated)
    report = {
        "krdict_surface_count": len(krdict),
        "stdict_surface_count": len(stdict),
        "stdict_general_surface_count": len(stdict_general),
        "opendict_surface_count": len(opendict),
        "primary_modern_surface_count": len(primary_modern),
        "primary_consensus_surface_count": len(consensus),
        "runtime_surface_count": len(runtime_surfaces),
        "runtime_primary_modern_covered_count": len(primary_modern & runtime_surfaces),
        "runtime_primary_consensus_covered_count": len(consensus & runtime_surfaces),
        "graph_primary_modern_covered_count": len(graph_covered_modern),
        "graph_primary_consensus_covered_count": len(graph_covered_consensus),
        "graph_covered_primary_modern_surfaces": sorted(graph_covered_modern),
        "graph_covered_primary_consensus_surfaces": sorted(graph_covered_consensus),
        "missing_primary_modern_surfaces": sorted(primary_modern - runtime_surfaces),
        "missing_primary_consensus_surfaces": sorted(consensus - runtime_surfaces),
        "graph_missing_primary_modern_surfaces": sorted(
            primary_modern - graph_covered_modern
        ),
        "graph_missing_primary_consensus_surfaces": sorted(
            consensus - graph_covered_consensus
        ),
        "runtime_surfaces_outside_primary_modern": sorted(runtime_surfaces - primary_modern),
        "runtime_rules": runtime,
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def group_by_surface(
    records: Iterable[ParticleRecord],
) -> dict[str, list[ParticleRecord]]:
    grouped: dict[str, list[ParticleRecord]] = defaultdict(list)
    for record in records:
        grouped[record.surface].append(record)
    return grouped


def source_surfaces(
    grouped: dict[str, list[ParticleRecord]], source: str
) -> set[str]:
    return {
        surface
        for surface, values in grouped.items()
        if any(value.source == source for value in values)
    }


def source_values(records: Iterable[ParticleRecord], source: str, field: str) -> str:
    values = sorted(
        {getattr(record, field) for record in records if record.source == source}
    )
    return "|".join(values) if values else "-"


def source_statuses(records: Iterable[ParticleRecord], source: str) -> str:
    values = sorted(
        {
            status
            for record in records
            if record.source == source
            for status in record.statuses
        }
    )
    return "|".join(values) if values else "-"


def runtime_model(
    path: Path,
) -> tuple[dict[str, list[str]], dict[str, list[str]]]:
    parsed = tomllib.loads(path.read_text(encoding="utf-8"))
    by_surface: dict[str, list[str]] = defaultdict(list)
    transitions = {}
    for rule in parsed.get("particle", []):
        transitions[rule["id"]] = rule.get("next", [])
        for surface in rule.get("forms", []):
            by_surface[surface].append(rule["id"])
    return (
        {
            surface: sorted(set(rule_ids))
            for surface, rule_ids in sorted(by_surface.items())
        },
        transitions,
    )


def generated_surfaces(
    runtime: dict[str, list[str]],
    transitions: dict[str, list[str]],
    max_rules: int,
) -> dict[str, list[tuple[str, ...]]]:
    forms_by_rule: dict[str, list[str]] = defaultdict(list)
    for surface, rule_ids in runtime.items():
        for rule_id in rule_ids:
            forms_by_rule[rule_id].append(surface)

    generated: dict[str, list[tuple[str, ...]]] = defaultdict(list)

    def visit(surface: str, path: tuple[str, ...]) -> None:
        generated[surface].append(path)
        if len(path) == max_rules:
            return
        for next_rule in transitions.get(path[-1], []):
            for form in forms_by_rule[next_rule]:
                visit(surface + form, (*path, next_rule))

    for rule_id, forms in forms_by_rule.items():
        for form in forms:
            visit(form, (rule_id,))
    return dict(generated)
