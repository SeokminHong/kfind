#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from fnc_dispositions import (  # noqa: E402
    load_ledger,
    load_report,
    validate_disposition_ledger,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a query-matrix FNc disposition ledger against a report."
    )
    parser.add_argument("report", type=Path)
    parser.add_argument("ledger", type=Path)
    parser.add_argument("--backend", default="kfind-full-pos")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = validate_disposition_ledger(
        load_report(args.report), load_ledger(args.ledger), args.backend
    )
    print(
        json.dumps(
            {
                "backend": args.backend,
                "raw_contract_false_negatives": summary.raw_contract_false_negatives,
                "unclassified_contract_false_negatives": (
                    summary.unclassified_contract_false_negatives
                ),
                "disposition_counts": summary.disposition_counts,
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
