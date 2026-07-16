#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_endings import (  # noqa: E402
    EndingRecord,
    krdict_endings,
    opendict_endings,
    stdict_endings,
    write_catalog,
)
from nikl_import import (  # noqa: E402
    INVALID_XML_BYTE,
    KRDICT_SHA256,
    OPENDICT_SHA256,
    STDICT_SHA256,
    extract_snapshot,
    file_sha256,
)


class SanitizedReader:
    def __init__(self, source):
        self.source = source

    def read(self, size: int = -1) -> bytes:
        return self.source.read(size).replace(INVALID_XML_BYTE, b"")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Audit NIKL ending headwords.")
    parser.add_argument("--krdict", required=True, type=Path)
    parser.add_argument("--stdict", required=True, type=Path)
    parser.add_argument("--opendict", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cache-dir", required=True, type=Path)
    return parser.parse_args()


def load_records(
    source: str,
    path: Path,
    sha256: str,
    tag: str,
    adapter,
    cache_dir: Path,
) -> list[EndingRecord]:
    actual = file_sha256(path)
    if actual != sha256:
        raise ValueError(f"{source}: expected SHA-256 {sha256}, found {actual}")
    target = cache_dir / source / actual
    marker = target / ".complete"
    expected_marker = f"{path.name}\n{actual}\n"
    if not marker.is_file() or marker.read_text(encoding="utf-8") != expected_marker:
        extract_snapshot(path, target, expected_marker)
    records = []
    for member in sorted(target.rglob("*.xml")):
        with member.open("rb") as raw:
            root = None
            for event, element in ET.iterparse(
                SanitizedReader(raw), events=("start", "end")
            ):
                if root is None:
                    root = element
                if event != "end" or element.tag != tag:
                    continue
                records.extend(adapter(element))
                element.clear()
                if root is not element:
                    root.clear()
    return records


def main() -> int:
    args = parse_args()
    records = []
    for source, path, sha256, tag, adapter in (
        ("krdict", args.krdict, KRDICT_SHA256, "LexicalEntry", krdict_endings),
        ("stdict", args.stdict, STDICT_SHA256, "item", stdict_endings),
        ("opendict", args.opendict, OPENDICT_SHA256, "item", opendict_endings),
    ):
        records.extend(load_records(source, path, sha256, tag, adapter, args.cache_dir))
    write_catalog(args.output, records)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
