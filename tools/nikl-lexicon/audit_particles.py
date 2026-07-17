#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_import import (  # noqa: E402
    KRDICT_SHA256,
    OPENDICT_SHA256,
    STDICT_SHA256,
)
from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_particles import (  # noqa: E402
    krdict_particles,
    opendict_particles,
    stdict_particles,
    write_catalog,
    write_coverage_report,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Audit NIKL particle headwords.")
    parser.add_argument("--krdict", required=True, type=Path)
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--opendict", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--rules", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    records = []
    for source, path, sha256, tag, adapter in (
        ("krdict", args.krdict, KRDICT_SHA256, "LexicalEntry", krdict_particles),
        ("stdict", args.stdict, STDICT_SHA256, "item", stdict_particles),
        ("opendict", args.opendict, OPENDICT_SHA256, "item", opendict_particles),
    ):
        records.extend(
            load_catalog_records(source, path, sha256, tag, adapter, args.cache_dir)
        )
    write_catalog(args.output, records)
    write_coverage_report(args.report, records, args.rules)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
