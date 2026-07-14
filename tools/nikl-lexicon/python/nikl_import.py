from __future__ import annotations

import csv
import hashlib
import io
import json
import os
import re
import shutil
import tempfile
import unicodedata
import xml.etree.ElementTree as ET
import zipfile
from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass, replace
from pathlib import Path


SUPPORTED_POS = {"동사": "VV", "형용사": "VA"}
INVALID_XML_BYTE = b"\x08"
HOMONYM_SUFFIX = re.compile(r"(?P<lemma>.+?)(?P<homonym>\d{2,3})$")
KRDICT_SHA256 = "a8ab7d044d4f6341e0f217db63f38f4d18beed3e1f153130f6cb4e9494fea1d6"
STDICT_SHA256 = "880b31447146df5879c076012b21d4cc3c0c24e70fd91be7fc73f7ff7da34d52"
OPENDICT_SHA256 = "9e8807e5fade8c7b59431d1ab527fe93aafd15395001bcdde88511e8c9293b42"
KRDICT_INVALID_BYTE_LOCATIONS = (
    "1_5000_20260619.xml:7177584",
    "1_5000_20260619.xml:7707029",
    "2_5000_20260619.xml:26688097",
    "3_5000_20260619.xml:1622737",
    "5_5000_20260619.xml:2780232",
    "5_5000_20260619.xml:25054056",
    "5_5000_20260619.xml:25059684",
)


@dataclass(frozen=True, order=True)
class PredicateRecord:
    source: str
    source_id: str
    raw_homonym: str
    lemma: str
    pos: str
    lexical_status: str
    conjugations: tuple[str, ...]
    related_adverbs: tuple[str, ...]


@dataclass(frozen=True)
class ImportStats:
    source: str
    filename: str
    sha256: str
    item_count: int
    predicate_count: int
    predicate_with_conjugations_count: int
    related_adverb_count: int
    sanitized_byte_count: int
    sanitized_locations: tuple[str, ...]


def normalize_text(value: str) -> str:
    return unicodedata.normalize("NFC", value.strip())


def normalize_headword(raw: str) -> tuple[str, str]:
    normalized = normalize_text(raw).replace("-", "").replace("^", "").replace(" ", "")
    match = HOMONYM_SUFFIX.fullmatch(normalized)
    if match is None:
        return normalized, ""
    return match.group("lemma"), match.group("homonym")


def direct_feat(element: ET.Element, attribute: str) -> str | None:
    for child in element.findall("./feat"):
        if child.get("att") == attribute:
            return child.get("val")
    return None


def descendant_texts(element: ET.Element, paths: Iterable[str]) -> tuple[str, ...]:
    values = {
        normalize_text(child.text)
        for path in paths
        for child in element.findall(path)
        if child.text and normalize_text(child.text)
    }
    return tuple(sorted(values))


def krdict_record(entry: ET.Element) -> Iterable[PredicateRecord]:
    pos = SUPPORTED_POS.get(direct_feat(entry, "partOfSpeech") or "")
    lemma_element = entry.find("./Lemma")
    raw_lemma = direct_feat(lemma_element, "writtenForm") if lemma_element is not None else None
    if pos is None or raw_lemma is None:
        return ()
    lemma, suffix_homonym = normalize_headword(raw_lemma)
    if not lemma.endswith("다"):
        return ()
    forms = []
    for word_form in entry.findall("./WordForm"):
        if direct_feat(word_form, "type") != "활용":
            continue
        written_form = direct_feat(word_form, "writtenForm")
        if written_form:
            forms.append(normalize_text(written_form))
    source_id = entry.get("val") or entry.get("id") or ""
    raw_homonym = direct_feat(entry, "homonym_number") or suffix_homonym
    return (
        PredicateRecord(
            source="krdict",
            source_id=source_id,
            raw_homonym=raw_homonym,
            lemma=lemma,
            pos=pos,
            lexical_status="일반어",
            conjugations=tuple(sorted(set(forms))),
            related_adverbs=(),
        ),
    )


def stdict_record(item: ET.Element) -> Iterable[PredicateRecord]:
    target_code = normalize_text(item.findtext("./target_code", ""))
    word_info = item.find("./word_info")
    if word_info is None:
        return ()
    raw_word = word_info.findtext("./word", "")
    lemma, raw_homonym = normalize_headword(raw_word)
    if not lemma.endswith("다"):
        return ()
    forms = descendant_texts(
        word_info,
        (
            "./conju_info/conjugation_info/conjugation",
            "./conju_info/abbreviation_info/abbreviation",
        ),
    )
    records = []
    for pos_info in word_info.findall("./pos_info"):
        pos = SUPPORTED_POS.get(normalize_text(pos_info.findtext("./pos", "")))
        if pos is None:
            continue
        types = descendant_texts(pos_info, (".//sense_info/type",))
        definitions = descendant_texts(pos_info, (".//sense_info/definition",))
        status = lexical_status(types, definitions)
        pos_code = normalize_text(pos_info.findtext("./pos_code", ""))
        records.append(
            PredicateRecord(
                source="stdict",
                source_id=f"{target_code}:{pos_code}" if pos_code else target_code,
                raw_homonym=raw_homonym,
                lemma=lemma,
                pos=pos,
                lexical_status=status,
                conjugations=forms,
                related_adverbs=(),
            )
        )
    return records


