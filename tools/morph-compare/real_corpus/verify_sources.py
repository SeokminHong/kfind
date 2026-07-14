#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

from evaluate import load_json, load_jsonl, validate_cases, validate_sources


def raw_url(repository: str, revision: str, source_path: str) -> str:
    prefix = "https://github.com/"
    if not repository.startswith(prefix):
        raise ValueError(f"unsupported repository URL {repository!r}")
    repository_path = repository.removeprefix(prefix).removesuffix(".git")
    encoded_path = "/".join(urllib.parse.quote(part) for part in source_path.split("/"))
    return f"https://raw.githubusercontent.com/{repository_path}/{revision}/{encoded_path}"


def fetch_source(source: dict[str, Any], source_file: dict[str, Any]) -> bytes:
    request = urllib.request.Request(
        raw_url(source["repository"], source["revision"], source_file["path"]),
        headers={"User-Agent": "kfind-real-corpus-verifier"},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def verify_source_bytes(
    source_id: str, source_file: dict[str, Any], source_bytes: bytes
) -> list[str]:
    actual_sha256 = hashlib.sha256(source_bytes).hexdigest()
    if actual_sha256 != source_file["sha256"]:
        raise ValueError(
            f"source file {source_id}:{source_file['path']} SHA-256 mismatch: "
            f"expected {source_file['sha256']}, got {actual_sha256}"
        )
    try:
        return source_bytes.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise ValueError(
            f"source file {source_id}:{source_file['path']} is not UTF-8"
        ) from error


def verify_excerpts(
    cases: list[dict[str, Any]], source_lines: dict[tuple[str, str], list[str]]
) -> None:
    for case in cases:
        lines = source_lines[(case["source_id"], case["source_path"])]
        line_start = case["source_line_start"]
        line_end = case["source_line_end"]
        if line_end > len(lines):
            raise ValueError(f"case {case['id']!r} source line exceeds the pinned file")
        excerpt = "\n".join(lines[line_start - 1 : line_end]).rstrip()
        if excerpt != case["text"].rstrip():
            raise ValueError(f"case {case['id']!r} does not match its pinned source excerpt")


def main() -> None:
    fixture_dir = Path(__file__).parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=fixture_dir / "cases.jsonl")
    parser.add_argument("--sources", type=Path, default=fixture_dir / "sources.json")
    arguments = parser.parse_args()

    manifest = load_json(arguments.sources)
    files_by_key = validate_sources(manifest)
    cases = load_jsonl(arguments.cases)
    validate_cases(cases, files_by_key)

    source_lines = {}
    for source in manifest["sources"]:
        for source_file in source["files"]:
            source_bytes = fetch_source(source, source_file)
            source_lines[(source["id"], source_file["path"])] = verify_source_bytes(
                source["id"], source_file, source_bytes
            )
    verify_excerpts(cases, source_lines)
    print(f"verified {len(source_lines)} source files and {len(cases)} excerpts")


if __name__ == "__main__":
    main()
