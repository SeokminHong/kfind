from __future__ import annotations

import xml.etree.ElementTree as ET
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import TypeVar

from nikl_import import INVALID_XML_BYTE, extract_snapshot, file_sha256


Record = TypeVar("Record")


class SanitizedReader:
    def __init__(self, source):
        self.source = source

    def read(self, size: int = -1) -> bytes:
        return self.source.read(size).replace(INVALID_XML_BYTE, b"")


def load_catalog_records(
    source: str,
    path: Path,
    sha256: str,
    tag: str,
    adapter: Callable[[ET.Element], Iterable[Record]],
    cache_dir: Path,
) -> list[Record]:
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
