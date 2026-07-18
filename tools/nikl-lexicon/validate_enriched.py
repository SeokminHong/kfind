#!/usr/bin/env python3
"""Validate an enriched-predicate candidate without regenerating it."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_validate import (  # noqa: E402
    DEFAULT_MAX_ARTIFACT_BYTES,
    ValidationError,
    validate_candidate,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a generated enriched-predicate candidate."
    )
    parser.add_argument("candidate_directory", type=Path)
    parser.add_argument(
        "--max-artifact-bytes",
        type=int,
        default=DEFAULT_MAX_ARTIFACT_BYTES,
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        result = validate_candidate(
            args.candidate_directory,
            max_artifact_bytes=args.max_artifact_bytes,
        )
    except (OSError, tomllib.TOMLDecodeError, ValidationError) as error:
        print(f"enriched candidate validation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
