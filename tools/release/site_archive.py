#!/usr/bin/env python3

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import mimetypes
import shutil
import tarfile
import tempfile
from pathlib import Path
from typing import Any

from release import parse_version


ARCHIVE_SCHEMA_VERSION = 1
MANIFEST_SCHEMA_VERSION = 1
MAXIMUM_VERSION_COUNT = 100
TEXT_TYPES = {
    ".css": "text/css; charset=utf-8",
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
    ".txt": "text/plain; charset=utf-8",
    ".xml": "application/xml; charset=utf-8",
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def media_type(path: str) -> str:
    suffix = Path(path).suffix.lower()
    if suffix in TEXT_TYPES:
        return TEXT_TYPES[suffix]
    if suffix == ".wasm":
        return "application/wasm"
    if suffix == ".kfc":
        return "application/octet-stream"
    guessed, _ = mimetypes.guess_type(path)
    return guessed or "application/octet-stream"


def archive_files(root: Path, component_resource: Path) -> dict[str, Path]:
    files = {
        path.relative_to(root).as_posix(): path
        for path in root.rglob("*")
        if path.is_file()
    }
    files["api/component-resource"] = component_resource
    return dict(sorted(files.items()))


def write_tar_archive(files: dict[str, Path], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w", format=tarfile.PAX_FORMAT) as archive:
        for archive_path, source_path in files.items():
            info = archive.gettarinfo(str(source_path), arcname=archive_path)
            info.gid = 0
            info.gname = ""
            info.mtime = 0
            info.uid = 0
            info.uname = ""
            info.mode = 0o644
            with source_path.open("rb") as source:
                archive.addfile(info, source)


def write_gzip(source: Path, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as source_file, output.open("wb") as output_file:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output_file, mtime=0) as zipped:
            shutil.copyfileobj(source_file, zipped)


def build_index(version: str, archive: Path) -> dict[str, Any]:
    files: dict[str, dict[str, Any]] = {}
    with tarfile.open(archive, "r") as source:
        for member in source.getmembers():
            if not member.isfile():
                continue
            extracted = source.extractfile(member)
            if extracted is None:
                raise RuntimeError(f"archive member cannot be read: {member.name}")
            digest = hashlib.sha256()
            for chunk in iter(lambda: extracted.read(1024 * 1024), b""):
                digest.update(chunk)
            files[member.name] = {
                "cacheControl": "public, max-age=31536000, immutable",
                "contentType": media_type(member.name),
                "length": member.size,
                "offset": member.offset_data,
                "sha256": digest.hexdigest(),
            }
    return {
        "archiveSha256": sha256_file(archive),
        "archiveSize": archive.stat().st_size,
        "files": files,
        "schemaVersion": ARCHIVE_SCHEMA_VERSION,
        "version": version,
    }


def build_archive(
    version: str,
    root: Path,
    component_resource: Path,
    archive: Path,
    compressed_archive: Path,
    index: Path,
) -> None:
    parse_version(version)
    if not root.is_dir():
        raise RuntimeError(f"site build directory does not exist: {root}")
    if not component_resource.is_file():
        raise RuntimeError(f"component resource does not exist: {component_resource}")

    write_tar_archive(archive_files(root, component_resource), archive)
    index.parent.mkdir(parents=True, exist_ok=True)
    index.write_text(
        json.dumps(build_index(version, archive), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    write_gzip(archive, compressed_archive)


def semver_sort_key(version: str) -> tuple[int, int, int, int, int]:
    major, minor, patch, rc = parse_version(version)
    return (major, minor, patch, 1 if rc is None else 0, rc or 0)


def update_manifest(version: str, existing: Path | None, output: Path) -> None:
    _, _, _, rc = parse_version(version)
    versions: dict[str, dict[str, Any]] = {}
    if existing is not None and existing.is_file():
        value: object = json.loads(existing.read_text(encoding="utf-8"))
        if not isinstance(value, dict) or value.get("schemaVersion") != MANIFEST_SCHEMA_VERSION:
            raise RuntimeError("existing site version manifest is invalid")
        latest = value.get("latest")
        if not isinstance(latest, str):
            raise RuntimeError("existing site version latest is invalid")
        parse_version(latest)
        entries = value.get("versions")
        if (
            not isinstance(entries, list)
            or not entries
            or len(entries) > MAXIMUM_VERSION_COUNT
        ):
            raise RuntimeError("existing site version entries are invalid")
        for entry in entries:
            if not isinstance(entry, dict):
                raise RuntimeError("existing site version entry is invalid")
            entry_version = entry.get("version")
            entry_prerelease = entry.get("prerelease")
            entry_path = entry.get("path")
            if (
                not isinstance(entry_version, str)
                or not isinstance(entry_prerelease, bool)
                or entry_path != f"/versions/{entry_version}"
            ):
                raise RuntimeError("existing site version entry is invalid")
            _, _, _, entry_rc = parse_version(entry_version)
            if entry_prerelease != (entry_rc is not None):
                raise RuntimeError("existing site version prerelease flag is invalid")
            if entry_version in versions:
                raise RuntimeError("existing site version entry is duplicated")
            versions[entry_version] = entry
        expected_latest = max(versions, key=semver_sort_key)
        if latest != expected_latest:
            raise RuntimeError("existing site version latest is invalid")

    if version not in versions and len(versions) >= MAXIMUM_VERSION_COUNT:
        raise RuntimeError("site version manifest exceeds the version limit")
    versions[version] = {
        "path": f"/versions/{version}",
        "prerelease": rc is not None,
        "version": version,
    }
    ordered_versions = sorted(versions, key=semver_sort_key, reverse=True)
    manifest = {
        "latest": ordered_versions[0],
        "schemaVersion": MANIFEST_SCHEMA_VERSION,
        "versions": [versions[item] for item in ordered_versions],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build versioned kfind site artifacts")
    subparsers = parser.add_subparsers(dest="command", required=True)

    archive_parser = subparsers.add_parser("build")
    archive_parser.add_argument("--version", required=True)
    archive_parser.add_argument("--root", type=Path, required=True)
    archive_parser.add_argument("--component-resource", type=Path, required=True)
    archive_parser.add_argument("--archive", type=Path, required=True)
    archive_parser.add_argument("--compressed-archive", type=Path, required=True)
    archive_parser.add_argument("--index", type=Path, required=True)

    manifest_parser = subparsers.add_parser("update-manifest")
    manifest_parser.add_argument("--version", required=True)
    manifest_parser.add_argument("--existing", type=Path)
    manifest_parser.add_argument("--output", type=Path, required=True)
    return parser


def main() -> None:
    arguments = build_parser().parse_args()
    if arguments.command == "build":
        build_archive(
            arguments.version,
            arguments.root,
            arguments.component_resource,
            arguments.archive,
            arguments.compressed_archive,
            arguments.index,
        )
        return
    if arguments.command == "update-manifest":
        update_manifest(arguments.version, arguments.existing, arguments.output)
        return
    raise RuntimeError(f"unsupported command: {arguments.command}")


if __name__ == "__main__":
    main()
