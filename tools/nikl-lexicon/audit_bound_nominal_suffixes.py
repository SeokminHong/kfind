#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

from audit_bound_nominal_prefixes import (
    CatalogRecord,
    STDICT_SHA256,
    catalog_records,
    load_catalog_records,
)


def noun_suffix_candidates(
    records: list[CatalogRecord], source_id: str
) -> tuple[str, dict[str, set[str]]]:
    entries = [record for record in records if record.source_id == source_id]
    if len(entries) != 1:
        raise ValueError(f"접미사 표제어 {source_id}를 하나로 확정할 수 없습니다")
    suffix = entries[0]
    if suffix.tags != ("접사",) or suffix.written != f"-{suffix.word}" or not suffix.origin:
        raise ValueError(f"{source_id}: 원어가 있는 접미사 표제어가 필요합니다")
    independent = [
        record
        for record in records
        if record.written == suffix.word and record.origin == suffix.origin
    ]
    if not independent or any(record.tags != ("의존 명사",) for record in independent):
        raise ValueError(f"{source_id}: 같은 원어의 독립 용법을 의존 명사로 확정할 수 없습니다")

    candidates: dict[str, set[str]] = {}
    competing: set[str] = set()
    for record in records:
        if not record.word.endswith(suffix.word):
            continue
        head = record.word.removesuffix(suffix.word)
        if (
            head
            and record.written == f"{head}-{suffix.word}"
            and record.origin.endswith(suffix.origin)
            and record.tags == ("명사",)
        ):
            candidates.setdefault(record.word, set()).add(record.source_id)
        else:
            competing.add(record.word)
    return suffix.word, {
        word: references for word, references in candidates.items() if word not in competing
    }


def aligned_mecab_nouns(path: Path, suffix: str, candidates: dict[str, set[str]]) -> set[str]:
    aligned = set()
    with path.open(encoding="utf-8", newline="") as source:
        for row in csv.reader(source):
            if (
                len(row) < 12
                or row[0] not in candidates
                or row[4] != "NNG"
                or row[8] != "Compound"
            ):
                continue
            parts = [part.split("/", maxsplit=2) for part in row[11].split("+")]
            if len(parts) < 2 or any(len(part) != 3 for part in parts):
                continue
            if (
                parts[-1][:2] == [suffix, "NNG"]
                and "".join(part[0] for part in parts) == row[0]
            ):
                aligned.add(row[0])
    return aligned


def main() -> None:
    parser = argparse.ArgumentParser(description="사전의 근거와 일치하는 명사 접미사 분절을 추출합니다.")
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--mecab-nouns", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    parser.add_argument("--suffix-id", required=True)
    args = parser.parse_args()
    records = load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item", catalog_records, args.cache_dir
    )
    suffix, candidates = noun_suffix_candidates(records, args.suffix_id)
    aligned = aligned_mecab_nouns(args.mecab_nouns, suffix, candidates)
    output = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    output.writerow(("surface", "suffix", "stdict_ids"))
    for word in sorted(aligned):
        output.writerow((word, suffix, "|".join(sorted(candidates[word]))))


if __name__ == "__main__":
    main()
