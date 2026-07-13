#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import urllib.request
from pathlib import Path

from dataset import manifest_sources_by_name


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source_file:
        for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fetch(url: str, expected_sha256: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "kfind-benchmark/1"})
    with urllib.request.urlopen(request) as response, temporary.open("wb") as output:
        while chunk := response.read(1024 * 1024):
            output.write(chunk)
    actual_sha256 = sha256(temporary)
    if actual_sha256 != expected_sha256:
        temporary.unlink(missing_ok=True)
        raise ValueError(
            f"SHA-256 mismatch for {url}: expected {expected_sha256}, got {actual_sha256}"
        )
    temporary.replace(destination)


def fetch_manifest(manifest_path: Path, output: Path) -> None:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    for source in manifest_sources_by_name(manifest).values():
        for split in source["splits"].values():
            fetch(
                split["data_url"],
                split["data_sha256"],
                output / split["data_file"],
            )
        fetch(
            source["license_url"],
            source["license_sha256"],
            output / source["license_file"],
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    fetch_manifest(args.manifest, args.output)


if __name__ == "__main__":
    main()
