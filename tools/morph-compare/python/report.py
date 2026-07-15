from __future__ import annotations

import os
import platform
from collections import defaultdict

try:
    from .shadow_report import (
        KFIND_PROFILES,
        append_component_shadow_table,
        append_shadow_verification,
        classify_component_paths,
        classify_component_source_provenance,
        shadow_verification_summary,
        summarize_analysis_graph,
    )
except ImportError:
    from shadow_report import (
        KFIND_PROFILES,
        append_component_shadow_table,
        append_shadow_verification,
        classify_component_paths,
        classify_component_source_provenance,
        shadow_verification_summary,
        summarize_analysis_graph,
    )

BACKENDS = (*KFIND_PROFILES, "kiwi", "lindera")
CONNECTIVE_JI_PATH = ("ending.connective-ji",)
STRICT_SUBSPAN_POSITIONS = ("left-edge", "right-edge", "internal")


def quality_metrics(
    cases: list[dict[str, object]], predictions: dict[str, bool]
) -> dict[str, object]:
    tp = fp = tn = fn = 0
    for case in cases:
        expected = bool(case["expected"])
        predicted = predictions[case["id"]]
        if expected and predicted:
            tp += 1
        elif expected:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    negative_precision = tn / (tn + fp) if tn + fp else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "cases": len(cases),
        "tp": tp,
        "fp": fp,
        "tn": tn,
        "fn": fn,
        "accuracy_percent": round(100 * (tp + tn) / len(cases), 2),
        "precision_percent": round(100 * precision, 2),
        "hard_negative_precision_percent": round(100 * negative_precision, 2),
        "recall_percent": round(100 * recall, 2),
        "f1_percent": round(100 * f1, 2),
    }


def grouped_quality(
    cases: list[dict[str, object]], predictions: dict[str, bool], key: str
) -> dict[str, dict[str, object]]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        groups[str(case[key])].append(case)
    return {
        name: quality_metrics(group_cases, predictions)
        for name, group_cases in sorted(groups.items())
    }


def untagged_plan_metrics(
    cases: list[dict[str, object]],
    diagnostics: dict[str, dict[str, object]],
) -> dict[str, object]:
    positive_cases = [case for case in cases if case["expected"]]
    positive_diagnostics = [diagnostics[case["id"]] for case in positive_cases]
    total = len(positive_diagnostics)
    if total == 0:
        raise ValueError("untagged plan metrics require positive cases")

    def summarize(field: str) -> tuple[int, float]:
        count = sum(bool(diagnostic[field]) for diagnostic in positive_diagnostics)
        return count, round(100 * count / total, 2)

    expected_count, expected_percent = summarize("expected_pos_present")
    multi_count, multi_percent = summarize("multi_coarse_pos")
    literal_count, literal_percent = summarize("literal_fallback")
    return {
        "positive_cases": total,
        "expected_pos_present": expected_count,
        "expected_pos_present_percent": expected_percent,
        "multi_coarse_pos": multi_count,
        "multi_coarse_pos_percent": multi_percent,
        "literal_fallback": literal_count,
        "literal_fallback_percent": literal_percent,
    }


def product_workflows(
    boundary_comparison: dict[str, object], human_untagged: dict[str, object]
) -> dict[str, object]:
    agent = boundary_comparison["profiles"]["embedded"]["any"]
    human_profile = human_untagged["profiles"]["full-pos"]
    human = human_profile["boundaries"]["smart"]
    return {
        "agent": {
            "input": "explicit POS",
            "lexicon": "embedded",
            "boundary": "any",
            "quality": agent["quality"],
            "performance": agent["performance"],
            "primary_metrics": ["recall_percent", "cases_per_second"],
        },
        "human": {
            "input": "untagged",
            "lexicon": "full-pos",
            "boundary": "smart",
            "quality": human["quality"],
            "performance": human["performance"],
            "plan": human_profile["plan"],
            "primary_metrics": [
                "precision_percent",
                "recall_percent",
                "expected_pos_present_percent",
            ],
        },
        "library": {
            "default": "embedded engine without optional resources",
            "optional": ["full-pos lexicon", "component resource"],
        },
    }


