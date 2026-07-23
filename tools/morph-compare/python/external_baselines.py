from __future__ import annotations

import json
from pathlib import Path

try:
    from .adapters import spans_overlap
except ImportError:
    from adapters import spans_overlap


SCHEMA_VERSION = 2
EXTERNAL_BACKENDS = ("kiwi", "lindera", "mecab-ko", "komoran")
PERFORMANCE_FIELDS = (
    "initialization_seconds",
    "evaluation_seconds",
    "cases_per_second",
    "latency_p50_ms",
    "latency_p95_ms",
    "peak_rss_kib",
)


def load_external_baselines(
    path: Path,
    cases: list[dict[str, object]],
    metadata: dict[str, object],
) -> dict[str, object]:
    snapshot = json.loads(path.read_text(encoding="utf-8"))
    if snapshot.get("schema_version") != SCHEMA_VERSION:
        raise ValueError(
            f"external baseline schema mismatch: expected {SCHEMA_VERSION}, "
            f"got {snapshot.get('schema_version')}"
        )
    fixture_sha256 = metadata.get(
        "external_baseline_fixture_sha256", metadata["fixture_sha256"]
    )
    if snapshot.get("fixture_sha256") != fixture_sha256:
        raise ValueError(
            "external baseline fixture mismatch; run "
            "scripts/refresh-morph-baselines.sh"
        )
    if snapshot.get("case_count") != len(cases):
        raise ValueError("external baseline case count differs from the fixture")

    case_ids = [str(case["id"]) for case in cases]
    versions = {}
    predictions = {}
    matches = {}
    performance = {}
    availability = {}
    backends = snapshot.get("backends")
    if not isinstance(backends, dict):
        raise ValueError("external baseline backends must be an object")
    for backend in EXTERNAL_BACKENDS:
        entry = backends.get(backend)
        if not isinstance(entry, dict):
            raise ValueError(f"external baseline is missing {backend}")
        status = entry.get("status")
        if status == "unavailable":
            reason = entry.get("reason")
            if not isinstance(reason, str) or not reason:
                raise ValueError(f"unavailable {backend} baseline has no reason")
            availability[backend] = {"status": status, "reason": reason}
            continue
        if status != "available":
            raise ValueError(f"external baseline {backend} has invalid status {status!r}")
        version = entry.get("version")
        configuration = entry.get("configuration")
        if not isinstance(version, str) or not version:
            raise ValueError(f"external baseline {backend} has no version")
        if not isinstance(configuration, dict):
            raise ValueError(f"external baseline {backend} has no configuration")
        backend_matches = validate_results(backend, entry.get("results"), case_ids)
        matches[backend] = backend_matches
        predictions[backend] = {
            str(case["id"]): span_prediction(case, backend_matches[str(case["id"])])
            for case in cases
        }
        versions[backend] = {
            "backend": backend,
            "version": version,
            "profile": None,
            "lexicon_artifact_sha256": None,
            "enriched_artifact_sha256": None,
            "morphology_artifact_sha256": None,
            "component_artifact_sha256": None,
            "configuration": configuration,
            "snapshot": True,
        }
        performance[backend] = validate_performance(
            backend, entry.get("performance")
        )
        availability[backend] = {"status": status}
    environment = snapshot.get("environment")
    if not isinstance(environment, dict):
        raise ValueError("external baseline has no environment")
    return {
        "versions": versions,
        "predictions": predictions,
        "matches": matches,
        "performance": performance,
        "availability": availability,
        "environment": environment,
    }


def validate_performance(backend: str, value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ValueError(f"external baseline {backend} has no performance")
    if not isinstance(value.get("runs"), int) or value["runs"] < 1:
        raise ValueError(f"external baseline {backend} has invalid performance runs")
    if not isinstance(value.get("warmup_runs"), int) or value["warmup_runs"] < 1:
        raise ValueError(f"external baseline {backend} has invalid warm-up runs")
    for field in PERFORMANCE_FIELDS:
        if not isinstance(value.get(field), (int, float)):
            raise ValueError(
                f"external baseline {backend} has invalid performance {field}"
            )
    for range_name in ("run_min", "run_max"):
        measured_range = value.get(range_name)
        if not isinstance(measured_range, dict):
            raise ValueError(
                f"external baseline {backend} has no performance {range_name}"
            )
        for field in PERFORMANCE_FIELDS:
            if not isinstance(measured_range.get(field), (int, float)):
                raise ValueError(
                    f"external baseline {backend} has invalid "
                    f"{range_name} {field}"
                )
    return value


def validate_results(
    backend: str, results: object, case_ids: list[str]
) -> dict[str, list[dict[str, object]]]:
    if not isinstance(results, list):
        raise ValueError(f"external baseline {backend} results must be an array")
    result_ids = [result.get("id") for result in results if isinstance(result, dict)]
    if result_ids != case_ids:
        raise ValueError(f"external baseline {backend} result order differs from fixture")
    validated = {}
    for result in results:
        spans = result.get("matching_spans")
        if not isinstance(spans, list):
            raise ValueError(f"external baseline {backend} has invalid matching spans")
        for span in spans:
            if (
                not isinstance(span, dict)
                or not isinstance(span.get("byte_start"), int)
                or not isinstance(span.get("byte_end"), int)
                or span["byte_start"] < 0
                or span["byte_start"] >= span["byte_end"]
            ):
                raise ValueError(f"external baseline {backend} has an invalid span")
        validated[str(result["id"])] = spans
    return validated


def span_prediction(
    case: dict[str, object], spans: list[dict[str, object]]
) -> bool:
    if not case["expected"]:
        return bool(spans)
    gold_start = case["gold_byte_start"]
    gold_end = case["gold_byte_end"]
    if gold_start is None or gold_end is None:
        raise ValueError(f"positive case {case['id']} has no gold span")
    return any(
        spans_overlap(
            int(span["byte_start"]), int(span["byte_end"]), gold_start, gold_end
        )
        for span in spans
    )
