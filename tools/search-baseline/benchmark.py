#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import re
import shutil
import statistics
import subprocess
import time
from pathlib import Path
from typing import Any


QUALITY_METHODS = (
    "kfind_any",
    "kfind_smart",
    "regex_enumerated",
    "regex_stem",
)
TIMING_METHODS = (
    "kfind_any",
    "kfind_smart",
    "rg_enumerated",
    "grep_enumerated",
    "rg_stem",
    "grep_stem",
)
CONTRACT_REASONS = {
    "same-pos-homograph",
    "structurally-indistinguishable-homograph",
    "aligned-source-component",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def load_patterns(path: Path) -> list[dict[str, str]]:
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema_version") != 1:
        raise ValueError("patterns schema_version must be 1")
    queries = document.get("queries")
    if not isinstance(queries, list) or len(queries) != 7:
        raise ValueError("patterns must declare exactly 7 queries")

    required = {"id", "query", "enumerated", "stem"}
    normalized: list[dict[str, str]] = []
    seen_ids: set[str] = set()
    for query in queries:
        if not isinstance(query, dict) or set(query) != required:
            raise ValueError(f"invalid pattern record: {query!r}")
        if not all(isinstance(query[key], str) and query[key] for key in required):
            raise ValueError(f"pattern values must be non-empty strings: {query!r}")
        query_id = query["id"]
        if query_id in seen_ids:
            raise ValueError(f"duplicate query id: {query_id}")
        seen_ids.add(query_id)
        normalized.append({key: query[key] for key in required})
    return normalized


def load_fixture(
    path: Path,
    queries: list[dict[str, str]],
) -> list[dict[str, Any]]:
    cases = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if len(cases) != 112:
        raise ValueError(f"fixture must contain exactly 112 cases, got {len(cases)}")

    query_by_id = {query["id"]: query for query in queries}
    required = {
        "id",
        "query_id",
        "query",
        "expected",
        "contract_expected",
        "contract_reason",
        "text",
        "line",
    }
    seen_ids: set[str] = set()
    seen_texts: set[str] = set()
    for index, case in enumerate(cases, start=1):
        if not isinstance(case, dict) or set(case) != required:
            raise ValueError(f"invalid fixture record at line {index}")
        if case["line"] != index:
            raise ValueError(f"fixture line field must be contiguous at {case['id']}")
        if not isinstance(case["expected"], bool) or not isinstance(
            case["contract_expected"], bool
        ):
            raise ValueError(f"fixture expectations must be boolean at {case['id']}")
        if case["id"] in seen_ids:
            raise ValueError(f"duplicate fixture id: {case['id']}")
        if case["text"] in seen_texts:
            raise ValueError(f"duplicate fixture text: {case['text']}")
        seen_ids.add(case["id"])
        seen_texts.add(case["text"])

        query = query_by_id.get(case["query_id"])
        if query is None or case["query"] != query["query"]:
            raise ValueError(f"fixture query mismatch at {case['id']}")
        changed = case["expected"] != case["contract_expected"]
        if changed and case["contract_reason"] not in CONTRACT_REASONS:
            raise ValueError(f"invalid contract reason at {case['id']}")
        if not changed and case["contract_reason"] is not None:
            raise ValueError(f"unchanged case has a contract reason at {case['id']}")

    for query in queries:
        query_cases = [case for case in cases if case["query_id"] == query["id"]]
        positives = sum(1 for case in query_cases if case["expected"])
        if len(query_cases) != 16 or positives != 8:
            raise ValueError(
                f"{query['id']} must contain 8 raw positives and 8 raw negatives"
            )
    return cases


def metrics(
    cases: list[dict[str, Any]],
    predictions: set[int],
    contract: bool,
) -> dict[str, int | float]:
    tp = tn = fp = fn = 0
    expected_key = "contract_expected" if contract else "expected"
    for case in cases:
        expected = bool(case[expected_key])
        predicted = int(case["line"]) in predictions
        if expected and predicted:
            tp += 1
        elif expected:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "cases": len(cases),
        "tp": tp,
        "tn": tn,
        "fp": fp,
        "fn": fn,
        "accuracy_percent": round(100 * (tp + tn) / len(cases), 2),
        "precision_percent": round(100 * precision, 2),
        "recall_percent": round(100 * recall, 2),
        "f1_percent": round(100 * f1, 2),
    }


