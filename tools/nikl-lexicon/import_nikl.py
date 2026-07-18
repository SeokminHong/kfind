#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import sys
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_import import (  # noqa: E402
    KRDICT_INVALID_BYTE_LOCATIONS,
    KRDICT_SHA256,
    OPENDICT_SHA256,
    STDICT_SHA256,
    attach_attested_adverbials,
    attach_krdict_relations,
    import_snapshot,
    krdict_record,
    opendict_record,
    stdict_record,
    write_records,
    write_stats,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Normalize NIKL dictionary snapshots.")
    parser.add_argument("--krdict", required=True, type=Path)
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--opendict", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--stats", required=True, type=Path)
    parser.add_argument("--cache-dir", type=Path, default=default_cache_directory())
    return parser.parse_args()


def default_cache_directory() -> Path:
    if configured := os.environ.get("KFIND_NIKL_CACHE"):
        return Path(configured)
    cache_home = os.environ.get("XDG_CACHE_HOME")
    return Path(cache_home) / "kfind/nikl" if cache_home else Path.home() / ".cache/kfind/nikl"


def main() -> int:
    args = parse_args()
    sources = (
        (
            "krdict",
            args.krdict,
            "LexicalEntry",
            krdict_record,
            7,
            KRDICT_INVALID_BYTE_LOCATIONS,
            KRDICT_SHA256,
        ),
        ("stdict", args.stdict, "item", stdict_record, 0, (), STDICT_SHA256),
        ("opendict", args.opendict, "item", opendict_record, 0, (), OPENDICT_SHA256),
    )
    records = []
    stats = []
    for (
        source,
        path,
        element_tag,
        adapter,
        expected_invalid_bytes,
        expected_invalid_locations,
        expected_sha256,
    ) in sources:
        imported, source_stats = import_snapshot(
            source,
            path,
            element_tag,
            adapter,
            expected_invalid_bytes,
            expected_invalid_locations,
            expected_sha256,
            args.cache_dir,
        )
        if source == "krdict":
            imported, related_adverb_count, related_voice_derivation_count = attach_krdict_relations(
                imported, path, args.cache_dir
            )
            source_stats = replace(
                source_stats,
                related_adverb_count=related_adverb_count,
                related_voice_derivation_count=related_voice_derivation_count,
            )
        imported, attested_adverbial_count = attach_attested_adverbials(
            imported, source, path, args.cache_dir
        )
        source_stats = replace(
            source_stats, attested_adverbial_count=attested_adverbial_count
        )
        records.extend(imported)
        stats.append(source_stats)
    write_records(args.output, sorted(records))
    write_stats(args.stats, stats)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
