#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from python.report import KFIND_PROFILES, build_report, quality_metrics
from python.validation import load_cases, validate_local_context_dataset


DEFAULT_CASES = Path("/opt/morph-benchmark/data/unseen-local-context-cases.jsonl")
DEFAULT_METADATA = Path(
    "/opt/morph-benchmark/data/unseen-local-context-metadata.json"
)
DEFAULT_RUNNER = Path("/usr/local/bin/morph-benchmark-runner")
OUTCOMES = ("accept", "reject", "ambiguous", "unresolved")


def _required_span(value: dict[str, object], key: str) -> tuple[int, int]:
    span = value.get(key)
    if not isinstance(span, dict):
        raise ValueError(f"projection value has no {key} span")
    start = span.get("byte_start")
    end = span.get("byte_end")
    if not isinstance(start, int) or not isinstance(end, int) or start >= end:
        raise ValueError(f"projection value has an invalid {key} span")
    return start, end


def _required_int(value: dict[str, object], key: str) -> int:
    result = value.get(key)
    if not isinstance(result, int):
        raise ValueError(f"projection value has no integer {key}")
    return result


def _rule_path(value: dict[str, object]) -> tuple[str, ...]:
    rules = value.get("rule_path")
    if not isinstance(rules, list) or not all(
        isinstance(rule, str) for rule in rules
    ):
        raise ValueError("projection origin has an invalid rule_path")
    return tuple(rules)


def _origin_key(
    atom_index: int,
    core: tuple[int, int],
    value: dict[str, object],
) -> tuple[int, int, int, int, tuple[str, ...]]:
    return (
        atom_index,
        core[0],
        core[1],
        _required_int(value, "analysis_index"),
        _rule_path(value),
    )


def _outcome(evidence: dict[str, object]) -> str:
    decision = evidence.get("decision")
    if decision is None:
        return "unresolved"
    if decision not in OUTCOMES[:-1]:
        raise ValueError(f"unknown lattice decision {decision!r}")
    return str(decision)


def _span_prediction(
    case: dict[str, object], spans: list[dict[str, object]]
) -> bool:
    if not case["expected"]:
        return bool(spans)
    gold_start = case.get("gold_byte_start")
    gold_end = case.get("gold_byte_end")
    if not isinstance(gold_start, int) or not isinstance(gold_end, int):
        raise ValueError(f"positive case {case['id']} has no gold span")
    for span in spans:
        start = span.get("byte_start")
        end = span.get("byte_end")
        if not isinstance(start, int) or not isinstance(end, int) or start >= end:
            raise ValueError("projection contains an invalid match span")
        if start < gold_end and gold_start < end:
            return True
    return False


def _evidence_by_origin(
    shadow: dict[str, object],
) -> dict[tuple[int, int, int, int, tuple[str, ...]], dict[str, object]]:
    lattice = shadow.get("lattice", [])
    if not isinstance(lattice, list):
        raise ValueError("shadow verification lattice is not a list")
    indexed = {}
    for evidence in lattice:
        if not isinstance(evidence, dict):
            raise ValueError("shadow verification contains invalid evidence")
        key = _origin_key(
            _required_int(evidence, "atom_index"),
            _required_span(evidence, "target"),
            evidence,
        )
        if key in indexed:
            raise ValueError("shadow verification repeats a contextual origin")
        indexed[key] = evidence
    return indexed


def _origin_decision(
    atom_index: int,
    core: tuple[int, int],
    token: tuple[int, int],
    origin: dict[str, object],
    evidence: dict[str, object] | None,
) -> dict[str, object]:
    contextual = evidence is not None
    return {
        "atom_index": atom_index,
        "core": {"byte_start": core[0], "byte_end": core[1]},
        "token": {"byte_start": token[0], "byte_end": token[1]},
        "analysis_index": _required_int(origin, "analysis_index"),
        "rule_path": list(_rule_path(origin)),
        "contextual": contextual,
        "outcome": _outcome(evidence) if evidence is not None else "not-contextual",
        "status": evidence.get("status") if evidence is not None else None,
        "include_cost": evidence.get("include_cost") if evidence else None,
        "exclude_cost": evidence.get("exclude_cost") if evidence else None,
        "cost_margin": evidence.get("cost_margin") if evidence else None,
    }


def _select_single_atom_spans(
    candidates: list[dict[str, object]],
) -> list[dict[str, int]]:
    ordered = sorted(
        candidates,
        key=lambda candidate: (
            _required_span(candidate, "token")[0],
            -_required_span(candidate, "token")[1],
            _required_span(candidate, "core")[0],
            -_required_span(candidate, "core")[1],
        ),
    )
    selected = []
    at = 0
    for candidate in ordered:
        start, end = _required_span(candidate, "token")
        if start < at:
            continue
        selected.append({"byte_start": start, "byte_end": end})
        at = end
    return selected