def opendict_record(item: ET.Element) -> Iterable[PredicateRecord]:
    target_code = normalize_text(item.findtext("./target_code", ""))
    word_info = item.find("./wordInfo")
    if word_info is None:
        return ()
    raw_word = word_info.findtext("./word", "")
    lemma, raw_homonym = normalize_headword(raw_word)
    if not lemma.endswith("다"):
        return ()
    forms = descendant_texts(
        item,
        (
            ".//conju_info/conjugation_info/conjugation",
            ".//conju_info/abbreviation_info/abbreviation",
        ),
    )
    records = []
    for index, sense_info in enumerate(item.findall("./senseInfo"), start=1):
        pos = SUPPORTED_POS.get(normalize_text(sense_info.findtext("./pos", "")))
        if pos is None:
            continue
        types = descendant_texts(sense_info, ("./type",))
        definitions = descendant_texts(sense_info, ("./definition",))
        records.append(
            PredicateRecord(
                source="opendict",
                source_id=f"{target_code}:{index}",
                raw_homonym=raw_homonym,
                lemma=lemma,
                pos=pos,
                lexical_status=lexical_status(types, definitions),
                conjugations=forms,
                related_adverbs=(),
            )
        )
    return records


def lexical_status(types: tuple[str, ...], definitions: tuple[str, ...]) -> str:
    if definitions and all(definition.lstrip().startswith("→") for definition in definitions):
        return "redirect"
    if "일반어" in types:
        return "일반어"
    return "|".join(types) if types else "unknown"


def import_snapshot(
    source: str,
    path: Path,
    element_tag: str,
    adapter: Callable[[ET.Element], Iterable[PredicateRecord]],
    expected_invalid_bytes: int = 0,
    expected_invalid_locations: tuple[str, ...] | None = None,
    expected_sha256: str | None = None,
    cache_directory: Path | None = None,
) -> tuple[list[PredicateRecord], ImportStats]:
    records: list[PredicateRecord] = []
    item_count = 0
    sanitized_locations: list[str] = []
    sha256 = file_sha256(path)
    if expected_sha256 is not None and sha256 != expected_sha256:
        raise ValueError(f"{source}: expected SHA-256 {expected_sha256}, found {sha256}")
    for member, raw in snapshot_members(source, path, sha256, cache_directory):
        offsets = [match.start() for match in re.finditer(INVALID_XML_BYTE, raw)]
        sanitized_locations.extend(f"{member}:{offset}" for offset in offsets)
        if offsets:
            raw = raw.replace(INVALID_XML_BYTE, b"")
        for _, element in ET.iterparse(io.BytesIO(raw), events=("end",)):
            if element.tag != element_tag:
                continue
            item_count += 1
            records.extend(adapter(element))
            element.clear()
    if len(sanitized_locations) != expected_invalid_bytes:
        raise ValueError(
            f"{source}: expected {expected_invalid_bytes} invalid XML bytes, "
            f"found {len(sanitized_locations)}"
        )
    if (
        expected_invalid_locations is not None
        and tuple(sanitized_locations) != expected_invalid_locations
    ):
        raise ValueError(f"{source}: invalid XML byte locations changed")
    records.sort()
    stats = ImportStats(
        source=source,
        filename=path.name,
        sha256=sha256,
        item_count=item_count,
        predicate_count=len(records),
        predicate_with_conjugations_count=sum(bool(record.conjugations) for record in records),
        related_adverb_count=sum(len(record.related_adverbs) for record in records),
        sanitized_byte_count=len(sanitized_locations),
        sanitized_locations=tuple(sanitized_locations),
    )
    return records, stats


@dataclass(frozen=True)
class KrDictEntry:
    lemma: str
    pos: str
    related_forms: tuple[tuple[str, str, str], ...]


