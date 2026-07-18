#!/usr/bin/env python3

import argparse
import hashlib
import json
from pathlib import Path


def write_fixture(directory: Path, files: list[tuple[str, bytes]]) -> dict[str, object]:
    directory.mkdir(parents=True, exist_ok=False)
    digest = hashlib.sha256()
    total_bytes = 0
    for name, contents in files:
        (directory / name).write_bytes(contents)
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(contents)
        total_bytes += len(contents)
    return {
        "files": len(files),
        "bytes": total_bytes,
        "sha256": digest.hexdigest(),
    }


def repeated_files(file_count: int, lines_per_file: int) -> list[tuple[str, bytes]]:
    contents = "걸어\n".encode() * lines_per_file
    return [
        (f"file-{file_count - index - 1:04}.txt", contents)
        for index in range(file_count)
    ]


def unique_files(file_count: int, lines_per_file: int) -> list[tuple[str, bytes]]:
    files = []
    for file_index in range(file_count):
        contents = b"".join(
            f"걸어-{file_index:04}-{line_index:05}\n".encode()
            for line_index in range(lines_per_file)
        )
        files.append((f"file-{file_count - file_index - 1:04}.txt", contents))
    return files


def low_hit_files(file_count: int) -> list[tuple[str, bytes]]:
    return [
        (
            f"file-{file_count - index - 1:05}.txt",
            f"다른 줄 {index:05}\n".encode(),
        )
        for index in range(file_count)
    ]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--high-file-count", type=int, default=256)
    parser.add_argument("--lines-per-file", type=int, default=8192)
    parser.add_argument("--low-hit-file-count", type=int, default=8192)
    args = parser.parse_args()

    if min(args.high_file_count, args.lines_per_file, args.low_hit_file_count) < 1:
        parser.error("fixture sizes must be positive")
    if args.output.exists():
        parser.error(f"output already exists: {args.output}")

    metadata = {
        "repeated": write_fixture(
            args.output / "repeated",
            repeated_files(args.high_file_count, args.lines_per_file),
        ),
        "unique": write_fixture(
            args.output / "unique",
            unique_files(args.high_file_count, args.lines_per_file),
        ),
        "low_hit": write_fixture(
            args.output / "low-hit",
            low_hit_files(args.low_hit_file_count),
        ),
    }
    print(json.dumps(metadata, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
