#!/usr/bin/env python3

"""Report case-particle slots and missing frames by predicate and part of speech."""

from __future__ import annotations

import argparse
import csv
import io
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_import import (
    INVALID_XML_BYTE,
    KRDICT_INVALID_BYTE_LOCATIONS,
    KRDICT_SHA256,
    direct_feat,
    file_sha256,
    normalize_headword,
    snapshot_members,
)

PREDICATE_POS = {"동사": "VV", "형용사": "VA"}
SLOT = re.compile(r"(?<!\d)\d([^\s()\d]+)")


def iter_entries(archive: Path):
    checksum = file_sha256(archive)
    if checksum != KRDICT_SHA256:
        raise ValueError(f"unexpected Korean Basic Dictionary SHA-256: {checksum}")
    invalid_locations = []
    for member, raw in snapshot_members("krdict", archive, checksum, None):
        invalid_locations.extend(
            f"{member}:{match.start()}" for match in re.finditer(INVALID_XML_BYTE, raw)
        )
        for _, entry in ET.iterparse(io.BytesIO(raw.replace(INVALID_XML_BYTE, b"")), events=("end",)):
            if entry.tag == "LexicalEntry":
                yield entry
                entry.clear()
    if tuple(invalid_locations) != KRDICT_INVALID_BYTE_LOCATIONS:
        raise ValueError("Korean Basic Dictionary invalid XML byte locations changed")


def collect_frames(archive: Path):
    predicates = defaultdict(list)
    for entry in iter_entries(archive):
        pos = PREDICATE_POS.get(direct_feat(entry, "partOfSpeech"))
        lemma_element = entry.find("./Lemma")
        if pos is None or lemma_element is None:
            continue
        lemma, _ = normalize_headword(direct_feat(lemma_element, "writtenForm") or "")
        if not lemma.endswith("다"):
            continue
        for sense in entry.findall("./Sense"):
            frames = tuple(
                value.strip()
                for feature in sense.findall('.//feat[@att="syntacticPattern"]')
                if (value := feature.get("val", "")).strip()
            )
            annotations = bool(sense.findall('.//feat[@att="syntacticAnnotation"]'))
            predicates[lemma, pos].append((frames, annotations))
    return predicates


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    predicates = collect_frames(args.archive)
    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    writer.writerow(
        ("lemma", "pos", "senses", "framed_senses", "annotated_senses", "object_slot", "ro_slot")
    )
    for (lemma, pos), senses in sorted(predicates.items()):
        slots = {
            choice
            for frames, _ in senses
            for frame in frames
            for slot in SLOT.findall(frame)
            for choice in slot.split("/")
        }
        writer.writerow(
            (
                lemma,
                pos,
                len(senses),
                sum(bool(frames) for frames, _ in senses),
                sum(annotated for _, annotated in senses),
                int(not slots.isdisjoint({"을", "를"})),
                int(not slots.isdisjoint({"으로", "로"})),
            )
        )


if __name__ == "__main__":
    main()
