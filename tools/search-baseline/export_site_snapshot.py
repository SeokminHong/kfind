#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


QUALITY_METHODS = (
    "kfind_any",
    "kfind_smart",
    "regex_enumerated",
    "regex_stem",
)
PERFORMANCE_METHODS = (
    "kfind_any",
    "kfind_smart",
    "rg_enumerated",
    "grep_enumerated",
    "rg_stem",
    "grep_stem",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def select_metrics(metrics: dict[str, Any]) -> dict[str, int | float]:
    return {
        key: metrics[key]
        for key in (
            "tp",
            "tn",
            "fp",
            "fn",
            "precision_percent",
            "recall_percent",
            "f1_percent",
        )
    }


def export(report_path: Path) -> dict[str, Any]:
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema_version") != 2:
        raise ValueError("search baseline report schema_version must be 2")
    fixture = report["fixture"]
    if fixture["kind"] != "constructed-diagnostic":
        raise ValueError("search baseline fixture kind must be constructed-diagnostic")

    return {
        "schema_version": 2,
        "source_report": {
            "revision": report["revision"],
            "sha256": sha256(report_path),
        },
        "fixture": {
            key: fixture[key]
            for key in (
                "kind",
                "queries",
                "cases",
                "strict_positive",
                "strict_negative",
                "contract_positive",
                "contract_negative",
                "reviewed_cases",
                "sha256",
            )
        },
        "quality": [
            {
                "id": method,
                "raw": select_metrics(report["quality"][method]["raw"]),
                "contract_adjusted": select_metrics(
                    report["quality"][method]["contract_adjusted"]
                ),
            }
            for method in QUALITY_METHODS
        ],
        "performance": {
            "bytes": report["performance"]["bytes"],
            "lines": report["performance"]["lines"],
            "warmup": report["performance"]["warmup"],
            "runs": report["performance"]["runs"],
            "workload": report["performance"]["workload"],
            "methods": [
                {
                    "id": method,
                    **{
                        key: report["performance"]["methods"][method][key]
                        for key in (
                            "median_ms",
                            "min_ms",
                            "max_ms",
                            "p95_ms",
                            "effective_mib_per_second",
                        )
                    },
                }
                for method in PERFORMANCE_METHODS
            ],
        },
        "tools": {
            key: report["environment"][key] for key in ("kfind", "rg", "grep")
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    snapshot = export(args.report.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
