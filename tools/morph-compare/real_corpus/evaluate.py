#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


REQUIRED_ARTIFACT_TYPES = {"readme", "source-comment", "technical-doc"}
REQUIRED_SLICES = {
    "identifier-adjacent",
    "spacing-error",
    "mixed-script-number",
    "homonym",
    "compound-substring",
}
PROFILE_CONTRACTS = {
    "agent": {
        "lexicon": "embedded",
        "boundary": "any",
        "query_mode": "explicit-pos",
    },
    "user": {
        "lexicon": "full-pos",
        "boundary": "smart",
        "query_mode": "untagged",
    },
}
SUPPORTED_POS = {
    "noun",
    "verb",
    "adjective",
    "adverb",
    "pronoun",
    "determiner",
    "numeral",
}
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}")
REVISION_PATTERN = re.compile(r"[0-9a-f]{40}")


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as source_file:
        value = json.load(source_file)
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    cases = []
    with path.open(encoding="utf-8") as cases_file:
        for line_number, line in enumerate(cases_file, start=1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number} must contain a JSON object")
            cases.append(value)
    return cases


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source_file:
        for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_text(text: str) -> str:
    return " ".join(unicodedata.normalize("NFC", text).split())


def validate_sources(manifest: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    if manifest.get("schema_version") != 1:
        raise ValueError("source manifest schema_version must be 1")
    sources = manifest.get("sources")
    if not isinstance(sources, list) or not sources:
        raise ValueError("source manifest must contain sources")

    files_by_key: dict[tuple[str, str], dict[str, Any]] = {}
    source_ids = set()
    for source in sources:
        source_id = source.get("id")
        if not isinstance(source_id, str) or not source_id:
            raise ValueError("every source must have an id")
        if source_id in source_ids:
            raise ValueError(f"duplicate source id {source_id!r}")
        source_ids.add(source_id)
        if not REVISION_PATTERN.fullmatch(str(source.get("revision", ""))):
            raise ValueError(f"source {source_id!r} must pin a 40-character revision")
        for field in ("repository", "license", "license_url"):
            if not isinstance(source.get(field), str) or not source[field]:
                raise ValueError(f"source {source_id!r} must define {field}")

        files = source.get("files")
        if not isinstance(files, list) or not files:
            raise ValueError(f"source {source_id!r} must contain files")
        for source_file in files:
            source_path = source_file.get("path")
            key = (source_id, source_path)
            if not isinstance(source_path, str) or not source_path:
                raise ValueError(f"source {source_id!r} contains an invalid path")
            if key in files_by_key:
                raise ValueError(f"duplicate source file {source_id}:{source_path}")
            if not SHA256_PATTERN.fullmatch(str(source_file.get("sha256", ""))):
                raise ValueError(f"source file {source_id}:{source_path} has an invalid SHA-256")
            if not isinstance(source_file.get("url"), str) or not source_file["url"]:
                raise ValueError(f"source file {source_id}:{source_path} must define url")
            files_by_key[key] = source_file
    return files_by_key


def validate_cases(
    cases: list[dict[str, Any]], files_by_key: dict[tuple[str, str], dict[str, Any]]
) -> None:
    if not cases:
        raise ValueError("fixture must contain cases")
    ids = set()
    canonical_texts = set()
    artifact_types = set()
    slices = set()
    for case in cases:
        case_id = case.get("id")
        if not isinstance(case_id, str) or not case_id:
            raise ValueError("every case must have an id")
        if case_id in ids:
            raise ValueError(f"duplicate case id {case_id!r}")
        ids.add(case_id)

        source_key = (case.get("source_id"), case.get("source_path"))
        if source_key not in files_by_key:
            raise ValueError(f"case {case_id!r} references an unknown source file")
        line_start = case.get("source_line_start")
        line_end = case.get("source_line_end")
        if (
            not isinstance(line_start, int)
            or isinstance(line_start, bool)
            or not isinstance(line_end, int)
            or isinstance(line_end, bool)
            or line_start < 1
            or line_end < line_start
        ):
            raise ValueError(f"case {case_id!r} has an invalid source line range")

        artifact_type = case.get("artifact_type")
        if artifact_type not in REQUIRED_ARTIFACT_TYPES:
            raise ValueError(f"case {case_id!r} has an invalid artifact_type")
        artifact_types.add(artifact_type)
        case_slice = case.get("slice")
        if case_slice not in REQUIRED_SLICES:
            raise ValueError(f"case {case_id!r} has an invalid slice")
        slices.add(case_slice)
        if case.get("pos") not in SUPPORTED_POS:
            raise ValueError(f"case {case_id!r} has an invalid POS")
        if not isinstance(case.get("query"), str) or not case["query"]:
            raise ValueError(f"case {case_id!r} must define query")

        text = case.get("text")
        if not isinstance(text, str) or not text:
            raise ValueError(f"case {case_id!r} must define text")
        canonical = canonical_text(text)
        if canonical in canonical_texts:
            raise ValueError(f"case {case_id!r} duplicates canonical text")
        canonical_texts.add(canonical)

        expected = case.get("expected")
        if not isinstance(expected, bool):
            raise ValueError(f"case {case_id!r} must define expected as a boolean")
        gold_fields = (
            case.get("gold_text"),
            case.get("gold_byte_start"),
            case.get("gold_byte_end"),
        )
        if expected:
            validate_gold(case_id, text, *gold_fields)
        elif gold_fields != (None, None, None):
            raise ValueError(f"negative case {case_id!r} must not define a gold span")

    if artifact_types != REQUIRED_ARTIFACT_TYPES:
        missing = sorted(REQUIRED_ARTIFACT_TYPES - artifact_types)
        raise ValueError(f"fixture is missing artifact types: {missing}")
    if slices != REQUIRED_SLICES:
        missing = sorted(REQUIRED_SLICES - slices)
        raise ValueError(f"fixture is missing slices: {missing}")


def validate_gold(
    case_id: str,
    text: str,
    gold_text: Any,
    gold_byte_start: Any,
    gold_byte_end: Any,
) -> None:
    if not isinstance(gold_text, str) or not gold_text:
        raise ValueError(f"positive case {case_id!r} must define gold_text")
    if (
        not isinstance(gold_byte_start, int)
        or isinstance(gold_byte_start, bool)
        or not isinstance(gold_byte_end, int)
        or isinstance(gold_byte_end, bool)
        or gold_byte_start < 0
        or gold_byte_end <= gold_byte_start
    ):
        raise ValueError(f"positive case {case_id!r} has an invalid gold span")
    text_bytes = text.encode("utf-8")
    if gold_byte_end > len(text_bytes):
        raise ValueError(f"positive case {case_id!r} gold span exceeds text")
    try:
        actual = text_bytes[gold_byte_start:gold_byte_end].decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"positive case {case_id!r} gold span splits UTF-8") from error
    if actual != gold_text:
        raise ValueError(
            f"positive case {case_id!r} gold span selects {actual!r}, expected {gold_text!r}"
        )


