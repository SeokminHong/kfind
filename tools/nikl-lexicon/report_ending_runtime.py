#!/usr/bin/env python3

"""Join the complete ending inventory with span-level matcher results."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path


def coverage(spans, start: int, end: int) -> str:
    if any(span["byte_start"] == start and span["byte_end"] == end for span in spans):
        return "full"
    if any(span["byte_start"] < end and start < span["byte_end"] for span in spans):
        return "partial"
    return "missing"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    directory = args.directory
    inventory = read_json(directory / "inventory.json")
    with (directory / "cases.jsonl").open(encoding="utf-8") as stream:
        cases = [json.loads(line) for line in stream]
    profiles = {}
    for boundary in ("smart", "any"):
        report = read_json(directory / f"{boundary}.json")
        if report["profile"] != "full-pos" or report["boundary"] != boundary:
            raise ValueError(f"unexpected matcher profile for {boundary}")
        profiles[boundary] = report

    for row in inventory.values():
        for boundary in profiles:
            row[boundary] = dict.fromkeys(("full", "partial", "missing"), 0)

    for boundary, report in profiles.items():
        for case, result in zip(cases, report["results"], strict=True):
            if case["id"] != result["id"]:
                raise ValueError(f"result mismatch for {case['id']}")
            result_coverage = coverage(
                result["spans"], case["gold_byte_start"], case["gold_byte_end"]
            )
            inventory[case["ending"]][boundary][result_coverage] += 1
            case.setdefault("results", {})[boundary] = {
                "coverage": result_coverage,
                "spans": result["spans"],
            }

    counts = Counter()
    for ending, row in inventory.items():
        total = row["aligned_examples"]
        if total == 0:
            counts["nonstandard_redirect_only" if row["redirect_only"] else "no_aligned_evidence"] += 1
        for boundary in profiles:
            score = row[boundary]
            if sum(score.values()) != total:
                raise ValueError(f"wrong result count for {ending} ({boundary})")
            if total == 0:
                continue
            if score["missing"] == total:
                counts[f"{boundary}_all_missing"] += 1
            elif score["full"] == total:
                counts[f"{boundary}_all_full"] += 1
            else:
                counts[f"{boundary}_mixed_or_partial"] += 1

    result = {
        "revision": args.revision,
        "catalog_size": len(inventory),
        "case_count": len(cases),
        "counts": dict(counts),
        "profiles": {
            boundary: {key: value for key, value in report.items() if key != "results"}
            for boundary, report in profiles.items()
        },
        "inventory": inventory,
        "cases": cases,
    }
    (directory / "report.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(result["counts"], ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