def product_persona_comparison(
    boundary_comparison: dict[str, object],
    user_result: dict[str, object],
    user_plan: dict[str, object],
    dataset: dict[str, object],
) -> dict[str, object]:
    agent = boundary_comparison["profiles"]["embedded"]["any"]
    return {
        "task": "persona-adjusted sentence lemma/POS presence",
        "gold": "explicit lemma/POS with positive gold-span overlap",
        "dataset": dataset,
        "rows": {
            "agent": {
                "label": "Agent",
                "input": "explicit POS",
                "lexicon": "embedded",
                "boundary": "any",
                "quality": agent["quality"],
                "performance": agent["performance"],
            },
            "user": {
                "label": "User",
                "input": "POS omitted",
                "lexicon": "full-pos",
                "boundary": "smart",
                "quality": user_result["quality"],
                "performance": user_result["performance"],
                "plan": user_plan,
            },
        },
    }


def kfind_profile_comparison(
    cases: list[dict[str, object]],
    predictions: dict[str, dict[str, bool]],
    matches: dict[str, dict[str, list[dict[str, object]]]],
) -> dict[str, list[dict[str, object]]]:
    recovered = []
    still_failing = []
    regressed = []
    for case in cases:
        if not case["expected"]:
            continue
        record = {
            "case": case,
            "matching_spans": {
                profile: matches[profile][case["id"]] for profile in KFIND_PROFILES
            },
        }
        embedded_prediction = predictions["kfind-embedded"][case["id"]]
        full_pos_prediction = predictions["kfind-full-pos"][case["id"]]
        if not embedded_prediction and full_pos_prediction:
            recovered.append(record)
        elif not embedded_prediction:
            still_failing.append(record)
        elif not full_pos_prediction:
            regressed.append(record)
    return {
        "recovered_with_full_pos": recovered,
        "still_failing_with_full_pos": still_failing,
        "regressed_with_full_pos": regressed,
    }


def build_report(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    versions: dict[str, dict[str, object]],
    predictions: dict[str, dict[str, bool]],
    matches: dict[str, dict[str, list[dict[str, object]]]],
    performance_metrics: dict[str, dict[str, object]],
    kfind_diagnostics: dict[str, dict[str, dict[str, object] | None]],
    shadow_verification: dict[str, dict[str, dict[str, object]]],
    include_performance: bool = True,
) -> dict[str, object]:
    backends = tuple(predictions)
    if tuple(versions) != backends or tuple(matches) != backends:
        raise ValueError("versions, predictions, and matches must use the same backends")
    reference_backends = tuple(
        backend for backend in backends if backend not in KFIND_PROFILES
    )
    quality = {}
    has_slices = all("slice" in case for case in cases)
    for backend in backends:
        backend_quality = {
            "overall": quality_metrics(cases, predictions[backend]),
            "by_source": grouped_quality(cases, predictions[backend], "source"),
            "by_pos": grouped_quality(cases, predictions[backend], "pos"),
        }
        if has_slices:
            backend_quality["by_slice"] = grouped_quality(
                cases, predictions[backend], "slice"
            )
        if all("target_raw_tag" in case for case in cases):
            backend_quality["by_target_raw_tag"] = grouped_quality(
                cases, predictions[backend], "target_raw_tag"
            )
        if all("target_group" in case for case in cases):
            backend_quality["by_target_group"] = grouped_quality(
                cases, predictions[backend], "target_group"
            )
        quality[backend] = backend_quality
    failures = []
    for case in cases:
        backend_predictions = {
            backend: predictions[backend][case["id"]] for backend in backends
        }
        if all(value == case["expected"] for value in backend_predictions.values()):
            continue
        profile_causes = {
            profile: classify_primary_cause(
                case,
                backend_predictions,
                profile,
                matches[profile][case["id"]],
                kfind_diagnostics[profile][case["id"]],
                reference_backends,
            )
            for profile in KFIND_PROFILES
        }
        profile_cause_evidence = {
            profile: kfind_diagnostics[profile][case["id"]]
            for profile in KFIND_PROFILES
        }
        failures.append(
            {
                "case": case,
                "predictions": backend_predictions,
                "primary_cause": profile_causes["kfind-embedded"],
                "cause_evidence": profile_cause_evidence["kfind-embedded"],
                "profile_causes": profile_causes,
                "profile_cause_evidence": profile_cause_evidence,
                "matching_spans": {
                    backend: matches[backend][case["id"]] for backend in backends
                },
            }
        )
    return {
        "schema_version": 15,
        "task": "sentence lemma/POS presence with positive gold-span overlap",
        "dataset": metadata,
        "backends": list(backends),
        "reference_backends": list(reference_backends),
        "versions": versions,
        "environment": environment_metadata(),
        "quality": quality,
        "performance": performance_metrics if include_performance else None,
        "kfind_profile_comparison": kfind_profile_comparison(
            cases, predictions, matches
        ),
        "shadow_verification": {
            profile: shadow_verification_summary(
                shadow_verification[profile], cases
            )
            for profile in KFIND_PROFILES
        },
        "failures": failures,
        "adapter_errors": [],
    }