def command_for(
    method: str,
    query: dict[str, str],
    corpus: Path,
    kfind: Path,
    data_dir: Path,
    rg: str,
    grep: str,
    count: bool,
) -> list[str]:
    if method in {"kfind_any", "kfind_smart"}:
        return [
            str(kfind),
            "--no-pager",
            "--color",
            "never",
            "--no-filename",
            "--data-dir",
            str(data_dir),
            "--boundary",
            method.removeprefix("kfind_"),
            "--count" if count else "--line-number",
            query["query"],
            str(corpus),
        ]

    pattern = query["enumerated" if method.endswith("enumerated") else "stem"]
    if method.startswith("rg"):
        return [
            rg,
            "--no-config",
            "--color",
            "never",
            "--no-filename",
            "--count" if count else "--line-number",
            "-e",
            pattern,
            str(corpus),
        ]
    return [grep, "-E", "-c" if count else "-n", pattern, str(corpus)]


def matching_lines(command: list[str], environment: dict[str, str]) -> set[int]:
    completed = subprocess.run(
        command,
        check=True,
        text=True,
        capture_output=True,
        env=environment,
    )
    lines: set[int] = set()
    for output_line in completed.stdout.splitlines():
        match = re.match(r"^(\d+):", output_line)
        if match:
            lines.add(int(match.group(1)))
    return lines


def tool_version(command: list[str]) -> str:
    completed = subprocess.run(command, check=True, text=True, capture_output=True)
    output = completed.stdout or completed.stderr
    return output.splitlines()[0]


