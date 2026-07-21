from __future__ import annotations

import argparse
import json
from pathlib import Path

try:
    from .query_matrix_contract import (
        apply_contract_reviews,
        load_contract_reviews,
        sha256,
    )
    from .validation import load_cases, write_cases
except ImportError:
    from query_matrix_contract import apply_contract_reviews, load_contract_reviews, sha256
    from validation import load_cases, write_cases


def apply_fixture_contract(
    *, cases_path: Path, metadata_path: Path, reviews_path: Path
) -> dict[str, object]:
    cases = load_cases(cases_path)
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    query_mode = str(metadata["query_mode"])
    split = str(metadata["split"])
    summary = apply_contract_reviews(
        cases,
        load_contract_reviews(reviews_path),
        query_mode=query_mode,
        split=split,
    )
    write_cases(cases_path, cases)
    metadata["fixture_sha256"] = sha256(cases_path)
    metadata["contract_review"] = {
        "registry_sha256": sha256(reviews_path),
        **summary,
    }
    metadata_path.write_text(
        json.dumps(metadata, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return metadata


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--reviews", type=Path, required=True)
    args = parser.parse_args()
    metadata = apply_fixture_contract(
        cases_path=args.cases,
        metadata_path=args.metadata,
        reviews_path=args.reviews,
    )
    print(json.dumps(metadata["contract_review"], ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
