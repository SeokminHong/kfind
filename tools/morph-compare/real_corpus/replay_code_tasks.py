#!/usr/bin/env python3
"""코드 작업 pilot의 고정 입력, patch와 원본 검증을 재현한다."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path

from verify_sources import fetch_source, verify_source_bytes


def command(argv, cwd, **kwargs):
    return subprocess.run(
        argv, cwd=cwd, capture_output=True, timeout=120, check=False, **kwargs
    )


def relative_path(value):
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"입력 밖의 경로: {value}")
    return path


def verify_preserved(original, changed):
    marker = (
        "#[cfg(test)]" if "#[cfg(test)]" in original else "    if results.len() != 10 {"
    )
    if (
        marker not in changed
        or original.split(marker, 1)[1] != changed.split(marker, 1)[1]
    ):
        raise ValueError("원본 테스트 또는 완료 검증이 변경되었습니다")


def verify_threads(output):
    done = re.findall(r"^Thread (\d+) done$", output, re.MULTILINE)
    took = re.findall(r"^Thread (\d+) took (\d+)ms$", output, re.MULTILINE)
    if (
        sorted(map(int, done)) != list(range(10))
        or sorted(int(index) for index, _ in took) != list(range(10))
        or not all(int(duration) >= 250 for _, duration in took)
    ):
        raise ValueError("스레드 완료·반환값 검증 실패")


def replay(report, source_cache, work_root):
    source_files = {}
    for source in report["source_manifest"]["sources"]:
        for file in source["files"]:
            relative = relative_path(source["id"]) / relative_path(file["path"])
            path = source_cache / relative
            data = path.read_bytes() if path.exists() else fetch_source(source, file)
            verify_source_bytes(source["id"], file, data)
            source_files[str(relative)] = data
    results = []
    for trial in report["trials"]:
        with tempfile.TemporaryDirectory(
            prefix="code-task-", dir=work_root
        ) as temporary:
            directory = Path(temporary)
            inputs = directory / "input"
            for relative, data in source_files.items():
                path = inputs / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            if len(trial["patches"]) != 1:
                raise ValueError("각 pilot 과제는 하나의 source 파일만 수정합니다")
            patch = trial["patches"][0]
            relative = relative_path(patch["path"])
            original = source_files[str(relative)].decode()
            source_path = inputs / relative
            threaded = trial["trial"].startswith("threads-")
            argv = ["rustc", "--edition=2021"]
            if not threaded:
                argv.append("--test")
            argv += [str(source_path), "-o", str(directory / "check")]
            before = command(argv, inputs)
            if before.returncode == 0:
                before = command([str(directory / "check")], inputs)
            if before.returncode == 0:
                raise ValueError("수정 전 과제가 이미 검증을 통과합니다")
            initialized = command(["git", "init", "--quiet"], inputs)
            if initialized.returncode:
                raise RuntimeError(initialized.stderr.decode())
            applied = command(
                ["git", "apply", "-"], inputs, input=patch["diff"].encode()
            )
            if applied.returncode:
                raise RuntimeError(applied.stderr.decode())
            changed = source_path.read_text()
            verify_preserved(original, changed)
            if hashlib.sha256(changed.encode()).hexdigest() != patch["sha256"]:
                raise ValueError("수정 결과 checksum 불일치")
            for other, data in source_files.items():
                if other != str(relative) and (inputs / other).read_bytes() != data:
                    raise ValueError("과제 밖의 입력이 변경되었습니다")
            compiled = command(argv, inputs)
            if compiled.returncode:
                raise RuntimeError(compiled.stderr.decode())
            verified = command([str(directory / "check")], inputs)
            if verified.returncode:
                raise RuntimeError(verified.stdout.decode() + verified.stderr.decode())
            if threaded:
                verify_threads(verified.stdout.decode())
            results.append({"trial": trial["trial"], "replay_passed": True})
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--source-cache", type=Path, required=True)
    parser.add_argument("--work-root", type=Path, required=True)
    args = parser.parse_args()
    args.work_root.mkdir(parents=True, exist_ok=True)
    report = json.loads(args.report.read_text())
    print(json.dumps(replay(report, args.source_cache, args.work_root), indent=2))


if __name__ == "__main__":
    main()