def parse_profiles(values: Iterable[str]) -> dict[str, Path]:
    profiles = {}
    for value in values:
        name, separator, path = value.partition("=")
        if not separator or not name or not path:
            raise ValueError("--profile must use NAME=PATH")
        if name in profiles:
            raise ValueError(f"duplicate profile {name!r}")
        profiles[name] = Path(path)
    if not profiles:
        raise ValueError("at least one --profile is required")
    return profiles


def validate_profile_results(
    profile_name: str, profile: dict[str, Any], cases: list[dict[str, Any]]
) -> None:
    results = profile.get("results")
    if not isinstance(results, list):
        raise ValueError(f"profile {profile_name!r} must contain results")
    expected_ids = [case["id"] for case in cases]
    actual_ids = [result.get("id") for result in results]
    if actual_ids != expected_ids:
        raise ValueError(f"profile {profile_name!r} result ids do not match fixture order")
    for case, result in zip(cases, results):
        spans = result.get("spans")
        if not isinstance(spans, list):
            raise ValueError(f"profile {profile_name!r} case {case['id']!r} has invalid spans")
        text_bytes = case["text"].encode("utf-8")
        for span in spans:
            start = span.get("byte_start")
            end = span.get("byte_end")
            if (
                not isinstance(start, int)
                or isinstance(start, bool)
                or not isinstance(end, int)
                or isinstance(end, bool)
                or start < 0
                or end <= start
                or end > len(text_bytes)
            ):
                raise ValueError(
                    f"profile {profile_name!r} case {case['id']!r} has an invalid span"
                )
            try:
                text_bytes[start:end].decode("utf-8")
            except UnicodeDecodeError as error:
                raise ValueError(
                    f"profile {profile_name!r} case {case['id']!r} span splits UTF-8"
                ) from error


