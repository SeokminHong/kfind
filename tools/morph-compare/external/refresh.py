#!/usr/bin/env python3

from __future__ import annotations

import argparse
import base64
import importlib.metadata
import json
import subprocess
import sys
from pathlib import Path

import mecab_ko
from kiwipiepy import Kiwi

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from benchmark import (
    DEFAULT_CASES,
    DEFAULT_METADATA,
    DEFAULT_RUNNER,
    matching_candidate_spans,
    run_native_backend,
)
from python.adapters import (
    komoran_candidates,
    lindera_candidates,
    mecab_candidates,
    kiwi_candidates,
)
from python.external_baselines import EXTERNAL_BACKENDS, SCHEMA_VERSION
from python.validation import load_cases, validate_dataset


KOMORAN_CLASSPATH = "/opt/morph-benchmark/external:/opt/morph-benchmark/external/komoran.jar"


def results_from_candidates(
    cases: list[dict[str, object]], candidates_by_id: dict[str, set[object]]
) -> list[dict[str, object]]:
    return [
        {
            "id": case["id"],
            "matching_spans": matching_candidate_spans(
                case, candidates_by_id[str(case["id"])]
            ),
        }
        for case in cases
    ]


def capture_kiwi(cases: list[dict[str, object]]) -> dict[str, object]:
    kiwi = Kiwi()
    kiwi.tokenize("")
    candidates = {
        str(case["id"]): kiwi_candidates(
            str(case["text"]), kiwi.tokenize(str(case["text"]))
        )
        for case in cases
    }
    return available(
        importlib.metadata.version("kiwipiepy"),
        {"model": importlib.metadata.version("kiwipiepy_model")},
        results_from_candidates(cases, candidates),
    )


def capture_lindera(
    cases: list[dict[str, object]], cases_path: Path, runner: Path
) -> dict[str, object]:
    summary = run_native_backend(runner, "lindera", cases_path)
    results = {str(result["id"]): result for result in summary["results"]}
    candidates = {
        str(case["id"]): lindera_candidates(results[str(case["id"])]["tokens"])
        for case in cases
    }
    return available(
        str(summary["version"]),
        {"dictionary": "embedded-ko-dic"},
        results_from_candidates(cases, candidates),
    )


def capture_mecab(cases: list[dict[str, object]]) -> dict[str, object]:
    tagger = mecab_ko.Tagger()
    tagger.parse("")
    candidates = {
        str(case["id"]): mecab_candidates(
            str(case["text"]), tagger.parse(str(case["text"]))
        )
        for case in cases
    }
    return available(
        importlib.metadata.version("mecab-ko"),
        {"dictionary": importlib.metadata.version("mecab-ko-dic")},
        results_from_candidates(cases, candidates),
    )


def capture_komoran(cases: list[dict[str, object]]) -> dict[str, object]:
    encoded = "\n".join(
        base64.b64encode(str(case["text"]).encode()).decode() for case in cases
    )
    process = subprocess.run(
        ["java", "-cp", KOMORAN_CLASSPATH, "KomoranRunner"],
        input=encoded + "\n",
        text=True,
        capture_output=True,
    )
    if process.returncode != 0:
        raise RuntimeError(f"KOMORAN failed: {process.stderr.strip()}")
    output_lines = process.stdout.splitlines()
    if len(output_lines) != len(cases):
        raise ValueError("KOMORAN result count differs from fixture")
    candidates = {}
    for case, line in zip(cases, output_lines):
        tokens = []
        for encoded_token in filter(None, line.split(";")):
            morph, pos, begin, end = encoded_token.split(",")
            tokens.append(
                {
                    "morph": base64.b64decode(morph).decode(),
                    "pos": pos,
                    "begin": int(begin),
                    "end": int(end),
                }
            )
        candidates[str(case["id"])] = komoran_candidates(str(case["text"]), tokens)
    return available(
        "3.3.9",
        {"model": "FULL"},
        results_from_candidates(cases, candidates),
    )


def available(
    version: str, configuration: dict[str, object], results: list[dict[str, object]]
) -> dict[str, object]:
    return {
        "status": "available",
        "version": version,
        "configuration": configuration,
        "results": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--runner", type=Path, default=DEFAULT_RUNNER)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--backends", default=",".join(EXTERNAL_BACKENDS), help="comma-separated"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    selected = tuple(filter(None, args.backends.split(",")))
    unknown = sorted(set(selected) - set(EXTERNAL_BACKENDS))
    if unknown:
        raise ValueError(f"unknown external backends: {', '.join(unknown)}")
    cases = load_cases(args.cases)
    metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
    validate_dataset(args.cases, cases, metadata)
    backends = {
        backend: {"status": "unavailable", "reason": "snapshot not captured"}
        for backend in EXTERNAL_BACKENDS
    }
    if args.output.exists():
        current = json.loads(args.output.read_text(encoding="utf-8"))
        if current.get("fixture_sha256") == metadata["fixture_sha256"]:
            backends.update(current.get("backends", {}))
    captures = {
        "kiwi": lambda: capture_kiwi(cases),
        "lindera": lambda: capture_lindera(cases, args.cases, args.runner),
        "mecab-ko": lambda: capture_mecab(cases),
        "komoran": lambda: capture_komoran(cases),
    }
    for backend in selected:
        backends[backend] = captures[backend]()
    snapshot = {
        "schema_version": SCHEMA_VERSION,
        "fixture_sha256": metadata["fixture_sha256"],
        "case_count": len(cases),
        "backends": backends,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
