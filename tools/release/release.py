#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path
from typing import Iterable


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
VERSION_PATTERN = re.compile(
    r"^(?P<major>0|[1-9][0-9]*)\."
    r"(?P<minor>0|[1-9][0-9]*)\."
    r"(?P<patch>0|[1-9][0-9]*)"
    r"(?:-rc\.(?P<rc>[1-9][0-9]*))?$"
)
STABLE_TAG_PATTERN = re.compile(
    r"^v(?P<major>0|[1-9][0-9]*)\."
    r"(?P<minor>0|[1-9][0-9]*)\."
    r"(?P<patch>0|[1-9][0-9]*)$"
)
CHOCOLATEY_RC_WIDTH = 4
CURRENT_VERSION_FILES = (
    Path("README.md"),
    Path("packages/kfind/README.md"),
    Path("packages/kfind/package.json"),
    Path("site/package.json"),
    Path("site/src/documents/en/guide/getting-started.mdx"),
    Path("site/src/documents/en/guide/installation.mdx"),
    Path("site/src/documents/ko/guide/getting-started.mdx"),
    Path("site/src/documents/ko/guide/installation.mdx"),
    Path("specs/kfind.md"),
)
LOCKFILE_MANIFESTS = (
    Path("Cargo.toml"),
    Path("fuzz/Cargo.toml"),
    Path("tools/morph-compare/runner/Cargo.toml"),
    Path("tools/morph-index-benchmark/Cargo.toml"),
    Path("tools/nikl-lexicon/classifier/Cargo.toml"),
)


def parse_version(value: str) -> tuple[int, int, int, int | None]:
    match = VERSION_PATTERN.fullmatch(value)
    if match is None:
        raise ValueError(f"invalid kfind version: {value}")
    rc = match.group("rc")
    return (
        int(match.group("major")),
        int(match.group("minor")),
        int(match.group("patch")),
        None if rc is None else int(rc),
    )


def chocolatey_version(value: str) -> str:
    major, minor, patch, rc = parse_version(value)
    if rc is None:
        return value
    return f"{major}.{minor}.{patch}-rc{rc:0{CHOCOLATEY_RC_WIDTH}d}"


def workspace_version(repository_root: Path = REPOSITORY_ROOT) -> str:
    cargo_toml = (repository_root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(
        r"(?m)^\[workspace\.package\]\nversion = \"([^\"]+)\"$",
        cargo_toml,
    )
    if match is None:
        raise RuntimeError("workspace package version is missing")
    version = match.group(1)
    parse_version(version)
    return version


def next_version(tags: Iterable[str], bump: str, prerelease: bool) -> str:
    stable_versions = []
    tag_list = list(tags)
    for tag in tag_list:
        match = STABLE_TAG_PATTERN.fullmatch(tag)
        if match is not None:
            stable_versions.append(
                (
                    int(match.group("major")),
                    int(match.group("minor")),
                    int(match.group("patch")),
                )
            )

    major, minor, patch = max(stable_versions, default=(0, 0, 0))
    if bump == "major":
        target = (major + 1, 0, 0)
    elif bump == "minor":
        target = (major, minor + 1, 0)
    elif bump == "patch":
        target = (major, minor, patch + 1)
    else:
        raise ValueError(f"unsupported version bump: {bump}")

    core = ".".join(str(component) for component in target)
    if not prerelease:
        return core

    rc_pattern = re.compile(rf"^v{re.escape(core)}-rc\.([1-9][0-9]*)$")
    rc_versions = [
        int(match.group(1))
        for tag in tag_list
        if (match := rc_pattern.fullmatch(tag)) is not None
    ]
    return f"{core}-rc.{max(rc_versions, default=0) + 1}"


def replace_current_version(path: Path, current: str, target: str) -> None:
    text = path.read_text(encoding="utf-8")
    if current not in text:
        raise RuntimeError(f"current version {current} is missing from {path}")
    path.write_text(text.replace(current, target), encoding="utf-8")


def refresh_lockfiles(repository_root: Path) -> None:
    for relative_manifest in LOCKFILE_MANIFESTS:
        subprocess.run(
            [
                "cargo",
                "update",
                "--workspace",
                "--quiet",
                "--manifest-path",
                str(repository_root / relative_manifest),
            ],
            cwd=repository_root,
            check=True,
            stdout=subprocess.DEVNULL,
        )


def set_version(target: str, repository_root: Path = REPOSITORY_ROOT) -> None:
    parse_version(target)
    current = workspace_version(repository_root)
    if current != target:
        replace_current_version(repository_root / "Cargo.toml", current, target)
        replace_current_version(
            repository_root / "tools/morph-compare/runner/Cargo.toml",
            current,
            target,
        )
        for relative_path in CURRENT_VERSION_FILES:
            replace_current_version(repository_root / relative_path, current, target)
    refresh_lockfiles(repository_root)


def repository_tags(repository_root: Path = REPOSITORY_ROOT) -> list[str]:
    result = subprocess.run(
        ["git", "tag", "--list", "v*"],
        cwd=repository_root,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return result.stdout.splitlines()


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Prepare kfind release versions")
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("current-version")

    next_parser = subparsers.add_parser("next-version")
    next_parser.add_argument("--bump", choices=("major", "minor", "patch"), required=True)
    next_parser.add_argument("--prerelease", action="store_true")

    set_parser = subparsers.add_parser("set-version")
    set_parser.add_argument("version")

    chocolatey_parser = subparsers.add_parser("chocolatey-version")
    chocolatey_parser.add_argument("version")
    return parser


def main() -> None:
    arguments = build_parser().parse_args()
    if arguments.command == "current-version":
        print(workspace_version())
        return
    if arguments.command == "next-version":
        print(next_version(repository_tags(), arguments.bump, arguments.prerelease))
        return
    if arguments.command == "set-version":
        set_version(arguments.version)
        print(arguments.version)
        return
    if arguments.command == "chocolatey-version":
        print(chocolatey_version(arguments.version))
        return
    raise RuntimeError(f"unsupported command: {arguments.command}")


if __name__ == "__main__":
    main()
