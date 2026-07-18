#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import (  # noqa: E402
    KRDICT_SHA256,
    OPENDICT_SHA256,
    STDICT_SHA256,
    normalize_headword,
)
from nikl_lexemes import krdict_lexemes, opendict_lexemes, stdict_lexemes  # noqa: E402
from nikl_nominal_suffixes import (  # noqa: E402
    attached_nominal_suffixes,
    write_catalog,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Audit reviewed NIKL suffix headwords used by nominal validation."
    )
    parser.add_argument("--krdict", required=True, type=Path)
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--opendict", required=True, type=Path)
    parser.add_argument("--surface", required=True, action="append")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    requested = frozenset(normalize_headword(surface)[0] for surface in args.surface)
    records = []
    for source, path, sha256, tag, adapter in (
        ("krdict", args.krdict, KRDICT_SHA256, "LexicalEntry", krdict_lexemes),
        ("stdict", args.stdict, STDICT_SHA256, "item", stdict_lexemes),
        ("opendict", args.opendict, OPENDICT_SHA256, "item", opendict_lexemes),
    ):
        records.extend(
            load_catalog_records(
                source,
                path,
                sha256,
                tag,
                lambda element, adapter=adapter: adapter(element, requested),
                args.cache_dir,
            )
        )
    write_catalog(args.output, requested, attached_nominal_suffixes(records, requested))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
