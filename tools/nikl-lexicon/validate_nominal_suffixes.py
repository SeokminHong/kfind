#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_import import normalize_headword  # noqa: E402
from nikl_nominal_suffixes import validate_catalog  # noqa: E402


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a generated attached nominal suffix catalog."
    )
    parser.add_argument("catalog", type=Path)
    parser.add_argument("--surface", required=True, action="append")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    requested = frozenset(normalize_headword(surface)[0] for surface in args.surface)
    validate_catalog(args.catalog, requested)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
