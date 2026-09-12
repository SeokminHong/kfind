#!/usr/bin/env python3
"""고정 기술 문서에서 정답 위치 발견과 후보 검토량을 측정한다."""

from __future__ import annotations

import argparse
import json
import platform
import shutil
import statistics
import subprocess
import time
from pathlib import Path

from evaluate import (
    classify,
    empty_confusion,
    load_json,
    load_jsonl,
    sha256,
    validate_cases,
    validate_sources,
    with_metrics,
)
from verify_sources import fetch_source, verify_excerpts, verify_source_bytes


def prepare_sources(manifest, cases, cache):
    paths, lines = {}, {}
    for source in manifest["sources"]:
        paths[source["id"]] = []
        for file in source["files"]:
            path = cache / source["id"] / file["path"]
            data = path.read_bytes() if path.exists() else fetch_source(source, file)
            lines[(source["id"], file["path"])] = verify_source_bytes(
                source["id"], file, data
            )
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            paths[source["id"]].append(path.resolve())
    verify_excerpts(cases, lines)
    return paths, lines


def matches_from_json(stdout, ripgrep=False):
    matches = []
    for line in stdout.splitlines():
        record = json.loads(line)
        if record.get("type") != "match":
            continue
        if ripgrep:
            data = record["data"]
            matches.append(
                {
                    "path": data["path"]["text"],
                    "line": data["line_number"],
                    "text": data["lines"]["text"].rstrip("\r\n"),
                    "spans": [
                        {"byte_start": span["start"], "byte_end": span["end"]}
                        for span in data["submatches"]
                    ],
                }
            )
        else:
            matches.append(
                {
                    "path": record["path"],
                    "line": record["line"],
                    "text": record["text"],
                    "spans": [
                        {
                            "byte_start": span["token"]["start"],
                            "byte_end": span["token"]["end"],
                        }
                        for span in record["spans"]
                    ],
                }
            )
    return sorted(matches, key=lambda item: (item["path"], item["line"]))


def score(case, matches, cache, source_lines):
    target = str((cache / case["source_id"] / case["source_path"]).resolve())
    spans = []
    for matched in matches:
        if (
            matched["path"] != target
            or not case["source_line_start"]
            <= matched["line"]
            <= case["source_line_end"]
        ):
            continue
        preceding = source_lines[case["source_line_start"] - 1 : matched["line"] - 1]
        offset = sum(len(line.encode()) + 1 for line in preceding)
        spans.extend(
            {
                "byte_start": span["byte_start"] + offset,
                "byte_end": span["byte_end"] + offset,
            }
            for span in matched["spans"]
        )
    return {
        "classification": classify(case, spans),
        "target_file_found": case["expected"]
        and any(item["path"] == target for item in matches),
        "candidate_files": len({item["path"] for item in matches}),
        "candidate_lines": len(matches),
        "candidate_text_bytes": sum(len(item["text"].encode()) for item in matches),
    }


def timing(samples):
    ordered = sorted(samples)
    return {"median": statistics.median(samples), "min": ordered[0], "max": ordered[-1]}


def measure_cases(cases, methods, paths, lines, args, cache, empty_lexicon):
    results = {name: [] for name in methods}
    for case in cases:
        commands = {}
        for name, (binary, profile) in methods.items():
            if profile is None:
                command = [
                    str(binary),
                    "--no-config",
                    "--json",
                    "-n",
                    "-F",
                    "--",
                    case["query"],
                ]
            else:
                command = [
                    str(binary),
                    "--json",
                    "--no-pager",
                    "--threads",
                    "1",
                    "--user-lexicon",
                    str(empty_lexicon),
                    "--data-dir",
                    str(args.data_dir.resolve()),
                ]
                if profile == "agent":
                    command += ["--embedded", "--boundary", "any", "--pos", case["pos"]]
                command += ["--", case["query"]]
            commands[name] = command + [str(path) for path in paths[case["source_id"]]]
        observed, samples = {}, {name: [] for name in methods}
        statuses = {}
        for round_index in range(args.warmups + args.runs):
            names = list(methods)
            for offset in range(len(names)):
                name = names[(round_index + offset) % len(names)]
                started = time.perf_counter_ns()
                completed = subprocess.run(
                    commands[name], capture_output=True, check=False, timeout=60
                )
                elapsed = (time.perf_counter_ns() - started) / 1_000_000
                diagnostic = completed.stderr.decode()
                if completed.returncode not in (0, 1, 2) or (
                    completed.returncode == 2
                    and "structural_verification_incomplete" not in diagnostic
                ):
                    raise RuntimeError(f"{name} {case['id']}: {diagnostic}")
                if completed.returncode in (0, 1) and diagnostic:
                    raise RuntimeError(f"unexpected diagnostics: {diagnostic}")
                matches = matches_from_json(completed.stdout, name == "rg_literal")
                identity = (matches, completed.returncode, diagnostic)
                if name in observed and identity != observed[name]:
                    raise RuntimeError(f"unstable result: {name} {case['id']}")
                observed[name] = identity
                statuses[name] = completed.returncode
                if round_index >= args.warmups:
                    samples[name].append(elapsed)
        for profile in ["agent", "user"]:
            if (
                observed[f"baseline_{profile}"][0]
                != observed[f"candidate_{profile}"][0]
            ):
                raise RuntimeError(f"search matches changed: {profile} {case['id']}")
        for name in methods:
            metrics = score(
                case,
                observed[name][0],
                cache,
                lines[(case["source_id"], case["source_path"])],
            )
            results[name].append(
                {
                    "id": case["id"],
                    "positive": case["expected"],
                    "argv": commands[name],
                    "exit_code": statuses[name],
                    "diagnostic": observed[name][2],
                    "search_calls": 1,
                    "wall_ms": timing(samples[name]),
                    **metrics,
                }
            )
    return results


