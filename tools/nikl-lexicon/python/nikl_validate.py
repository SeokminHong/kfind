"""Distribution validation for generated enriched-predicate candidates."""

from __future__ import annotations

import csv
import tomllib
from pathlib import Path


EXPECTED_SCHEMA_VERSION = 5
EXPECTED_HEADER = [
    "lemma",
    "pos",
    "alternation",
    "flags",
    "overrides",
    "derivations",
]
DEFAULT_MAX_ARTIFACT_BYTES = 64 * 1024


class ValidationError(ValueError):
    """Raised when a generated candidate violates the distribution contract."""


def _required_integer(stats: dict[str, object], key: str) -> int:
    value = stats.get(key)
    if not isinstance(value, int):
        raise ValidationError(f"STATS.toml {key} must be an integer")
    return value


def validate_candidate(
    candidate_directory: Path,
    *,
    max_artifact_bytes: int = DEFAULT_MAX_ARTIFACT_BYTES,
) -> dict[str, int | str]:
    predicates_path = candidate_directory / "predicates.tsv"
    stats_path = candidate_directory / "STATS.toml"
    predicates_bytes = predicates_path.read_bytes()
    try:
        predicates_text = predicates_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValidationError("predicates.tsv must be UTF-8") from error

    rows = list(csv.reader(predicates_text.splitlines(), delimiter="\t"))
    if not rows or rows[0] != EXPECTED_HEADER:
        raise ValidationError("predicates.tsv header does not match schema 5")
    malformed_rows = [index for index, row in enumerate(rows[1:], start=2) if len(row) != 6]
    if malformed_rows:
        raise ValidationError(
            f"predicates.tsv rows must have 6 columns: {malformed_rows[:5]}"
        )

    stats = tomllib.loads(stats_path.read_text(encoding="utf-8"))
    schema_version = _required_integer(stats, "schema_version")
    if schema_version != EXPECTED_SCHEMA_VERSION:
        raise ValidationError(
            f"STATS.toml schema_version mismatch: "
            f"{schema_version} != {EXPECTED_SCHEMA_VERSION}"
        )

    artifact_bytes = len(predicates_bytes)
    reported_artifact_bytes = _required_integer(stats, "artifact_bytes")
    if artifact_bytes != reported_artifact_bytes:
        raise ValidationError(
            f"artifact_bytes mismatch: {artifact_bytes} != {reported_artifact_bytes}"
        )

    surface_only_count = sum(row[2] == "SurfaceOnly" for row in rows[1:])
    reported_surface_only_count = _required_integer(stats, "surface_only_count")
    if surface_only_count != reported_surface_only_count:
        raise ValidationError(
            "surface_only_count mismatch: "
            f"{surface_only_count} != {reported_surface_only_count}"
        )
    if artifact_bytes > max_artifact_bytes:
        raise ValidationError(
            f"predicates.tsv exceeds distribution limit: "
            f"{artifact_bytes} > {max_artifact_bytes}"
        )

    return {
        "candidate_directory": str(candidate_directory),
        "artifact_bytes": artifact_bytes,
        "max_artifact_bytes": max_artifact_bytes,
        "surface_only_count": surface_only_count,
    }