def empty_confusion() -> Counter[str]:
    return Counter({"tp": 0, "fp": 0, "tn": 0, "fn": 0})


def classify(case: dict[str, Any], spans: list[dict[str, int]]) -> str:
    if not case["expected"]:
        return "fp" if spans else "tn"
    gold_start = case["gold_byte_start"]
    gold_end = case["gold_byte_end"]
    overlaps = any(
        span["byte_start"] < gold_end and span["byte_end"] > gold_start for span in spans
    )
    return "tp" if overlaps else "fn"


def with_metrics(confusion: Counter[str]) -> dict[str, Any]:
    values = {key: confusion[key] for key in ("tp", "fp", "tn", "fn")}
    precision_denominator = values["tp"] + values["fp"]
    recall_denominator = values["tp"] + values["fn"]
    precision = values["tp"] / precision_denominator if precision_denominator else None
    recall = values["tp"] / recall_denominator if recall_denominator else None
    f1 = (
        2 * precision * recall / (precision + recall)
        if precision is not None and recall is not None and precision + recall
        else None
    )
    return {**values, "precision": precision, "recall": recall, "f1": f1}


def evaluate_profile(
    profile_name: str, profile: dict[str, Any], cases: list[dict[str, Any]]
) -> dict[str, Any]:
    validate_profile_results(profile_name, profile, cases)
    contract = PROFILE_CONTRACTS.get(profile_name)
    if contract is not None and (
        profile.get("profile") != contract["lexicon"]
        or profile.get("boundary") != contract["boundary"]
    ):
        raise ValueError(f"profile {profile_name!r} does not match its product contract")
    overall = empty_confusion()
    by_artifact: defaultdict[str, Counter[str]] = defaultdict(empty_confusion)
    by_slice: defaultdict[str, Counter[str]] = defaultdict(empty_confusion)
    failures = []
    for case, result in zip(cases, profile["results"]):
        classification = classify(case, result["spans"])
        overall[classification] += 1
        by_artifact[case["artifact_type"]][classification] += 1
        by_slice[case["slice"]][classification] += 1
        if classification in {"fp", "fn"}:
            failures.append(
                {
                    "id": case["id"],
                    "classification": classification,
                    "artifact_type": case["artifact_type"],
                    "slice": case["slice"],
                    "query": case["query"],
                    "pos": case["pos"],
                    "spans": result["spans"],
                }
            )
    runner = {key: value for key, value in profile.items() if key != "results"}
    return {
        "contract": contract,
        "runner": runner,
        "overall": with_metrics(overall),
        "by_artifact_type": {
            key: with_metrics(value) for key, value in sorted(by_artifact.items())
        },
        "by_slice": {key: with_metrics(value) for key, value in sorted(by_slice.items())},
        "failures": failures,
    }