def summarize(results):
    summaries = {}
    for name, rows in results.items():
        confusion = empty_confusion()
        for row in rows:
            confusion[row["classification"]] += 1
        raw = with_metrics(confusion)
        summaries[name] = {
            "raw": raw,
            "contract_adjusted": raw.copy(),
            "reviewed_cases": 0,
            "positive_tasks": sum(row["positive"] for row in rows),
            "target_files_found": sum(row["target_file_found"] for row in rows),
            "complete_target_files_found": sum(
                row["target_file_found"] and row["exit_code"] != 2 for row in rows
            ),
            "incomplete_searches": sum(row["exit_code"] == 2 for row in rows),
            "candidate_files": sum(row["candidate_files"] for row in rows),
            "candidate_lines": sum(row["candidate_lines"] for row in rows),
            "candidate_text_bytes": sum(row["candidate_text_bytes"] for row in rows),
            "sum_task_median_wall_ms": sum(row["wall_ms"]["median"] for row in rows),
        }
    return summaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--data-dir", required=True, type=Path)
    parser.add_argument(
        "--source-cache", type=Path, default=Path("target/task-search/sources")
    )
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    args = parser.parse_args()
    if args.runs < 5 or args.warmups < 1:
        parser.error("fresh-process warmup >= 1 and measured runs >= 5 are required")
    fixture = Path(__file__).parent
    cases = load_jsonl(fixture / "cases.jsonl")
    manifest = load_json(fixture / "sources.json")
    validate_cases(cases, validate_sources(manifest))
    cache = args.source_cache.resolve()
    paths, lines = prepare_sources(manifest, cases, cache)
    empty_lexicon = cache.parent / "empty-user-lexicon.toml"
    empty_lexicon.write_text("")
    rg = shutil.which("rg")
    if not rg:
        parser.error("rg is required")
    methods = {"rg_literal": (Path(rg), None)}
    for label, binary in [("baseline", args.baseline), ("candidate", args.candidate)]:
        for profile in ["agent", "user"]:
            methods[f"{label}_{profile}"] = (binary.resolve(), profile)
    results = measure_cases(cases, methods, paths, lines, args, cache, empty_lexicon)
    summaries = summarize(results)
    report = {
        "schema_version": 1,
        "environment": {
            "platform": platform.platform(),
            "python": platform.python_version(),
            "rg": subprocess.check_output([rg, "--version"], text=True).splitlines()[0],
        },
        "revisions": {
            "baseline": args.baseline_revision,
            "candidate": args.candidate_revision,
        },
        "binaries": {
            name: {"path": str(binary), "sha256": sha256(binary)}
            for name, (binary, _) in methods.items()
        },
        "resources": {
            path.name: sha256(path)
            for path in args.data_dir.iterdir()
            if path.is_file()
        },
        "runner_sha256": sha256(Path(__file__)),
        "fixture_sha256": sha256(fixture / "cases.jsonl"),
        "sources_manifest_sha256": sha256(fixture / "sources.json"),
        "sources": manifest,
        "measurement": {
            "warmups": args.warmups,
            "runs": args.runs,
            "fresh_process": True,
            "ordering": "per-case round-robin rotation",
            "wall_unit": "ms",
            "reviewed_cases": 0,
        },
        "summaries": summaries,
        "tasks": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(summaries, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
