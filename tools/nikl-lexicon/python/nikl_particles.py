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
    hosts: tuple[str, ...]


def normalize_particle(raw: str) -> tuple[str, str]:
    headword = normalize_text(raw)
    surface, _ = normalize_headword(headword)
    return headword, surface


def grammar_hosts(notes: Iterable[str]) -> tuple[str, ...]:
    text = " ".join(normalize_text(note) for note in notes if normalize_text(note))
    hosts = set()
    if any(marker in text for marker in ("체언", "명사", "대명사", "수사")):
        hosts.add("nominal")
    if "부사" in text:
        hosts.add("adverb")
    if any(marker in text for marker in ("어미", "활용형", "용언")):
        hosts.add("predicate-ending")
    return tuple(sorted(hosts))


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
            hosts=grammar_hosts(
                direct_feat(sense, "annotation") or ""
                for sense in entry.findall("./Sense")
            ),
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
                hosts=grammar_hosts(
                    grammar.text or "" for grammar in pos_info.findall(".//grammar")
                ),
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
                hosts=grammar_hosts(
                    grammar.text or "" for grammar in sense.findall(".//grammar")
                ),
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
                "krdict_hosts",
                "stdict_hosts",
                "opendict_hosts",
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
                    source_hosts(values, "krdict"),
                    source_hosts(values, "stdict"),
                    source_hosts(values, "opendict"),
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
    runtime, transitions, rule_grammar = runtime_model(rules_path)
    runtime_surfaces = set(runtime)
    generated = generated_surfaces(runtime, transitions, max_rules=4)
    graph_covered_modern = primary_modern & set(generated)
    graph_covered_consensus = consensus & set(generated)
    consensus_adverb_hosts = {
        surface
        for surface, values in grouped.items()
        if source_has_host(values, "krdict", "adverb")
        and source_has_host(values, "stdict", "adverb", require_general=True)
    }
    runtime_adverb_hosts = {
        surface
        for surface, rule_ids in runtime.items()
        if any(
            rule_grammar[rule_id]["role"] == "auxiliary"
            and "adverb" in rule_grammar[rule_id]["hosts"]
            for rule_id in rule_ids
        )
    }
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
        "dictionary_consensus_adverb_host_surfaces": sorted(consensus_adverb_hosts),
        "runtime_adverb_host_surfaces": sorted(runtime_adverb_hosts),
        "runtime_adverb_host_missing_consensus": sorted(
            consensus_adverb_hosts - runtime_adverb_hosts
        ),
        "runtime_adverb_host_outside_consensus": sorted(
            runtime_adverb_hosts - consensus_adverb_hosts
        ),
        "runtime_rule_grammar": rule_grammar,
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


def source_hosts(records: Iterable[ParticleRecord], source: str) -> str:
    values = sorted(
        {
            host
            for record in records
            if record.source == source
            for host in record.hosts
        }
    )
    return "|".join(values) if values else "-"


def source_has_host(
    records: Iterable[ParticleRecord],
    source: str,
    host: str,
    require_general: bool = False,
) -> bool:
    return any(
        record.source == source
        and host in record.hosts
        and (not require_general or "일반어" in record.statuses)
        for record in records
    )


def runtime_model(
    path: Path,
) -> tuple[
    dict[str, list[str]], dict[str, list[str]], dict[str, dict[str, object]]
]:
    parsed = tomllib.loads(path.read_text(encoding="utf-8"))
    by_surface: dict[str, list[str]] = defaultdict(list)
    transitions = {}
    grammar = {}
    for rule in parsed.get("particle", []):
        transitions[rule["id"]] = rule.get("next", [])
        grammar[rule["id"]] = {
            "role": rule["role"],
            "hosts": rule["hosts"],
        }
        for surface in rule.get("forms", []):
            by_surface[surface].append(rule["id"])
    return (
        {
            surface: sorted(set(rule_ids))
            for surface, rule_ids in sorted(by_surface.items())
        },
        transitions,
        grammar,
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
