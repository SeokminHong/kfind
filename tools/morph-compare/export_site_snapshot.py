#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


SNAPSHOT_FIELDS = (
    "backends",
    "quality",
    "performance",
    "component_startup",
    "product_workflows",
    "product_use_cases",
    "product_persona_comparison",
    "external_baselines",
    "robustness",
)
REVISION_PATTERN = re.compile(r"[0-9a-f]{7,40}")


def export_snapshot(report_path: Path, revision: str) -> dict[str, object]:
    if REVISION_PATTERN.fullmatch(revision) is None:
        raise ValueError("revision must be a 7-40 character lowercase Git hash")

    report_bytes = report_path.read_bytes()
    report = json.loads(report_bytes)
    required_fields = (*SNAPSHOT_FIELDS, "query_matrix")
    missing = [field for field in required_fields if field not in report]
    if missing:
        raise ValueError(f"report is missing site chart fields: {', '.join(missing)}")

    return {
        "site_snapshot_schema_version": 3,
        "source_report": {
            "revision": revision,
            "sha256": hashlib.sha256(report_bytes).hexdigest(),
            "schema_version": report.get("schema_version"),
        },
        **{field: report[field] for field in SNAPSHOT_FIELDS},
        "query_matrix": query_matrix_summary(report["query_matrix"]),
    }


def query_matrix_summary(query_matrix: dict[str, object]) -> dict[str, object]:
    explicit = query_matrix["explicit_pos"]
    profiles = ("kfind-embedded", "kfind-full-pos")
    return {
        "explicit_pos": {
            "dataset": {
                "cases": explicit["dataset"]["cases"],
                "contract_review": explicit["dataset"]["contract_review"],
            },
            "quality": {
                profile: {
                    "overall": explicit["quality"][profile]["overall"],
                    "contract_adjusted": {
                        "overall": explicit["quality"][profile][
                            "contract_adjusted"
                        ]["overall"]
                    },
                }
                for profile in profiles
            },
        }
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()

    snapshot = export_snapshot(args.report, args.revision)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