def project_case(
    case: dict[str, object],
    union_prediction: bool,
    union_spans: list[dict[str, object]],
    candidates: list[dict[str, object]],
    shadow: dict[str, object],
) -> tuple[dict[str, object], Counter[str]]:
    evidence_by_origin = _evidence_by_origin(shadow)
    matched_evidence = set()
    projected_candidates = []
    origin_decisions = []
    for candidate in candidates:
        atom_index = _required_int(candidate, "atom_index")
        if atom_index != 0:
            raise ValueError("unseen projection requires single-atom candidates")
        core = _required_span(candidate, "core")
        token = _required_span(candidate, "token")
        origins = candidate.get("origins")
        if not isinstance(origins, list) or not origins:
            raise ValueError("policy candidate has no origins")
        retained = []
        for origin in origins:
            if not isinstance(origin, dict):
                raise ValueError("policy candidate contains an invalid origin")
            key = _origin_key(atom_index, core, origin)
            evidence = evidence_by_origin.get(key)
            if evidence is not None:
                matched_evidence.add(key)
            decision = _origin_decision(atom_index, core, token, origin, evidence)
            origin_decisions.append(decision)
            if decision["outcome"] != "reject":
                retained.append(origin)
        if retained:
            projected_candidates.append(candidate | {"origins": retained})
    if matched_evidence != evidence_by_origin.keys():
        raise ValueError("lattice evidence does not map to a verified atom origin")

    projected_spans = _select_single_atom_spans(projected_candidates)
    projected_prediction = _span_prediction(case, projected_spans)
    if _span_prediction(case, union_spans) != union_prediction:
        raise ValueError(f"union prediction differs for case {case['id']}")
    outcomes = Counter(_outcome(evidence) for evidence in evidence_by_origin.values())
    gold_span = None
    if case["expected"]:
        gold_span = {
            "byte_start": case["gold_byte_start"],
            "byte_end": case["gold_byte_end"],
        }
    return (
        {
            "expected": bool(case["expected"]),
            "gold_span": gold_span,
            "union_prediction": union_prediction,
            "projected_prediction": projected_prediction,
            "union_spans": union_spans,
            "projected_spans": projected_spans,
            "origins": origin_decisions,
        },
        outcomes,
    )


def build_copula_policy_projection(
    cases: list[dict[str, object]], evaluation: dict[str, object]
) -> dict[str, object]:
    profiles = {}
    for profile in KFIND_PROFILES:
        predictions = {}
        by_case = {}
        outcomes: Counter[str] = Counter()
        for case in cases:
            case_id = str(case["id"])
            record, case_outcomes = project_case(
                case,
                bool(evaluation["predictions"][profile][case_id]),
                evaluation["matches"][profile][case_id],
                evaluation["policy_candidates"][profile][case_id],
                evaluation["shadow_verification"][profile][case_id],
            )
            predictions[case_id] = bool(record["projected_prediction"])
            by_case[case_id] = record
            outcomes.update(case_outcomes)
        quality = quality_metrics(cases, predictions)
        profiles[profile] = {
            "target_confusion_matrix": {
                name: quality[name] for name in ("tp", "fp", "tn", "fn")
            },
            "target_precision_percent": quality["precision_percent"],
            "gold_recall_percent": quality["recall_percent"],
            "origin_outcomes": {
                outcome: outcomes[outcome] for outcome in OUTCOMES
            },
            "by_case": by_case,
        }
    return {"policy": "copula-lattice", "profiles": profiles}


def render_markdown(report: dict[str, object]) -> str:
    dataset = report["dataset"]
    source = dataset["sources"][0]
    lines = [
        "# kfind copula lattice unseen evaluation",
        "",
        f"- fixture: `{dataset['fixture_sha256']}`",
        f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
        f"{dataset['negative_cases']} negative)",
        f"- source: {source['name']} {source['revision']} {source['split']}",
        f"- source SHA-256: `{source['data_sha256']}`",
        "- evaluation: one measured run without warm-up",
        "",
        "| profile | precision | recall | TP | FP | TN | FN | accept | reject | "
        "ambiguous | unresolved |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    profiles = report["copula_policy_projection"]["profiles"]
    for profile in KFIND_PROFILES:
        projection = profiles[profile]
        confusion = projection["target_confusion_matrix"]
        outcomes = projection["origin_outcomes"]
        lines.append(
            f"| {profile} | {projection['target_precision_percent']}% | "
            f"{projection['gold_recall_percent']}% | {confusion['tp']} | "
            f"{confusion['fp']} | {confusion['tn']} | {confusion['fn']} | "
            f"{outcomes['accept']} | {outcomes['reject']} | "
            f"{outcomes['ambiguous']} | {outcomes['unresolved']} |"
        )
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    parser.add_argument("--metadata", type=Path, default=DEFAULT_METADATA)
    parser.add_argument("--runner", type=Path, default=DEFAULT_RUNNER)
    parser.add_argument("--output", type=Path, default=Path("/output/report.json"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        from benchmark import evaluate_dataset

        cases = load_cases(args.cases)
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        validate_local_context_dataset(
            args.cases, cases, metadata, "unseen-local-context"
        )
        evaluation = evaluate_dataset(cases, args.cases, args.runner, 1, False)
        report = build_report(
            cases,
            metadata,
            evaluation["versions"],
            evaluation["predictions"],
            evaluation["matches"],
            evaluation["performance"],
            evaluation["diagnostics"],
            evaluation["shadow_verification"],
            include_performance=False,
        )
        report["schema_version"] = 13
        report["task"] = "sealed copula lattice product-candidate projection"
        report["copula_policy_projection"] = build_copula_policy_projection(
            cases, evaluation
        )
        markdown = render_markdown(report)
        print(markdown, end="")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        args.output.with_suffix(".md").write_text(markdown, encoding="utf-8")
        return 0
    except (KeyError, OSError, RuntimeError, TypeError, ValueError) as error:
        print(f"unseen benchmark failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
