from __future__ import annotations

from collections import Counter

from python.adapters import spans_overlap


def build_agent_shadow_report(
    cases: list[dict[str, object]], raw: dict[str, object]
) -> dict[str, object]:
    expected_ids = [case["id"] for case in cases]
    result_ids = [result["id"] for result in raw["results"]]
    if result_ids != expected_ids:
        raise ValueError("Agent shadow result order differs from fixture order")

    by_outcome: Counter[str] = Counter()
    by_path_presence: dict[str, Counter[str]] = {}
    by_exact_analysis: dict[str, Counter[str]] = {}
    records = []
    for case, result in zip(cases, raw["results"], strict=True):
        for matched in result["matches"]:
            outcome = match_outcome(case, matched)
            path_presence = classify_path_presence(matched["lattice"])
            exact_analysis = classify_exact_analysis(
                matched["exact_whole_token_analyses"]
            )
            by_outcome[outcome] += 1
            by_path_presence.setdefault(path_presence, Counter())[outcome] += 1
            by_exact_analysis.setdefault(exact_analysis, Counter())[outcome] += 1
            records.append(
                {
                    "case_id": case["id"],
                    "query": case["query"],
                    "query_pos": case["pos"],
                    "outcome": outcome,
                    "path_presence": path_presence,
                    "exact_analysis": exact_analysis,
                    "strict_subspan": matched["token"] != matched["whole_token"],
                    "match": matched,
                }
            )

    return {
        "profile": raw["profile"],
        "boundary": raw["boundary"],
        "morphology_artifact_sha256": raw["morphology_artifact_sha256"],
        "case_count": len(cases),
        "match_count": len(records),
        "by_outcome": dict(sorted(by_outcome.items())),
        "by_path_presence": nested_counts(by_path_presence),
        "by_exact_analysis": nested_counts(by_exact_analysis),
        "projections": {
            "all": projection_metrics(cases, records, lambda _record: True),
            "include-path": projection_metrics(
                cases,
                records,
                lambda record: record["path_presence"]
                in {"include-only", "include-and-exclude"},
            ),
            "include-only": projection_metrics(
                cases,
                records,
                lambda record: record["path_presence"] == "include-only",
            ),
        },
        "records": records,
    }


def classify_path_presence(lattice: list[dict[str, object]]) -> str:
    evaluated = [item for item in lattice if item["status"] == "evaluated"]
    if len(evaluated) != len(lattice) or not evaluated:
        return "unresolved"
    include = any(item["include_cost"] is not None for item in evaluated)
    exclude = any(item["exclude_cost"] is not None for item in evaluated)
    if include and exclude:
        return "include-and-exclude"
    if include:
        return "include-only"
    if exclude:
        return "exclude-only"
    return "no-complete-path"


def classify_exact_analysis(analyses: list[dict[str, object]]) -> str:
    if not analyses:
        return "none"
    if any(is_predicate_pos(analysis["pos"]) for analysis in analyses):
        return "predicate-or-mixed"
    return "non-predicate-only"


def is_predicate_pos(value: str) -> bool:
    predicate_tags = {"VV", "VA", "VX", "VCP", "VCN", "XSV", "XSA"}
    return bool(predicate_tags.intersection(value.upper().split("+")))


def match_outcome(case: dict[str, object], matched: dict[str, object]) -> str:
    if not case["expected"]:
        return "negative-span"
    gold_start = case["gold_byte_start"]
    gold_end = case["gold_byte_end"]
    if gold_start is None or gold_end is None:
        raise ValueError(f"positive case {case['id']} has no gold span")
    token = matched["token"]
    if spans_overlap(
        token["byte_start"], token["byte_end"], gold_start, gold_end
    ):
        return "gold-overlap"
    return "positive-other-span"


def projection_metrics(
    cases: list[dict[str, object]],
    records: list[dict[str, object]],
    keep,
) -> dict[str, object]:
    kept_by_case: dict[str, list[dict[str, object]]] = {}
    for record in records:
        if keep(record):
            kept_by_case.setdefault(record["case_id"], []).append(record)

    tp = fp = tn = fn = 0
    for case in cases:
        kept = kept_by_case.get(case["id"], [])
        predicted = any(
            record["outcome"] == "gold-overlap" for record in kept
        ) if case["expected"] else bool(kept)
        if case["expected"] and predicted:
            tp += 1
        elif case["expected"]:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "true_positive": tp,
        "false_positive": fp,
        "true_negative": tn,
        "false_negative": fn,
        "precision": round(precision, 6),
        "recall": round(recall, 6),
        "f1": round(f1, 6),
    }


def nested_counts(values: dict[str, Counter[str]]) -> dict[str, dict[str, int]]:
    return {
        key: dict(sorted(counts.items()))
        for key, counts in sorted(values.items())
    }
