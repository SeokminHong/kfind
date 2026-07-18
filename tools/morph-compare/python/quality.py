from __future__ import annotations

from collections import defaultdict
from collections.abc import Callable


RECLASSIFICATION_REASONS = {
    "aligned-source-component",
    "same-pos-homograph",
    "structurally-indistinguishable-homograph",
}
EXCLUSION_REASONS = {
    "cost-prohibitive",
    "gold-or-adapter",
    "out-of-contract",
    "structurally-unresolvable",
}


def contract_expected(case: dict[str, object]) -> bool | None:
    strict_expected = bool(case["expected"])
    if "contract_expected" not in case:
        if "contract_reason" in case:
            raise ValueError(
                f"case {case['id']} has contract_reason without contract_expected"
            )
        return strict_expected

    adjusted_expected = case["contract_expected"]
    reason = case.get("contract_reason")
    if adjusted_expected is None:
        if reason not in EXCLUSION_REASONS:
            raise ValueError(
                f"case {case['id']} has unsupported exclusion reason {reason!r}"
            )
        return None
    if not isinstance(adjusted_expected, bool):
        raise ValueError(
            f"case {case['id']} contract_expected must be boolean or null"
        )
    if adjusted_expected == strict_expected:
        raise ValueError(
            f"case {case['id']} contract_expected must differ from expected"
        )
    if strict_expected or not adjusted_expected:
        raise ValueError(
            f"case {case['id']} may only reclassify strict negative to contract positive"
        )
    if reason not in RECLASSIFICATION_REASONS:
        raise ValueError(
            f"case {case['id']} has unsupported contract_reason {reason!r}"
        )
    return adjusted_expected


def quality_metrics(
    cases: list[dict[str, object]], predictions: dict[str, bool]
) -> dict[str, object]:
    counts = _confusion_counts(cases, predictions, lambda case: bool(case["expected"]))
    return _derived_metrics(len(cases), counts, "")


def contract_quality_metrics(
    cases: list[dict[str, object]], predictions: dict[str, bool]
) -> dict[str, object]:
    included_cases = [case for case in cases if contract_expected(case) is not None]
    counts = _confusion_counts(included_cases, predictions, contract_expected)
    metrics = _derived_metrics(len(included_cases), counts, "contract_")
    reclassified_reasons: dict[str, int] = defaultdict(int)
    excluded_reasons: dict[str, int] = defaultdict(int)
    for case in cases:
        expected = contract_expected(case)
        if "contract_expected" not in case:
            continue
        reasons = excluded_reasons if expected is None else reclassified_reasons
        reasons[str(case["contract_reason"])] += 1
    return {
        **metrics,
        "reviewed_cases": sum(reclassified_reasons.values())
        + sum(excluded_reasons.values()),
        "reclassified_cases": sum(reclassified_reasons.values()),
        "reclassified_by_reason": dict(sorted(reclassified_reasons.items())),
        "excluded_cases": sum(excluded_reasons.values()),
        "excluded_by_reason": dict(sorted(excluded_reasons.items())),
    }


def grouped_quality(
    cases: list[dict[str, object]], predictions: dict[str, bool], key: str
) -> dict[str, dict[str, object]]:
    return _grouped_metrics(cases, predictions, key, quality_metrics)


def grouped_contract_quality(
    cases: list[dict[str, object]], predictions: dict[str, bool], key: str
) -> dict[str, dict[str, object]]:
    return _grouped_metrics(cases, predictions, key, contract_quality_metrics)


def _confusion_counts(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    expected_for: Callable[[dict[str, object]], bool | None],
) -> tuple[int, int, int, int]:
    tp = fp = tn = fn = 0
    for case in cases:
        expected = expected_for(case)
        if expected is None:
            raise ValueError("excluded case reached confusion count")
        predicted = predictions[str(case["id"])]
        if expected and predicted:
            tp += 1
        elif expected:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    return tp, fp, tn, fn


def _derived_metrics(
    case_count: int, counts: tuple[int, int, int, int], prefix: str
) -> dict[str, object]:
    tp, fp, tn, fn = counts
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    negative_precision = tn / (tn + fp) if tn + fp else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "cases": case_count,
        f"{prefix}tp": tp,
        f"{prefix}fp": fp,
        f"{prefix}tn": tn,
        f"{prefix}fn": fn,
        f"{prefix}accuracy_percent": round(100 * (tp + tn) / case_count, 2)
        if case_count
        else 0.0,
        f"{prefix}precision_percent": round(100 * precision, 2),
        f"{prefix}hard_negative_precision_percent": round(
            100 * negative_precision, 2
        ),
        f"{prefix}recall_percent": round(100 * recall, 2),
        f"{prefix}f1_percent": round(100 * f1, 2),
    }


def _grouped_metrics(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    key: str,
    calculate: Callable[
        [list[dict[str, object]], dict[str, bool]], dict[str, object]
    ],
) -> dict[str, dict[str, object]]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        groups[str(case[key])].append(case)
    return {
        name: calculate(group_cases, predictions)
        for name, group_cases in sorted(groups.items())
    }
