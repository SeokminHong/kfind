#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
from pathlib import Path

from benchmark import aggregate_performance, performance


ARTIFACTS = (
    ("KFIND_FULL_POS_LEXICON", "lexicon_artifact_sha256"),
    ("KFIND_COMPONENT_RESOURCE", "component_artifact_sha256"),
    ("KFIND_ENRICHED_PREDICATES", "enriched_artifact_sha256"),
)


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="같은 입력에서 Smart 기준선과 후보의 성능을 교대 측정합니다."
    )
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path, help="측정 결과를 보존할 새 디렉터리")
    for arm in ("baseline", "candidate"):
        parser.add_argument(
            f"--{arm}",
            nargs=5,
            required=True,
            metavar=("RUNNER", "REVISION", "FULL_POS", "COMPONENT", "ENRICHED"),
        )
    args = parser.parse_args()
    system = platform.system()
    if system not in ("Darwin", "Linux"):
        parser.error("macOS와 Linux에서 최대 RSS를 측정할 수 있습니다.")

    source = args.input.resolve(strict=True)
    source_sha256 = sha256(source)
    with source.open(encoding="utf-8") as fixture:
        case_ids = [json.loads(line)["id"] for line in fixture if line.strip()]
    if not case_ids:
        parser.error("입력에 측정할 질의가 없습니다.")
    configurations = {}
    for arm in ("baseline", "candidate"):
        runner, revision, *resources = getattr(args, arm)
        binary = Path(runner).resolve(strict=True)
        paths = [Path(resource).resolve(strict=True) for resource in resources]
        environment = os.environ.copy()
        artifacts = {}
        for (key, field), path in zip(ARTIFACTS, paths, strict=True):
            environment[key] = str(path)
            artifacts[field] = sha256(path)
        configurations[arm] = binary, revision, environment, artifacts, sha256(binary)

    output = args.output.resolve()
    output.mkdir()
    measured = {"baseline": [], "candidate": []}
    span_hashes = {}
    for index in range(6):
        order = ("baseline", "candidate") if index % 2 else ("candidate", "baseline")
        for arm in order:
            binary, _, environment, artifacts, _ = configurations[arm]
            result = output / f"{arm}-{index}.json"
            log = output / f"{arm}-{index}.log"
            command = [str(binary), "boundary", "full-pos", "smart", str(source), str(result)]
            if system == "Darwin":
                command = ["/usr/bin/time", "-l", *command]
            with log.open("w", encoding="utf-8") as stderr:
                subprocess.run(
                    command, env={**environment, "LC_ALL": "C"}, stderr=stderr, check=True
                )
            payload = json.loads(result.read_text(encoding="utf-8"))
            if payload.get("fixture_sha256") != source_sha256:
                raise ValueError(f"{result}: 실행 입력 체크섬이 없거나 측정 입력과 다릅니다.")
            if (
                (payload["profile"], payload["boundary"]) != ("full-pos", "smart")
                or any(payload[key] != expected for key, expected in artifacts.items())
                or [case["id"] for case in payload["results"]] != case_ids
            ):
                raise ValueError(f"{result}: 실행 모드, 리소스 또는 질의 순서 불일치")
            spans = [case["spans"] for case in payload["results"]]
            span_sha256 = hashlib.sha256(json.dumps(spans, sort_keys=True).encode()).hexdigest()
            if span_hashes.setdefault(arm, span_sha256) != span_sha256:
                raise ValueError(f"{result}: 같은 측정군에서 반환 범위가 달라졌습니다.")
            if system == "Darwin":
                rss = re.search(
                    r"^\s*(\d+)\s+maximum resident set size\s*$",
                    log.read_text(encoding="utf-8"),
                    re.M,
                )
                if rss:
                    payload["peak_rss_kib"] = int(rss[1]) / 1024
            if not payload["peak_rss_kib"]:
                raise ValueError(f"{result}: 최대 RSS를 찾을 수 없습니다.")
            if index:
                latencies = [case["latency_ms"] for case in payload["results"]]
                measured[arm].append(performance(payload, latencies))
    if source_sha256 != sha256(source):
        raise ValueError(f"{source}: 측정 중 입력이 변경됐습니다.")

    report = {
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "rss_source": "/usr/bin/time -l" if system == "Darwin" else "/proc/self/status VmHWM",
        },
        "input": {"path": str(source), "sha256": source_sha256, "rows": len(case_ids)},
        "runner_command": "boundary full-pos smart INPUT OUTPUT",
        "warmup_index": 0,
    }
    for arm, (_, revision, _, artifacts, binary_sha256) in configurations.items():
        report[arm] = {
            "revision": revision,
            "runner_sha256": binary_sha256,
            "resources": artifacts,
            "span_sha256": span_hashes[arm],
            "performance": aggregate_performance(measured[arm], warmup_runs=1),
        }
    (output / "summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