def build_report(
    revision: str,
    cases_path: Path,
    sources_path: Path,
    manifest: dict[str, Any],
    cases: list[dict[str, Any]],
    profiles: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    if not REVISION_PATTERN.fullmatch(revision):
        raise ValueError("revision must be a 40-character Git commit")
    artifact_counts = Counter(case["artifact_type"] for case in cases)
    slice_counts = Counter(case["slice"] for case in cases)
    return {
        "schema_version": 1,
        "revision": revision,
        "fixture": {
            "cases_sha256": sha256(cases_path),
            "sources_sha256": sha256(sources_path),
            "case_count": len(cases),
            "positive_count": sum(case["expected"] for case in cases),
            "negative_count": sum(not case["expected"] for case in cases),
            "artifact_type_counts": dict(sorted(artifact_counts.items())),
            "slice_counts": dict(sorted(slice_counts.items())),
        },
        "sources": manifest["sources"],
        "interpretation": {
            "purpose": "blind diagnostic fixture",
            "balanced": False,
            "product_rule_selection": False,
            "replaces_ud_regression": False,
        },
        "profiles": profiles,
    }


def format_percentage(value: float | None) -> str:
    if value is None or not math.isfinite(value):
        return "—"
    return f"{value * 100:.2f}%"


def metric_row(label: str, metrics: dict[str, Any]) -> str:
    return (
        f"| {label} | {metrics['tp']} | {metrics['fp']} | {metrics['tn']} | "
        f"{metrics['fn']} | {format_percentage(metrics['precision'])} | "
        f"{format_percentage(metrics['recall'])} | {format_percentage(metrics['f1'])} |"
    )


def render_markdown(report: dict[str, Any]) -> str:
    fixture = report["fixture"]
    lines = [
        "# 현실 기술 코퍼스 blind 평가",
        "",
        f"- revision: `{report['revision']}`",
        f"- fixture SHA-256: `{fixture['cases_sha256']}`",
        f"- source manifest SHA-256: `{fixture['sources_sha256']}`",
        f"- cases: {fixture['case_count']} (positive {fixture['positive_count']}, negative {fixture['negative_count']})",
        "",
        "query와 gold span은 제품 실행 전에 고정했다. 25건의 불균형 진단 fixture이므로 제품 전체 품질 점수나 profile 순위로 해석하지 않는다.",
        "이 평가는 기존 UD 회귀 fixture를 대체하거나 규칙 선택에 사용하지 않는다.",
        "",
        "## 출처",
        "",
        "| source | revision | license | files |",
        "| --- | --- | --- | ---: |",
    ]
    for source in report["sources"]:
        lines.append(
            f"| [{source['id']}]({source['repository']}) | `{source['revision'][:7]}` | "
            f"[{source['license']}]({source['license_url']}) | {len(source['files'])} |"
        )

    lines.extend(
        [
            "",
            "## 전체 결과",
            "",
            "Agent는 `embedded + any + explicit POS`, User는 `full-POS + smart + untagged`다.",
            "",
            "| profile | TP | FP | TN | FN | precision | recall | F1 |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name, profile in report["profiles"].items():
        lines.append(metric_row(name, profile["overall"]))

    lines.extend(
        [
            "",
            "## slice",
            "",
            "| profile / slice | TP | FP | TN | FN | precision | recall | F1 |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name, profile in report["profiles"].items():
        for slice_name, metrics in profile["by_slice"].items():
            lines.append(metric_row(f"{name} / {slice_name}", metrics))

    lines.extend(
        [
            "",
            "## 원문 유형",
            "",
            "| profile / artifact | TP | FP | TN | FN | precision | recall | F1 |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name, profile in report["profiles"].items():
        for artifact_type, metrics in profile["by_artifact_type"].items():
            lines.append(metric_row(f"{name} / {artifact_type}", metrics))

    lines.extend(["", "## 실패 case", ""])
    failures = [
        (name, failure)
        for name, profile in report["profiles"].items()
        for failure in profile["failures"]
    ]
    if failures:
        lines.extend(
            [
                "| profile | case | 분류 | slice | query | POS |",
                "| --- | --- | --- | --- | --- | --- |",
            ]
        )
        for name, failure in failures:
            lines.append(
                f"| {name} | `{failure['id']}` | {failure['classification'].upper()} | "
                f"{failure['slice']} | `{failure['query']}` | {failure['pos']} |"
            )
    else:
        lines.append("없음.")
    lines.append("")
    return "\n".join(lines)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as output_file:
        json.dump(value, output_file, ensure_ascii=False, indent=2)
        output_file.write("\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--profile", action="append", default=[])
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-markdown", type=Path, required=True)
    arguments = parser.parse_args()

    manifest = load_json(arguments.sources)
    files_by_key = validate_sources(manifest)
    cases = load_jsonl(arguments.cases)
    validate_cases(cases, files_by_key)
    profile_paths = parse_profiles(arguments.profile)
    profiles = {
        name: evaluate_profile(name, load_json(path), cases)
        for name, path in profile_paths.items()
    }
    report = build_report(
        arguments.revision,
        arguments.cases,
        arguments.sources,
        manifest,
        cases,
        profiles,
    )
    write_json(arguments.output_json, report)
    arguments.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    arguments.output_markdown.write_text(render_markdown(report), encoding="utf-8")


if __name__ == "__main__":
    main()