def classify_primary_cause(
    case: dict[str, object],
    predictions: dict[str, bool],
    profile: str,
    profile_spans: list[dict[str, object]],
    diagnostic: dict[str, object] | None,
    reference_backends: tuple[str, ...] = ("kiwi", "lindera"),
) -> str | None:
    if profile not in KFIND_PROFILES:
        raise ValueError(f"unknown kfind profile {profile}")
    if not case["expected"] or predictions[profile]:
        return None
    if len(reference_backends) >= 2 and all(
        not predictions[backend] for backend in reference_backends
    ):
        return "gold-or-adapter"
    if diagnostic is None:
        raise ValueError(f"missing kfind diagnostic for positive case {case['id']}")
    if not diagnostic["auto_has_expected_pos_analysis"]:
        return "lexicon-missing"
    if profile_spans:
        return "span-mismatch"
    if diagnostic["any_boundary_gold_overlap"]:
        return "boundary-rejected"
    if diagnostic["gold_anchor_overlap"]:
        return "continuation-rejected"
    return "surface-missing"


def environment_metadata() -> dict[str, object]:
    memory_kib = None
    try:
        with open("/proc/meminfo", encoding="utf-8") as meminfo:
            total = next(
                (line for line in meminfo if line.startswith("MemTotal:")), None
            )
        if total is not None:
            memory_kib = int(total.split()[1])
    except FileNotFoundError:
        pass
    return {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "logical_cpu_count": os.cpu_count(),
        "memory_kib": memory_kib,
        "python": platform.python_version(),
    }


