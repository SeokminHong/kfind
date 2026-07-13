#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from benchmark import DEFAULT_RUNNER, evaluate_dataset
from python.report import append_local_context_summary, build_report
from python.validation import load_cases, validate_local_context_dataset


DEFAULT_CASES = Path("/opt/morph-benchmark/data/blind-local-context-cases.jsonl")
DEFAULT_METADATA = Path(
    "/opt/morph-benchmark/data/blind-local-context-metadata.json"
)


def render_markdown(report: dict[str, object]) -> str:
    blind = report["blind_local_context"]
    dataset = blind["dataset"]
    source = dataset["sources"][0]
    lines = [
        "# kfind copula lattice blind evaluation",
        "",
        f"- fixture: `{dataset['fixture_sha256']}`",
        f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
        f"{dataset['negative_cases']} negative)",
        f"- source: {source['name']} {source['revision']} {source['split']}",
        f"- source SHA-256: `{source['data_sha256']}`",
        "- evaluation: one measured run without warm-up",
    ]
    append_local_context_summary(lines, blind)
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--runner", type=Path, default=DEFAULT_RUNNER)
    parser.add_argument("--output", type=Path, default=Path("/output/report.json"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        cases = load_cases(args.cases)
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        validate_local_context_dataset(
            args.cases, cases, metadata, "blind-local-context"
        )
        evaluation = evaluate_dataset(cases, args.cases, args.runner, 1, False)
        blind = build_report(
            cases,
            metadata,
            evaluation["versions"],
            evaluation["predictions"],
            evaluation["matches"],
            evaluation["performance"],
            evaluation["diagnostics"],
            evaluation["shadow_verification"],
            include_performance=False,
        )
        report = {
            "schema_version": 1,
            "task": "sealed copula lattice blind evaluation",
            "blind_local_context": blind,
        }
        markdown = render_markdown(report)
        print(markdown, end="")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        args.output.with_suffix(".md").write_text(markdown, encoding="utf-8")
        return 0
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"blind benchmark failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