def attach_krdict_related_adverbs(
    records: Iterable[PredicateRecord],
    path: Path,
    cache_directory: Path | None,
) -> tuple[list[PredicateRecord], int]:
    sha256 = file_sha256(path)
    if sha256 != KRDICT_SHA256:
        raise ValueError(f"krdict: expected SHA-256 {KRDICT_SHA256}, found {sha256}")
    entries: dict[str, KrDictEntry] = {}
    for _, raw in snapshot_members("krdict", path, sha256, cache_directory):
        raw = raw.replace(INVALID_XML_BYTE, b"")
        for _, element in ET.iterparse(io.BytesIO(raw), events=("end",)):
            if element.tag != "LexicalEntry":
                continue
            source_id = element.get("val") or element.get("id") or ""
            lemma_element = element.find("./Lemma")
            raw_lemma = (
                direct_feat(lemma_element, "writtenForm") if lemma_element is not None else None
            )
            if source_id and raw_lemma:
                lemma, _ = normalize_headword(raw_lemma)
                related_forms = tuple(
                    sorted(
                        (
                            direct_feat(related, "type") or "",
                            direct_feat(related, "id") or "",
                            normalize_text(direct_feat(related, "writtenForm") or ""),
                        )
                        for related in element.findall("./RelatedForm")
                    )
                )
                entries[source_id] = KrDictEntry(
                    lemma=lemma,
                    pos=direct_feat(element, "partOfSpeech") or "",
                    related_forms=related_forms,
                )
            element.clear()

    related_by_source = krdict_related_adverbs(entries)
    enriched = [
        replace(record, related_adverbs=related_by_source.get(record.source_id, ()))
        for record in records
    ]
    return enriched, sum(len(values) for values in related_by_source.values())


def krdict_related_adverbs(
    entries: dict[str, KrDictEntry],
) -> dict[str, tuple[str, ...]]:
    related_by_source: dict[str, tuple[str, ...]] = {}
    for source_id, source in entries.items():
        if source.pos not in SUPPORTED_POS:
            continue
        surfaces = set()
        for relation_type, target_id, written_form in source.related_forms:
            target = entries.get(target_id)
            if (
                relation_type != "파생어"
                or target is None
                or target.pos != "부사"
                or written_form != target.lemma
            ):
                continue
            reverse_matches = any(
                reverse_id == source_id and reverse_written == source.lemma
                for _, reverse_id, reverse_written in target.related_forms
            )
            if reverse_matches:
                surfaces.add(target.lemma)
        if surfaces:
            related_by_source[source_id] = tuple(sorted(surfaces))
    return related_by_source


def snapshot_members(
    source: str,
    archive_path: Path,
    sha256: str,
    cache_directory: Path | None,
) -> Iterator[tuple[str, bytes]]:
    if cache_directory is None:
        with zipfile.ZipFile(archive_path) as archive:
            for name in sorted(archive.namelist()):
                if name.lower().endswith(".xml"):
                    yield name, archive.read(name)
        return

    target = cache_directory / source / sha256
    marker = target / ".complete"
    expected_marker = f"{archive_path.name}\n{sha256}\n"
    if not marker.is_file() or marker.read_text(encoding="utf-8") != expected_marker:
        extract_snapshot(archive_path, target, expected_marker)
    for path in sorted(target.rglob("*.xml")):
        yield path.relative_to(target).as_posix(), path.read_bytes()


def extract_snapshot(archive_path: Path, target: Path, marker: str) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=f".{target.name}.", dir=target.parent))
    try:
        with zipfile.ZipFile(archive_path) as archive:
            for member in sorted(archive.namelist()):
                if not member.lower().endswith(".xml"):
                    continue
                relative = Path(member)
                if relative.is_absolute() or ".." in relative.parts:
                    raise ValueError(f"unsafe ZIP member: {member}")
                destination = temporary / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                with archive.open(member) as source, destination.open("wb") as output:
                    shutil.copyfileobj(source, output)
        (temporary / ".complete").write_text(marker, encoding="utf-8")
        if target.exists():
            shutil.rmtree(target)
        os.replace(temporary, target)
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_records(path: Path, records: Iterable[PredicateRecord]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(
            (
                "source",
                "source_id",
                "raw_homonym",
                "lemma",
                "pos",
                "lexical_status",
                "conjugations",
                "related_adverbs",
            )
        )
        for record in records:
            writer.writerow(
                (
                    record.source,
                    record.source_id,
                    record.raw_homonym,
                    record.lemma,
                    record.pos,
                    record.lexical_status,
                    "|".join(record.conjugations),
                    "|".join(record.related_adverbs),
                )
            )


def write_stats(path: Path, stats: Iterable[ImportStats]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "schema_version = 2",
        'generator = "tools/nikl-lexicon/import_nikl.py@2"',
        'license = "CC BY-SA 2.0 KR"',
        'extracted_fields = ["source_id", "homonym", "lemma", '
        '"part-of-speech", "conjugation", "related-form-id", "related-form-surface"]',
    ]
    for value in stats:
        lines.extend(
            (
                "",
                "[[source]]",
                f"name = {toml_string(value.source)}",
                f"filename = {toml_string(value.filename)}",
                f"sha256 = {toml_string(value.sha256)}",
                f"item_count = {value.item_count}",
                f"predicate_count = {value.predicate_count}",
                "predicate_with_conjugations_count = "
                f"{value.predicate_with_conjugations_count}",
                f"related_adverb_count = {value.related_adverb_count}",
                f"sanitized_byte_count = {value.sanitized_byte_count}",
                "sanitized_locations = ["
                + ", ".join(toml_string(location) for location in value.sanitized_locations)
                + "]",
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)