def render_markdown(report: dict[str, object]) -> str:
    dataset = report["dataset"]
    backends = report.get("backends", list(BACKENDS))
    lines = [
        "# Held-out morphology benchmark",
        "",
        f"- fixture: `{dataset['fixture_sha256']}`",
        f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
        f"{dataset['negative_cases']} negative)",
        f"- seed: `{dataset['seed']}`",
        f"- UD release: {dataset['ud_release']}",
        f"- environment: `{report['environment']['platform']}` / "
        f"{report['environment']['logical_cpu_count']} logical CPUs",
        "",
        "## Sources",
        "",
        "| source | license | SHA-256 |",
        "| --- | --- | --- |",
    ]
    for source in dataset["sources"]:
        lines.append(
            f"| [{source['name']}]({source['data_url']}) | {source['license']} | "
            f"`{source['data_sha256']}` |"
        )
    lines.extend(
        [
            "",
            "## Versions and profiles",
            "",
            "| result | backend | version | profile | lexicon SHA-256 | enriched SHA-256 | morphology SHA-256 | component SHA-256 |",
            "| --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for result_name in backends:
        version = report["versions"][result_name]
        artifact = version["lexicon_artifact_sha256"] or "n/a"
        enriched = version.get("enriched_artifact_sha256") or "n/a"
        morphology = version.get("morphology_artifact_sha256") or "n/a"
        component = version.get("component_artifact_sha256") or "n/a"
        lines.append(
            f"| {result_name} | {version['backend']} | {version['version']} | "
            f"{version['profile'] or 'n/a'} | `{artifact}` | `{enriched}` | "
            f"`{morphology}` | `{component}` |"
        )
    append_product_workflows(lines, report)
    append_external_baselines(lines, report)
    append_product_use_cases(lines, report.get("product_use_cases"))
    append_agent_precision_shadow(lines, report.get("agent_precision_shadow"))
    append_quality_sections(lines, report)
    append_boundary_comparison(lines, report.get("boundary_comparison"))
    append_human_untagged(lines, report.get("human_untagged"))
    append_component_startup(lines, report.get("component_startup"))
    append_shadow_verification(lines, report)
    append_profile_comparison(lines, report)
    append_failures(lines, report)
    append_development_summary(lines, report.get("development"))
    append_hard_negative_summary(lines, report.get("hard_negatives"))
    lines.extend(
        [
            "",
            "The current run measures kfind. External analyzer quality and performance are "
            "pinned snapshots captured by an explicit refresh against the same fixture and "
            "workload.",
        ]
    )
    return "\n".join(lines) + "\n"


def append_agent_precision_shadow(
    lines: list[str], shadow: dict[str, object] | None
) -> None:
    if shadow is None:
        return
    lines.extend(
        [
            "",
            "## Agent precision shadow",
            "",
            "This diagnostic runs after timed evaluation and does not change `any` matches.",
            "",
            "| dataset | policy | TP | FP | FN | precision | recall | F1 |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for dataset, result in shadow.items():
        for policy, metrics in result["projections"].items():
            lines.append(
                f"| {dataset} | {policy} | {metrics['true_positive']} | "
                f"{metrics['false_positive']} | {metrics['false_negative']} | "
                f"{metrics['precision']:.2%} | {metrics['recall']:.2%} | "
                f"{metrics['f1']:.2%} |"
            )

    lines.extend(
        [
            "",
            "| dataset | path presence | gold overlap | negative span | other span |",
            "| --- | --- | ---: | ---: | ---: |",
        ]
    )
    for dataset, result in shadow.items():
        for path_presence, outcomes in result["by_path_presence"].items():
            lines.append(
                f"| {dataset} | {path_presence} | "
                f"{outcomes.get('gold-overlap', 0)} | "
                f"{outcomes.get('negative-span', 0)} | "
                f"{outcomes.get('positive-other-span', 0)} |"
            )


def append_product_workflows(lines: list[str], report: dict[str, object]) -> None:
    workflows = report.get("product_workflows")
    if workflows is None:
        return
    lines.extend(
        [
            "",
            "## Product workflows",
            "",
            "Agent search prioritizes recall and throughput; false positives are candidates "
            "for context inspection. Human search prioritizes precise untagged results.",
            "",
            "| workflow | input | lexicon | boundary | precision | recall | F1 | FP candidates | cases/s |",
            "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name in ("agent", "human"):
        workflow = workflows[name]
        quality = workflow["quality"]
        performance = workflow["performance"]
        lines.append(
            f"| {name} | {workflow['input']} | {workflow['lexicon']} | "
            f"{workflow['boundary']} | {quality['precision_percent']}% | "
            f"{quality['recall_percent']}% | {quality['f1_percent']}% | "
            f"{quality['fp']} | {performance['cases_per_second']} |"
        )
    human_plan = workflows["human"]["plan"]
    lines.extend(
        [
            "",
            f"- human intended-POS plan coverage: "
            f"{human_plan['expected_pos_present_percent']}%",
            f"- library default: {workflows['library']['default']}",
            "- library optional resources: "
            + ", ".join(workflows["library"]["optional"]),
            "- workflows are not combined into one score",
        ]
    )


def append_product_use_cases(
    lines: list[str], use_cases: dict[str, object] | None
) -> None:
    if use_cases is None:
        return
    corpus = use_cases["corpus"]
    lines.extend(
        [
            "",
            "## Product CLI use cases",
            "",
            "Fresh-process CLI measurements include startup, query compilation, filesystem "
            "walk, scan, verification, and output serialization.",
            "",
            f"- profile: {use_cases['profile']}",
            f"- corpus: {corpus['bytes']} bytes across {corpus['files']} files; "
            f"SHA-256 `{corpus['sha256']}`",
            f"- cache: {use_cases['cache']}",
            "",
            "| workflow | output | wall | throughput | peak RSS | matching lines |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for name in ("agent", "human"):
        workflow = use_cases["workflows"][name]
        performance = workflow["performance"]
        lines.append(
            f"| {name} | {workflow['output']} | "
            f"{performance['wall_seconds']:.4f}s "
            f"[{performance['run_min']['wall_seconds']:.4f}, "
            f"{performance['run_max']['wall_seconds']:.4f}] | "
            f"{performance['throughput_mib_s']:.2f} MiB/s | "
            f"{format_rss(performance['peak_rss_kib'])} | "
            f"{workflow['matching_lines']} |"
        )
    lines.extend(
        [
            "",
            f"- agent command: `{use_cases['workflows']['agent']['command']}`",
            f"- human command: `{use_cases['workflows']['human']['command']}`",
            "- library resource combinations are reported separately under optional "
            "component startup",
        ]
    )


def append_external_baselines(lines: list[str], report: dict[str, object]) -> None:
    snapshot = report.get("external_baselines")
    if snapshot is None:
        return
    lines.extend(
        [
            "",
            "## Product persona and external comparison",
            "",
            "All rows use the same 1,000-case explicit-POS fixture and gold. Agent keeps "
            "explicit POS, User omits POS with full-POS + smart, and external analyzers "
            "keep explicit POS. Agent and User are measured in the current run; external "
            "rows are pinned snapshots. Every performance row uses one discarded warm-up "
            "and five measured fresh processes.",
            "",
            "This is a persona-adjusted product comparison, not an identical-input backend "
            "ranking. The User row includes query planning and ambiguity, while the "
            "explicit-POS gold counts matches for another POS as errors.",
            "",
            "| backend | precision | recall | F1 | init median | cases/s median | "
            "p95 median | peak RSS |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    persona = report.get("product_persona_comparison")
    quality_by_backend = report.get("quality")
    performance_by_backend = snapshot.get("performance")
    if (
        persona is not None
        and quality_by_backend is not None
        and performance_by_backend is not None
    ):
        for name in ("agent", "user"):
            row = persona["rows"][name]
            append_comparison_row(
                lines, row["label"], row["quality"], row["performance"]
            )
        for backend, performance in performance_by_backend.items():
            quality = quality_by_backend[backend]["overall"]
            append_comparison_row(lines, backend, quality, performance)
    else:
        lines.append("| unavailable | n/a | n/a | n/a | n/a | n/a | n/a | n/a |")

    environment = snapshot.get("environment")
    if environment is not None:
        lines.extend(
            [
                "",
                f"- external snapshot environment: {environment['platform']}; "
                f"{environment['logical_cpus']} logical CPUs; Python "
                f"{environment['python']}",
            ]
        )
    lines.extend(
        [
            "- the separate Human untagged section uses production-like negatives and is "
            "not part of this comparison",
            "",
            "### External snapshot ranges",
            "",
            "| backend | status | runs | init [min, max] | cases/s [min, max] | "
            "p95 [min, max] | peak RSS [min, max] |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend, availability in snapshot["availability"].items():
        status = availability["status"]
        if availability.get("reason"):
            status += f": {availability['reason']}"
        performance = (performance_by_backend or {}).get(backend)
        if performance is None:
            lines.append(f"| {backend} | {status} | n/a | n/a | n/a | n/a | n/a |")
            continue
        minimum = performance["run_min"]
        maximum = performance["run_max"]
        lines.append(
            f"| {backend} | {status} | {performance['runs']} | "
            f"{performance['initialization_seconds']:.4f}s "
            f"[{minimum['initialization_seconds']:.4f}, "
            f"{maximum['initialization_seconds']:.4f}] | "
            f"{performance['cases_per_second']} "
            f"[{minimum['cases_per_second']}, {maximum['cases_per_second']}] | "
            f"{performance['latency_p95_ms']}ms "
            f"[{minimum['latency_p95_ms']}, {maximum['latency_p95_ms']}] | "
            f"{format_rss(performance['peak_rss_kib'])} "
            f"[{format_rss(minimum['peak_rss_kib'])}, "
            f"{format_rss(maximum['peak_rss_kib'])}] |"
        )


def append_comparison_row(
    lines: list[str],
    backend: str,
    quality: dict[str, object],
    performance: dict[str, object],
) -> None:
    lines.append(
        f"| {backend} | {quality['precision_percent']}% | "
        f"{quality['recall_percent']}% | {quality['f1_percent']}% | "
        f"{performance['initialization_seconds']:.4f}s | "
        f"{performance['cases_per_second']} | "
        f"{performance['latency_p95_ms']}ms | "
        f"{format_rss(performance['peak_rss_kib'])} |"
    )


def append_quality_sections(lines: list[str], report: dict[str, object]) -> None:
    lines.extend(
        [
            "",
            "## Overall quality",
            "",
            "| backend | accuracy | precision | recall | F1 | TP | FP | TN | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in report["backends"]:
        metrics = report["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['accuracy_percent']}% | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | {metrics['tp']} | "
            f"{metrics['fp']} | {metrics['tn']} | {metrics['fn']} |"
        )
    append_performance(lines, report)
    append_grouped_quality(lines, report, "source", "by_source")
    append_grouped_quality(lines, report, "POS", "by_pos")


def append_performance(lines: list[str], report: dict[str, object]) -> None:
    lines.extend(
        [
            "",
            "## End-to-end performance",
            "",
            "| backend | runs | init median | cases/s median [min, max] | p95 median [min, max] | peak RSS median [min, max] |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in report["performance"]:
        metrics = report["performance"][backend]
        rss = metrics["peak_rss_kib"]
        rss_text = f"{rss / 1024:.1f} MiB" if rss is not None else "n/a"
        rss_min = metrics["run_min"]["peak_rss_kib"]
        rss_max = metrics["run_max"]["peak_rss_kib"]
        rss_range = (
            f"[{rss_min / 1024:.1f}, {rss_max / 1024:.1f}] MiB"
            if rss_min is not None and rss_max is not None
            else "n/a"
        )
        lines.append(
            f"| {backend} | {metrics['runs']} | {metrics['initialization_seconds']:.4f}s | "
            f"{metrics['cases_per_second']} "
            f"[{metrics['run_min']['cases_per_second']}, {metrics['run_max']['cases_per_second']}] | "
            f"{metrics['latency_p95_ms']}ms "
            f"[{metrics['run_min']['latency_p95_ms']}, {metrics['run_max']['latency_p95_ms']}] | "
            f"{rss_text} {rss_range} |"
        )


def append_boundary_comparison(
    lines: list[str], comparison: dict[str, object] | None
) -> None:
    if comparison is None:
        return
    lines.extend(
        [
            "",
            "## Boundary policy comparison",
            "",
            "The same fixture is compiled and matched for every profile. Smart loads the "
            "component resource; token and any do not.",
            "",
            "| profile | boundary | precision | recall | F1 | init median | cases/s median [min, max] | p95 median [min, max] | peak RSS |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile, results in comparison["profiles"].items():
        for boundary in comparison["boundaries"]:
            result = results[boundary]
            quality = result["quality"]
            performance = result["performance"]
            rss = performance["peak_rss_kib"]
            rss_text = f"{rss / 1024:.1f} MiB" if rss is not None else "n/a"
            lines.append(
                f"| {profile} | {boundary} | {quality['precision_percent']}% | "
                f"{quality['recall_percent']}% | {quality['f1_percent']}% | "
                f"{performance['initialization_seconds']:.4f}s | "
                f"{performance['cases_per_second']} "
                f"[{performance['run_min']['cases_per_second']}, "
                f"{performance['run_max']['cases_per_second']}] | "
                f"{performance['latency_p95_ms']}ms "
                f"[{performance['run_min']['latency_p95_ms']}, "
                f"{performance['run_max']['latency_p95_ms']}] | {rss_text} |"
            )


def append_component_startup(
    lines: list[str], startup: dict[str, dict[str, object]] | None
) -> None:
    if startup is None:
        return
    lines.extend(
        [
            "",
            "## Optional component startup",
            "",
            "Each profile runs in a fresh process after one discarded warm-up. Component profiles "
            "construct the resource-less engine first, then explicitly load the component asset.",
            "",
            "| profile | runs | base init median [min, max] | component load median [min, max] | total init median [min, max] | base peak RSS | final peak RSS |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile, metrics in startup.items():
        lines.append(
            f"| {profile} | {metrics['runs']} | "
            f"{format_seconds(metrics, 'base_initialization_seconds')} | "
            f"{format_seconds(metrics, 'component_initialization_seconds')} | "
            f"{format_seconds(metrics, 'initialization_seconds')} | "
            f"{format_rss(metrics['base_peak_rss_kib'])} | "
            f"{format_rss(metrics['peak_rss_kib'])} |"
        )


def append_human_untagged(
    lines: list[str], human: dict[str, object] | None
) -> None:
    if human is None:
        return
    dataset = human["dataset"]
    lines.extend(
        [
            "",
            "## Human untagged search",
            "",
            "The query is compiled without a global POS or atom tag. A negative sentence "
            "contains no supported POS analysis for the query lemma.",
            "",
            f"- fixture: `{dataset['fixture_sha256']}`",
            f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
            f"{dataset['negative_cases']} negative)",
            "",
            "| profile | boundary | precision | recall | F1 | TP | FP | TN | FN | init median | cases/s median | p95 median | peak RSS |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile, profile_result in human["profiles"].items():
        for boundary in human["boundaries"]:
            result = profile_result["boundaries"][boundary]
            quality = result["quality"]
            performance = result["performance"]
            lines.append(
                f"| {profile} | {boundary} | {quality['precision_percent']}% | "
                f"{quality['recall_percent']}% | {quality['f1_percent']}% | "
                f"{quality['tp']} | {quality['fp']} | {quality['tn']} | "
                f"{quality['fn']} | {performance['initialization_seconds']:.4f}s | "
                f"{performance['cases_per_second']} | "
                f"{performance['latency_p95_ms']}ms | "
                f"{format_rss(performance['peak_rss_kib'])} |"
            )
    lines.extend(
        [
            "",
            "| profile | positive plans | intended POS present | multi-POS plan | literal fallback |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile, profile_result in human["profiles"].items():
        plan = profile_result["plan"]
        lines.append(
            f"| {profile} | {plan['positive_cases']} | "
            f"{plan['expected_pos_present_percent']}% "
            f"({plan['expected_pos_present']}) | "
            f"{plan['multi_coarse_pos_percent']}% ({plan['multi_coarse_pos']}) | "
            f"{plan['literal_fallback_percent']}% ({plan['literal_fallback']}) |"
        )


def format_seconds(metrics: dict[str, object], name: str) -> str:
    value = metrics[name]
    if value is None:
        return "n/a"
    return (
        f"{value:.4f}s [{metrics['run_min'][name]:.4f}, "
        f"{metrics['run_max'][name]:.4f}]"
    )


def format_rss(value: int | float | None) -> str:
    return f"{value / 1024:.1f} MiB" if value is not None else "n/a"


def append_grouped_quality(
    lines: list[str], report: dict[str, object], label: str, key: str
) -> None:
    lines.extend(
        [
            "",
            f"## Quality by {label}",
            "",
            f"| {label} | backend | accuracy | precision | recall | F1 |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    backends = report["backends"]
    groups = sorted(report["quality"][backends[0]][key])
    for group in groups:
        for backend in backends:
            metrics = report["quality"][backend][key][group]
            lines.append(
                f"| {group} | {backend} | {metrics['accuracy_percent']}% | "
                f"{metrics['precision_percent']}% | {metrics['recall_percent']}% | "
                f"{metrics['f1_percent']}% |"
            )


def append_profile_comparison(lines: list[str], report: dict[str, object]) -> None:
    comparison = report["kfind_profile_comparison"]
    lines.extend(
        [
            "",
            "## kfind full-POS recovery",
            "",
            f"- recovered: {len(comparison['recovered_with_full_pos'])}",
            f"- still failing: {len(comparison['still_failing_with_full_pos'])}",
            f"- regressed: {len(comparison['regressed_with_full_pos'])}",
        ]
    )


def append_failures(lines: list[str], report: dict[str, object]) -> None:
    backends = report["backends"]
    lines.extend(
        [
            "",
            f"## Failures ({len(report['failures'])} cases)",
            "",
            "| case | source | query/POS | embedded cause | full-POS cause | expected | "
            + " | ".join(backends)
            + " |",
            "| --- | --- | --- | --- | --- | --- | "
            + " | ".join("---" for _ in backends)
            + " |",
        ]
    )
    for failure in report["failures"][:30]:
        case = failure["case"]
        predicted = failure["predictions"]
        lines.append(
            f"| {case['id']} | {case['source']} | {case['query']}/{case['pos']} | "
            f"{failure['profile_causes']['kfind-embedded'] or 'n/a'} | "
            f"{failure['profile_causes']['kfind-full-pos'] or 'n/a'} | "
            f"{case['expected']} | "
            + " | ".join(str(predicted[backend]) for backend in backends)
            + " |"
        )
    if len(report["failures"]) > 30:
        lines.extend(["", "The JSON report contains every failure and matching span."])


def append_development_summary(
    lines: list[str], development: dict[str, object] | None
) -> None:
    if development is None:
        return
    dataset = development["dataset"]
    lines.extend(
        [
            "",
            "## Development split",
            "",
            f"- fixture: `{dataset['fixture_sha256']}`",
            f"- cases: {dataset['cases']}",
            "",
            "| backend | precision | recall | F1 | TP | FP | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in development.get("backends", list(BACKENDS)):
        metrics = development["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | "
            f"{metrics['tp']} | {metrics['fp']} | {metrics['fn']} |"
        )
    append_development_failure_diagnostics(lines, development)
    append_component_shadow_table(lines, development["shadow_verification"])


def append_development_failure_diagnostics(
    lines: list[str], development: dict[str, object]
) -> None:
    full_pos_failures = [
        failure
        for failure in development["failures"]
        if failure["case"]["expected"]
        and not failure["predictions"]["kfind-full-pos"]
    ]
    counts: dict[tuple[str, str], int] = defaultdict(int)
    for failure in full_pos_failures:
        key = (
            failure["profile_causes"]["kfind-full-pos"],
            failure["case"]["pos"],
        )
        counts[key] += 1
    lines.extend(
        [
            "",
            "### full-POS positive false negatives",
            "",
            "| primary cause | POS | cases |",
            "| --- | --- | ---: |",
        ]
    )
    for (cause, pos), count in sorted(counts.items()):
        lines.append(f"| {cause} | {pos} | {count} |")

    predicate_failures = [
        failure
        for failure in full_pos_failures
        if failure["profile_causes"]["kfind-full-pos"] == "boundary-rejected"
        and failure["case"]["pos"] in {"verb", "adjective"}
    ]
    lines.extend(
        [
            "",
            "### Predicate boundary-rejected slice",
            "",
            "| case | query/POS | gold surface | any-boundary rule paths |",
            "| --- | --- | --- | --- |",
        ]
    )
    for failure in predicate_failures:
        case = failure["case"]
        evidence = failure["profile_cause_evidence"]["kfind-full-pos"]
        paths = {
            tuple(origin["rule_path"])
            for matched in evidence["any_boundary_gold_matches"]
            for origin in matched["origins"]
        }
        if not paths:
            raise ValueError(
                f"boundary-rejected predicate {case['id']} has no rule-path evidence"
            )
        rendered_paths = "<br>".join(
            " -> ".join(path) if path else "(lexical)" for path in sorted(paths)
        )
        gold_bytes = case["text"].encode("utf-8")[
            case["gold_byte_start"] : case["gold_byte_end"]
        ]
        gold_surface = gold_bytes.decode("utf-8")
        lines.append(
            f"| {case['id']} | {case['query']}/{case['pos']} | {gold_surface} | "
            f"{rendered_paths} |"
        )

    append_connective_ji_position_evidence(lines, predicate_failures)


def strict_subspan_position(
    gold_start: int, gold_end: int, token_start: int, token_end: int
) -> str:
    if not gold_start <= token_start < token_end <= gold_end:
        raise ValueError("any-boundary token is not contained in the gold span")
    if token_start == gold_start and token_end == gold_end:
        raise ValueError("any-boundary token is not a strict gold subspan")
    if token_start == gold_start:
        return "left-edge"
    if token_end == gold_end:
        return "right-edge"
    return "internal"


def append_connective_ji_position_evidence(
    lines: list[str], predicate_failures: list[dict[str, object]]
) -> None:
    rows: set[tuple[str, str, str, str, str]] = set()
    for failure in predicate_failures:
        case = failure["case"]
        evidence = failure["profile_cause_evidence"]["kfind-full-pos"]
        text_bytes = case["text"].encode("utf-8")
        gold_start = case["gold_byte_start"]
        gold_end = case["gold_byte_end"]
        gold_surface = text_bytes[gold_start:gold_end].decode("utf-8")
        for matched in evidence["any_boundary_gold_matches"]:
            has_connective_ji = any(
                tuple(origin["rule_path"]) == CONNECTIVE_JI_PATH
                for origin in matched["origins"]
            )
            if not has_connective_ji:
                continue
            token = matched["token"]
            try:
                position = strict_subspan_position(
                    gold_start,
                    gold_end,
                    token["byte_start"],
                    token["byte_end"],
                )
            except ValueError as error:
                raise ValueError(
                    f"connective-ji case {case['id']}: {error}"
                ) from error
            token_surface = text_bytes[
                token["byte_start"] : token["byte_end"]
            ].decode("utf-8")
            rows.add(
                (
                    case["id"],
                    f"{case['query']}/{case['pos']}",
                    gold_surface,
                    token_surface,
                    position,
                )
            )
    if not rows:
        return

    case_ids_by_position: dict[str, set[str]] = {
        position: set() for position in STRICT_SUBSPAN_POSITIONS
    }
    for case_id, _, _, _, position in rows:
        case_ids_by_position[position].add(case_id)
    lines.extend(
        [
            "",
            "### `ending.connective-ji` position evidence",
            "",
            "| any token position | cases |",
            "| --- | ---: |",
        ]
    )
    for position in STRICT_SUBSPAN_POSITIONS:
        lines.append(f"| {position} | {len(case_ids_by_position[position])} |")
    lines.extend(
        [
            "",
            "| case | query/POS | gold surface | any token | position |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for case_id, query_pos, gold_surface, token_surface, position in sorted(rows):
        lines.append(
            f"| {case_id} | {query_pos} | {gold_surface} | {token_surface} | "
            f"{position} |"
        )


def append_hard_negative_summary(
    lines: list[str], hard_negatives: dict[str, object] | None
) -> None:
    if hard_negatives is None:
        return
    lines.extend(
        [
            "",
            "## Hard negatives",
            "",
            f"- fixture: `{hard_negatives['dataset']['fixture_sha256']}`",
            f"- cases: {hard_negatives['dataset']['cases']}",
            "",
            "| slice | backend | hard-negative precision | FP | TN |",
            "| --- | --- | ---: | ---: | ---: |",
        ]
    )
    slices = sorted(hard_negatives["quality"]["kfind-embedded"]["by_slice"])
    for slice_name in slices:
        for backend in hard_negatives.get("backends", list(BACKENDS)):
            metrics = hard_negatives["quality"][backend]["by_slice"][slice_name]
            lines.append(
                f"| {slice_name} | {backend} | "
                f"{metrics['hard_negative_precision_percent']}% | "
                f"{metrics['fp']} | {metrics['tn']} |"
            )
    append_component_shadow_table(lines, hard_negatives["shadow_verification"])