def optional_command(command: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            command,
            check=True,
            text=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return completed.stdout.strip() or None


def memory_bytes() -> int | None:
    value = optional_command(["sysctl", "-n", "hw.memsize"])
    if value and value.isdigit():
        return int(value)
    meminfo = Path("/proc/meminfo")
    if meminfo.exists():
        match = re.search(r"^MemTotal:\s+(\d+)\s+kB$", meminfo.read_text(), re.MULTILINE)
        if match:
            return int(match.group(1)) * 1024
    return None


def render_markdown(report: dict[str, Any]) -> str:
    fixture = report["fixture"]
    performance = report["performance"]
    environment = report["environment"]
    memory = environment["memory_bytes"]
    memory_gib = f"{memory / 1024 / 1024 / 1024:.1f} GiB" if memory else "unavailable"
    lines = [
        "# 형태 질의와 정규식 검색 기준선",
        "",
        "## 범위",
        "",
        "이 보고서는 동일한 7개 형태 질의를 kfind full-POS any/smart와 두 종류의 수동 "
        "정규식으로 실행한 "
        "constructed 진단입니다. Held-out 품질 benchmark나 일반적인 한국어 검색 품질의 "
        "순위로 해석하지 않습니다.",
        "",
        f"- revision: `{report['revision']}`",
        f"- fixture: {fixture['cases']} cases, {fixture['queries']} queries, "
        f"`{fixture['sha256']}`",
        f"- raw: positive {fixture['strict_positive']}, negative "
        f"{fixture['strict_negative']}",
        f"- contract-adjusted: positive {fixture['contract_positive']}, negative "
        f"{fixture['contract_negative']}, reviewed {fixture['reviewed_cases']}",
        f"- performance corpus: {performance['lines']:,} lines, "
        f"{performance['bytes'] / 1024 / 1024:.2f} MiB, "
        f"`{performance['corpus_sha256']}`",
        f"- timing: warm-up {performance['warmup']}회, measured "
        f"{performance['runs']}회, 7개 query fresh-process batch",
        "",
        "## 환경",
        "",
        f"- platform: `{environment['platform']}`",
        f"- CPU: `{environment['cpu']}`, logical CPUs {environment['cpu_count']}",
        f"- memory: {memory_gib}",
        f"- Python: `{environment['python']}`",
        f"- kfind: `{environment['kfind']}`, binary `{environment['kfind_sha256']}`",
        f"- full POS: `{environment['full_pos_sha256']}`",
        f"- component: `{environment['component_sha256']}`",
        f"- ripgrep: `{environment['rg']}`",
        f"- grep: `{environment['grep']}`",
        "",
        "## 전체 품질",
        "",
        "| method | raw TP/TN/FP/FN | raw P/R/F1 | TPᶜ/TNᶜ/FPᶜ/FNᶜ | Pᶜ/Rᶜ/F1ᶜ |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for method in QUALITY_METHODS:
        result = report["quality"][method]
        raw = result["raw"]
        contract = result["contract_adjusted"]
        lines.append(
            f"| {method} | {raw['tp']}/{raw['tn']}/{raw['fp']}/{raw['fn']} | "
            f"{raw['precision_percent']:.2f}/{raw['recall_percent']:.2f}/"
            f"{raw['f1_percent']:.2f}% | "
            f"{contract['tp']}/{contract['tn']}/{contract['fp']}/{contract['fn']} | "
            f"{contract['precision_percent']:.2f}/{contract['recall_percent']:.2f}/"
            f"{contract['f1_percent']:.2f}% |"
        )

    lines.extend(
        [
            "",
            "## Fresh-process batch 시간",
            "",
            "| method | median | min | max | p95 | effective MiB/s |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for method in TIMING_METHODS:
        result = performance["methods"][method]
        lines.append(
            f"| {method} | {result['median_ms']:.2f} ms | "
            f"{result['min_ms']:.2f} ms | {result['max_ms']:.2f} ms | "
            f"{result['p95_ms']:.2f} ms | "
            f"{result['effective_mib_per_second']:.1f} |"
        )

    lines.extend(
        [
            "",
            "## Query별 raw F1",
            "",
        "| query | kfind any | kfind smart | enumerated regex | stem regex |",
        "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for query in report["patterns"]:
        query_id = query["id"]
        lines.append(
            f"| `{query['query']}` | "
            f"{report['quality']['kfind_any']['by_query'][query_id]['raw']['f1_percent']:.2f}% | "
            f"{report['quality']['kfind_smart']['by_query'][query_id]['raw']['f1_percent']:.2f}% | "
            f"{report['quality']['regex_enumerated']['by_query'][query_id]['raw']['f1_percent']:.2f}% | "
            f"{report['quality']['regex_stem']['by_query'][query_id]['raw']['f1_percent']:.2f}% |"
        )

    lines.extend(
        [
            "",
            "## 방법",
            "",
            "- Enumerated regex는 사람이 고른 활용 표면형을 `|`로 열거합니다. Stem regex는 "
            "짧은 어간 후보만 열거합니다. 두 정규식에는 품사 판정이나 token boundary가 없습니다.",
            "- `rg`와 `grep`의 matching line 집합이 같은지 실행 중 검증합니다. 품질은 정규식 "
            "전략별로 합치고 실행시간은 도구별로 분리합니다.",
            "- Kfind는 같은 full-POS resource에서 `boundary=any`와 `boundary=smart`를 "
            "각각 실행하고 품질과 시간을 독립된 행으로 보존합니다.",
            "- Contract-adjusted는 같은 예측을 fixture에 미리 선언한 기대값으로 다시 평가합니다. "
            "다른 검색 모드나 후처리가 아닙니다.",
            "- 각 시간 batch는 7개 질의를 별도 fresh process로 실행해 같은 단일 파일을 7회 "
            "스캔합니다. Matching-line count만 계산하고 stdout은 폐기합니다.",
            "- 품질과 실행시간은 서로 다른 단위이며 하나의 점수로 합치지 않습니다.",
            "",
            "## 재현",
            "",
            "```console",
            "scripts/benchmark-search-baseline.sh",
            "```",
            "",
            "정규식, 실제 executable·입력 경로를 포함한 전체 명령 배열과 case-level failure는 "
            "같은 이름의 JSON 보고서에 보존합니다.",
            "",
        ]
    )
    return "\n".join(lines)


def quality_results(
    queries: list[dict[str, str]],
    cases: list[dict[str, Any]],
    corpus: Path,
    kfind: Path,
    data_dir: Path,
    rg: str,
    grep: str,
    environment: dict[str, str],
) -> dict[str, Any]:
    prediction_commands = {
        "kfind_any": "kfind_any",
        "kfind_smart": "kfind_smart",
        "regex_enumerated": "rg_enumerated",
        "regex_stem": "rg_stem",
    }
    predictions = {method: set() for method in QUALITY_METHODS}
    by_query_predictions: dict[str, dict[str, set[int]]] = {
        method: {} for method in QUALITY_METHODS
    }

    for query in queries:
        query_cases = [case for case in cases if case["query_id"] == query["id"]]
        first_line = int(query_cases[0]["line"])
        last_line = int(query_cases[-1]["line"])
        for method, command_method in prediction_commands.items():
            found = matching_lines(
                command_for(
                    command_method,
                    query,
                    corpus,
                    kfind,
                    data_dir,
                    rg,
                    grep,
                    False,
                ),
                environment,
            )
            scoped = {line for line in found if first_line <= line <= last_line}
            predictions[method].update(scoped)
            by_query_predictions[method][query["id"]] = scoped

        for regex_kind in ("enumerated", "stem"):
            grep_lines = matching_lines(
                command_for(
                    f"grep_{regex_kind}",
                    query,
                    corpus,
                    kfind,
                    data_dir,
                    rg,
                    grep,
                    False,
                ),
                environment,
            )
            rg_lines = matching_lines(
                command_for(
                    f"rg_{regex_kind}",
                    query,
                    corpus,
                    kfind,
                    data_dir,
                    rg,
                    grep,
                    False,
                ),
                environment,
            )
            if grep_lines != rg_lines:
                raise RuntimeError(
                    f"rg/grep {regex_kind} predictions differ for {query['query']}"
                )

    results: dict[str, Any] = {}
    for method in QUALITY_METHODS:
        failures: dict[str, list[dict[str, Any]]] = {
            "raw": [],
            "contract_adjusted": [],
        }
        for case in cases:
            predicted = int(case["line"]) in predictions[method]
            base = {
                "id": case["id"],
                "query": case["query"],
                "text": case["text"],
                "predicted": predicted,
            }
            if predicted != case["expected"]:
                failures["raw"].append({**base, "expected": case["expected"]})
            if predicted != case["contract_expected"]:
                failures["contract_adjusted"].append(
                    {
                        **base,
                        "expected": case["contract_expected"],
                        "contract_reason": case["contract_reason"],
                    }
                )

        results[method] = {
            "raw": metrics(cases, predictions[method], False),
            "contract_adjusted": metrics(cases, predictions[method], True),
            "failures": failures,
            "by_query": {},
        }
        for query in queries:
            query_cases = [case for case in cases if case["query_id"] == query["id"]]
            query_predictions = by_query_predictions[method][query["id"]]
            results[method]["by_query"][query["id"]] = {
                "raw": metrics(query_cases, query_predictions, False),
                "contract_adjusted": metrics(
                    query_cases,
                    query_predictions,
                    True,
                ),
            }
    return results


def performance_results(
    queries: list[dict[str, str]],
    corpus: Path,
    kfind: Path,
    data_dir: Path,
    rg: str,
    grep: str,
    environment: dict[str, str],
    warmup: int,
    runs: int,
) -> dict[str, Any]:
    timings = {method: [] for method in TIMING_METHODS}
    for round_index in range(warmup + runs):
        shift = round_index % len(TIMING_METHODS)
        ordered_methods = TIMING_METHODS[shift:] + TIMING_METHODS[:shift]
        for method in ordered_methods:
            started = time.perf_counter()
            for query in queries:
                subprocess.run(
                    command_for(
                        method,
                        query,
                        corpus,
                        kfind,
                        data_dir,
                        rg,
                        grep,
                        True,
                    ),
                    check=True,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE,
                    env=environment,
                )
            elapsed = time.perf_counter() - started
            if round_index >= warmup:
                timings[method].append(elapsed)

    total_scanned_mib = corpus.stat().st_size * len(queries) / 1024 / 1024
    methods: dict[str, Any] = {}
    for method, values in timings.items():
        median_seconds = statistics.median(values)
        methods[method] = {
            "runs_seconds": values,
            "median_ms": round(1000 * median_seconds, 3),
            "min_ms": round(1000 * min(values), 3),
            "max_ms": round(1000 * max(values), 3),
            "p95_ms": round(1000 * percentile(values, 0.95), 3),
            "effective_mib_per_second": round(total_scanned_mib / median_seconds, 1),
        }
    return methods


def main() -> None:
    script_directory = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=Path, default=script_directory / "fixture.jsonl")
    parser.add_argument("--patterns", type=Path, default=script_directory / "patterns.json")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--kfind", type=Path, default=Path("target/release/kfind"))
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--repetitions", type=int, default=4096)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()

    if min(args.repetitions, args.warmup, args.runs) < 1:
        raise ValueError("repetitions, warmup, and runs must be positive")

    fixture_path = args.fixture.resolve()
    patterns_path = args.patterns.resolve()
    output = args.output.resolve()
    kfind = args.kfind.resolve()
    data_dir = args.data_dir.resolve()
    rg = shutil.which("rg")
    grep = shutil.which("grep")
    if rg is None or grep is None:
        raise RuntimeError("rg and grep must be available")
    for required in (
        kfind,
        data_dir / "lexicon.bin",
        data_dir / "morphology-component-compact.kfc",
    ):
        if not required.is_file():
            raise FileNotFoundError(required)

    queries = load_patterns(patterns_path)
    cases = load_fixture(fixture_path, queries)
    output.mkdir(parents=True, exist_ok=True)
    corpus = output / "fixture.txt"
    performance_corpus = output / "performance-corpus.txt"
    report_path = output / "report.json"
    markdown_path = output / "report.md"
    corpus.write_text(
        "\n".join(str(case["text"]) for case in cases) + "\n",
        encoding="utf-8",
    )
    performance_corpus.write_text(
        corpus.read_text(encoding="utf-8") * args.repetitions,
        encoding="utf-8",
    )

    environment = os.environ.copy()
    environment["LC_ALL"] = "C"
    resource_check = subprocess.run(
        [
            str(kfind),
            "--check-data",
            "--json",
            "--data-dir",
            str(data_dir),
        ],
        check=True,
        text=True,
        capture_output=True,
        env=environment,
    )
    quality = quality_results(
        queries,
        cases,
        corpus,
        kfind,
        data_dir,
        rg,
        grep,
        environment,
    )
    methods = performance_results(
        queries,
        performance_corpus,
        kfind,
        data_dir,
        rg,
        grep,
        environment,
        args.warmup,
        args.runs,
    )

    commands = {
        method: [
            command_for(
                method,
                query,
                performance_corpus,
                kfind,
                data_dir,
                rg,
                grep,
                True,
            )
            for query in queries
        ]
        for method in TIMING_METHODS
    }
    report = {
        "schema_version": 2,
        "revision": args.revision,
        "environment": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "cpu": optional_command(["sysctl", "-n", "machdep.cpu.brand_string"])
            or platform.processor()
            or None,
            "cpu_count": os.cpu_count(),
            "memory_bytes": memory_bytes(),
            "python": platform.python_version(),
            "kfind": tool_version([str(kfind), "--version"]),
            "kfind_sha256": sha256(kfind),
            "full_pos_sha256": sha256(data_dir / "lexicon.bin"),
            "component_sha256": sha256(
                data_dir / "morphology-component-compact.kfc"
            ),
            "kfind_resources": json.loads(resource_check.stdout),
            "rg": tool_version([rg, "--version"]),
            "grep": tool_version([grep, "--version"]),
        },
        "fixture": {
            "kind": "constructed-diagnostic",
            "queries": len(queries),
            "cases": len(cases),
            "strict_positive": sum(1 for case in cases if case["expected"]),
            "strict_negative": sum(1 for case in cases if not case["expected"]),
            "contract_positive": sum(
                1 for case in cases if case["contract_expected"]
            ),
            "contract_negative": sum(
                1 for case in cases if not case["contract_expected"]
            ),
            "reviewed_cases": sum(
                1
                for case in cases
                if case["contract_expected"] != case["expected"]
            ),
            "review_reasons": {
                reason: sum(
                    1 for case in cases if case["contract_reason"] == reason
                )
                for reason in sorted(CONTRACT_REASONS)
            },
            "sha256": sha256(fixture_path),
            "patterns_sha256": sha256(patterns_path),
            "corpus_sha256": sha256(corpus),
        },
        "patterns": queries,
        "quality": quality,
        "performance": {
            "corpus_sha256": sha256(performance_corpus),
            "bytes": performance_corpus.stat().st_size,
            "lines": len(cases) * args.repetitions,
            "repetitions": args.repetitions,
            "warmup": args.warmup,
            "runs": args.runs,
            "workload": f"{len(queries)} independent fresh-process queries per batch",
            "methods": methods,
        },
        "commands": commands,
    }
    report_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    markdown_path.write_text(render_markdown(report), encoding="utf-8")
    print(f"search baseline report: {report_path}")
    print(f"search baseline markdown: {markdown_path}")


if __name__ == "__main__":
    main()
