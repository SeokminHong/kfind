#!/usr/bin/env python3

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


BACKENDS = (
    "kfind-embedded",
    "kfind-full-pos",
    "kiwi",
    "lindera",
)
COLORS = {
    "kfind-embedded": "#2563eb",
    "kfind-full-pos": "#7c3aed",
    "kiwi": "#059669",
    "lindera": "#d97706",
    "mecab-ko": "#dc2626",
    "komoran": "#0891b2",
}
BOUNDARY_SERIES = (
    ("embedded", "smart"),
    ("embedded", "token"),
    ("embedded", "any"),
    ("full-pos", "smart"),
    ("full-pos", "token"),
    ("full-pos", "any"),
)
BOUNDARY_COLORS = {
    ("embedded", "smart"): "#2563eb",
    ("full-pos", "smart"): "#60a5fa",
    ("embedded", "token"): "#d97706",
    ("full-pos", "token"): "#fbbf24",
    ("embedded", "any"): "#4d7c0f",
    ("full-pos", "any"): "#a3e635",
}
HUMAN_UNTAGGED_SERIES = (
    ("embedded", "smart"),
    ("embedded", "any"),
    ("full-pos", "smart"),
    ("full-pos", "any"),
)
STYLE = """
<style>
  .background { fill: #ffffff; }
  text { fill: #111827; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .muted { fill: #4b5563; }
  .grid { stroke: #d1d5db; stroke-width: 1; }
  .axis { stroke: #6b7280; stroke-width: 1; }
  @media (prefers-color-scheme: dark) {
    .background { fill: #0d1117; }
    text { fill: #f3f4f6; }
    .muted { fill: #9ca3af; }
    .grid { stroke: #374151; }
    .axis { stroke: #9ca3af; }
  }
</style>
"""


def text(x: float, y: float, value: object, css_class: str = "", anchor: str = "start") -> str:
    attributes = f' class="{css_class}"' if css_class else ""
    return (
        f'<text x="{x:.1f}" y="{y:.1f}" text-anchor="{anchor}"{attributes}>'
        f"{html.escape(str(value))}</text>"
    )


def rect(x: float, y: float, width: float, height: float, fill: str, radius: int = 3) -> str:
    return (
        f'<rect x="{x:.1f}" y="{y:.1f}" width="{max(width, 0):.1f}" '
        f'height="{height:.1f}" rx="{radius}" fill="{fill}"/>'
    )


def metric_value(value: float, unit: str) -> str:
    if unit in {"cases/s", "MiB"}:
        return f"{value:.1f}"
    return f"{value:.4f}"


def svg_document(width: int, height: int, title: str, description: str, body: list[str]) -> str:
    return "\n".join(
        [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
            f"<title id=\"title\">{html.escape(title)}</title>",
            f"<desc id=\"desc\">{html.escape(description)}</desc>",
            STYLE,
            f'<rect class="background" width="{width}" height="{height}"/>',
            *body,
            "</svg>",
            "",
        ]
    )


def render_quality(report: dict[str, object]) -> str:
    backends = tuple(report.get("backends", BACKENDS))
    width, height = 1280, 650
    left, right, top, bottom = 80, 32, 86, 120
    plot_width = width - left - right
    plot_height = height - top - bottom
    metrics = (
        ("Accuracy", "accuracy_percent"),
        ("Precision", "precision_percent"),
        ("Recall", "recall_percent"),
        ("F1", "f1_percent"),
    )
    body = [
        text(left, 38, "Held-out morphology quality", anchor="start"),
        text(left, 62, "1,000 balanced lemma/POS presence cases · percent", "muted"),
    ]
    for tick in range(0, 101, 20):
        y = top + plot_height * (1 - tick / 100)
        body.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}"/>')
        body.append(text(left - 10, y + 5, tick, "muted", "end"))
    group_width = plot_width / len(metrics)
    bar_width = 36
    gap = 10
    for group_index, (label, key) in enumerate(metrics):
        center = left + group_width * (group_index + 0.5)
        start = center - (
            len(backends) * bar_width + (len(backends) - 1) * gap
        ) / 2
        for backend_index, backend in enumerate(backends):
            value = float(report["quality"][backend]["overall"][key])
            x = start + backend_index * (bar_width + gap)
            bar_height = plot_height * value / 100
            y = top + plot_height - bar_height
            body.append(rect(x, y, bar_width, bar_height, COLORS[backend]))
            label_y = y - 8 - (backend_index % 2) * 16
            body.append(
                text(x + bar_width / 2, label_y, f"{value:.2f}", anchor="middle")
            )
        body.append(text(center, top + plot_height + 28, label, anchor="middle"))
    for index, backend in enumerate(backends):
        column = index % 3
        row = index // 3
        legend_x = left + column * 380
        legend_y = height - 54 + row * 28
        body.append(rect(legend_x, legend_y - 13, 16, 16, COLORS[backend], 2))
        body.append(text(legend_x + 24, legend_y, backend))
    return svg_document(
        width,
        height,
        "Held-out morphology quality",
        "Grouped bars compare accuracy, precision, recall, and F1 for kfind profiles and pinned external quality snapshots.",
        body,
    )


def render_performance(report: dict[str, object]) -> str:
    backends = tuple(report["performance"])
    width, height = 1280, 720
    body = [
        text(52, 38, "End-to-end performance and memory", anchor="start"),
        text(52, 62, "Single initialization · 1,000 cases · lower is better except throughput", "muted"),
    ]
    panels = (
        ("Throughput", "cases_per_second", "cases/s", True),
        ("Initialization", "initialization_seconds", "s", False),
        ("p95 latency", "latency_p95_ms", "ms", False),
        ("Peak RSS", "peak_rss_kib", "MiB", False),
    )
    panel_width = 548
    positions = ((52, 96), (680, 96), (52, 390), (680, 390))
    for (label, key, unit, higher_better), (panel_x, panel_y) in zip(panels, positions):
        values = {}
        for backend in backends:
            value = float(report["performance"][backend][key])
            if key == "peak_rss_kib":
                value /= 1024
            values[backend] = value
        maximum = max(values.values()) * 1.12
        body.append(text(panel_x, panel_y, label))
        direction = "higher is better" if higher_better else "lower is better"
        body.append(text(panel_x + panel_width, panel_y, direction, "muted", "end"))
        chart_top = panel_y + 28
        label_width = 150
        bar_max_width = panel_width - label_width - 140
        for index, backend in enumerate(backends):
            row_y = chart_top + index * 60
            value = values[backend]
            body.append(text(panel_x, row_y + 24, backend))
            bar_x = panel_x + label_width
            bar_width = bar_max_width * value / maximum if maximum else 0
            body.append(rect(bar_x, row_y + 7, bar_width, 25, COLORS[backend]))
            rendered = metric_value(value, unit)
            body.append(text(bar_x + bar_max_width + 8, row_y + 25, f"{rendered} {unit}"))
        axis_y = chart_top + len(backends) * 60
        body.append(
            f'<line class="axis" x1="{panel_x + label_width}" y1="{axis_y}" '
            f'x2="{panel_x + label_width + bar_max_width}" y2="{axis_y}"/>'
        )
    return svg_document(
        width,
        height,
        "End-to-end morphology performance",
        "Four horizontal bar panels compare throughput, initialization time, p95 latency, and peak RSS.",
        body,
    )


def render_product_workflows(report: dict[str, object]) -> str:
    workflows = report["product_workflows"]
    agent = workflows["agent"]
    human = workflows["human"]
    width, height = 1280, 660
    body = [
        text(52, 38, "Product workflow benchmark", anchor="start"),
        text(
            52,
            62,
            "Agent: embedded · any · explicit POS  |  Human: full-POS · smart · untagged",
            "muted",
        ),
    ]

    quality_x, quality_y = 52, 112
    quality_label_width = 190
    quality_bar_width = 360
    quality_rows = (
        (
            "Agent · recall",
            float(agent["quality"]["recall_percent"]),
            COLORS["kfind-embedded"],
        ),
        (
            "Human · precision",
            float(human["quality"]["precision_percent"]),
            COLORS["kfind-full-pos"],
        ),
        (
            "Human · recall",
            float(human["quality"]["recall_percent"]),
            COLORS["kfind-full-pos"],
        ),
        (
            "Human · POS plan",
            float(human["plan"]["expected_pos_present_percent"]),
            COLORS["kfind-full-pos"],
        ),
    )
    body.append(text(quality_x, quality_y - 28, "Quality and query usability · percent"))
    for tick in range(0, 101, 25):
        x = quality_x + quality_label_width + quality_bar_width * tick / 100
        body.append(
            f'<line class="grid" x1="{x:.1f}" y1="{quality_y - 8}" '
            f'x2="{x:.1f}" y2="{quality_y + 250}"/>'
        )
        body.append(text(x, quality_y + 278, tick, "muted", "middle"))
    for index, (label, value, color) in enumerate(quality_rows):
        row_y = quality_y + index * 64
        body.append(text(quality_x, row_y + 24, label))
        bar_x = quality_x + quality_label_width
        body.append(rect(bar_x, row_y + 7, quality_bar_width * value / 100, 25, color))
        body.append(text(bar_x + quality_bar_width + 12, row_y + 25, f"{value:.2f}%"))

    throughput_x, throughput_y = 700, 112
    throughput_label_width = 100
    throughput_bar_width = 350
    throughput_rows = (
        (
            "Agent",
            float(agent["performance"]["cases_per_second"]),
            COLORS["kfind-embedded"],
        ),
        (
            "Human",
            float(human["performance"]["cases_per_second"]),
            COLORS["kfind-full-pos"],
        ),
    )
    maximum_throughput = max(value for _, value, _ in throughput_rows) * 1.08
    body.append(text(throughput_x, throughput_y - 28, "Throughput · cases/s"))
    for index, (label, value, color) in enumerate(throughput_rows):
        row_y = throughput_y + index * 64
        body.append(text(throughput_x, row_y + 24, label))
        bar_x = throughput_x + throughput_label_width
        body.append(
            rect(
                bar_x,
                row_y + 7,
                throughput_bar_width * value / maximum_throughput,
                25,
                color,
            )
        )
        body.append(
            text(
                bar_x + throughput_bar_width + 12,
                row_y + 25,
                f"{value:,.1f}",
            )
        )

    initialization_y = 340
    initialization_rows = (
        (
            "Agent",
            float(agent["performance"]["initialization_seconds"]) * 1_000,
            COLORS["kfind-embedded"],
        ),
        (
            "Human",
            float(human["performance"]["initialization_seconds"]) * 1_000,
            COLORS["kfind-full-pos"],
        ),
    )
    maximum_initialization = max(value for _, value, _ in initialization_rows) * 1.08
    body.append(text(throughput_x, initialization_y - 28, "Initialization · ms"))
    for index, (label, value, color) in enumerate(initialization_rows):
        row_y = initialization_y + index * 64
        body.append(text(throughput_x, row_y + 24, label))
        bar_x = throughput_x + throughput_label_width
        body.append(
            rect(
                bar_x,
                row_y + 7,
                throughput_bar_width * value / maximum_initialization,
                25,
                color,
            )
        )
        body.append(
            text(
                bar_x + throughput_bar_width + 12,
                row_y + 25,
                f"{value:,.2f}",
            )
        )

    body.extend(
        [
            text(
                52,
                592,
                f"Agent false-positive candidates: {agent['quality']['fp']}",
            ),
            text(
                700,
                592,
                "Library default: embedded engine without optional resources",
            ),
            text(
                700,
                620,
                "Optional: full-POS lexicon and component resource",
                "muted",
            ),
        ]
    )
    return svg_document(
        width,
        height,
        "Product workflow benchmark",
        "Quality, throughput, and initialization compare the agent embedded-any workflow with the human full-POS-smart workflow.",
        body,
    )


def render_product_use_cases(report: dict[str, object]) -> str:
    workflows = report["product_use_cases"]["workflows"]
    startup = report["component_startup"]
    agent_color = COLORS["kfind-embedded"]
    human_color = COLORS["kfind-full-pos"]
    width, height = 1280, 800
    body = [
        text(52, 38, "Product use-case performance", anchor="start"),
        text(
            52,
            62,
            "Fresh CLI process · warm filesystem cache · 100 MiB across 1,000 files · one match",
            "muted",
        ),
    ]

    cli_panels = (
        (
            52,
            "Wall time · ms · lower is better",
            tuple(
                (
                    label,
                    float(workflows[key]["performance"]["wall_seconds"]) * 1_000,
                    color,
                )
                for label, key, color in (
                    ("Agent", "agent", agent_color),
                    ("Human", "human", human_color),
                )
            ),
            lambda value: f"{value:.1f} ms",
        ),
        (
            455,
            "Throughput · MiB/s · higher is better",
            tuple(
                (
                    label,
                    float(workflows[key]["performance"]["throughput_mib_s"]),
                    color,
                )
                for label, key, color in (
                    ("Agent", "agent", agent_color),
                    ("Human", "human", human_color),
                )
            ),
            lambda value: f"{value:,.1f}",
        ),
        (
            858,
            "Peak RSS · MiB · lower is better",
            tuple(
                (
                    label,
                    float(workflows[key]["performance"]["peak_rss_kib"]) / 1024,
                    color,
                )
                for label, key, color in (
                    ("Agent", "agent", agent_color),
                    ("Human", "human", human_color),
                )
            ),
            lambda value: f"{value:.1f} MiB",
        ),
    )
    for panel_x, label, rows, formatter in cli_panels:
        body.append(text(panel_x, 112, label))
        maximum = max(value for _, value, _ in rows) * 1.08
        for index, (row_label, value, color) in enumerate(rows):
            row_y = 144 + index * 64
            body.append(text(panel_x, row_y + 24, row_label))
            body.append(
                rect(
                    panel_x + 72,
                    row_y + 7,
                    198 * value / maximum,
                    25,
                    color,
                )
            )
            body.append(text(panel_x + 280, row_y + 25, formatter(value)))

    library_profiles = (
        ("embedded", "Embedded", agent_color),
        ("embedded-component", "Embedded + component", agent_color),
        ("full-pos", "Full-POS", human_color),
        ("full-pos-component", "Full-POS + component", human_color),
    )
    library_panels = (
        (
            52,
            "Library initialization · ms · lower is better",
            tuple(
                (
                    label,
                    float(startup[key]["initialization_seconds"]) * 1_000,
                    color,
                )
                for key, label, color in library_profiles
            ),
            lambda value: f"{value:.1f} ms",
        ),
        (
            680,
            "Library peak RSS · MiB · lower is better",
            tuple(
                (
                    label,
                    float(startup[key]["peak_rss_kib"]) / 1024,
                    color,
                )
                for key, label, color in library_profiles
            ),
            lambda value: f"{value:.1f} MiB",
        ),
    )
    for panel_x, label, rows, formatter in library_panels:
        body.append(text(panel_x, 374, label))
        maximum = max(value for _, value, _ in rows) * 1.08
        for index, (row_label, value, color) in enumerate(rows):
            row_y = 406 + index * 58
            body.append(text(panel_x, row_y + 24, row_label))
            body.append(
                rect(
                    panel_x + 190,
                    row_y + 7,
                    235 * value / maximum,
                    25,
                    color,
                )
            )
            body.append(text(panel_x + 435, row_y + 25, formatter(value)))

    body.extend(
        [
            rect(52, 710, 16, 16, agent_color, 2),
            text(76, 723, "Agent / embedded resource path"),
            rect(390, 710, 16, 16, human_color, 2),
            text(414, 723, "Human / full-POS resource path"),
            text(
                1228,
                723,
                "CLI wall time includes startup, scan, verification, and output serialization",
                "muted",
                "end",
            ),
        ]
    )
    return svg_document(
        width,
        height,
        "Product use-case performance",
        "Fresh-process agent and human CLI wall time, throughput, and peak RSS are shown separately from library resource initialization and memory costs.",
        body,
    )


def render_boundary_quality(report: dict[str, object]) -> str:
    width, height = 1280, 680
    left, right, top, bottom = 80, 32, 86, 116
    plot_width = width - left - right
    plot_height = height - top - bottom
    metrics = (
        ("Precision", "precision_percent"),
        ("Recall", "recall_percent"),
        ("F1", "f1_percent"),
    )
    comparison = report["boundary_comparison"]["profiles"]
    body = [
        text(left, 38, "kfind quality by lexicon profile and boundary", anchor="start"),
        text(left, 62, "1,000 balanced lemma/POS presence cases · percent", "muted"),
    ]
    for tick in range(0, 101, 20):
        y = top + plot_height * (1 - tick / 100)
        body.append(
            f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}"/>'
        )
        body.append(text(left - 10, y + 5, tick, "muted", "end"))
    group_width = plot_width / len(metrics)
    bar_width = 48
    gap = 10
    for group_index, (label, key) in enumerate(metrics):
        center = left + group_width * (group_index + 0.5)
        start = center - (
            len(BOUNDARY_SERIES) * bar_width + (len(BOUNDARY_SERIES) - 1) * gap
        ) / 2
        for series_index, (profile, boundary) in enumerate(BOUNDARY_SERIES):
            value = float(comparison[profile][boundary]["quality"][key])
            x = start + series_index * (bar_width + gap)
            bar_height = plot_height * value / 100
            y = top + plot_height - bar_height
            body.append(
                rect(x, y, bar_width, bar_height, BOUNDARY_COLORS[(profile, boundary)])
            )
            body.append(text(x + bar_width / 2, y - 8, f"{value:.2f}", anchor="middle"))
        body.append(text(center, top + plot_height + 28, label, anchor="middle"))
    for index, (profile, boundary) in enumerate(BOUNDARY_SERIES):
        column = index % 3
        row = index // 3
        legend_x = left + column * 360
        legend_y = height - 50 + row * 28
        body.append(
            rect(
                legend_x,
                legend_y - 13,
                16,
                16,
                BOUNDARY_COLORS[(profile, boundary)],
                2,
            )
        )
        body.append(text(legend_x + 24, legend_y, f"{profile} · {boundary}"))
    return svg_document(
        width,
        height,
        "kfind quality by lexicon profile and boundary",
        "Grouped bars compare precision, recall, and F1 across embedded and full-POS profiles using smart, token, and any boundaries.",
        body,
    )


def render_boundary_performance(report: dict[str, object]) -> str:
    width, height = 1280, 780
    left = 52
    body = [
        text(left, 38, "kfind performance by lexicon profile and boundary"),
        text(
            left,
            62,
            "Fresh process · 1,000 cases · lower is better except throughput",
            "muted",
        ),
    ]
    panels = (
        ("Throughput", "cases_per_second", "cases/s", True),
        ("Initialization", "initialization_seconds", "s", False),
        ("p95 latency", "latency_p95_ms", "ms", False),
        ("Peak RSS", "peak_rss_kib", "MiB", False),
    )
    positions = ((52, 96), (680, 96), (52, 430), (680, 430))
    panel_width = 548
    comparison = report["boundary_comparison"]["profiles"]
    for (label, key, unit, higher_better), (panel_x, panel_y) in zip(panels, positions):
        values = {}
        for profile, boundary in BOUNDARY_SERIES:
            value = float(comparison[profile][boundary]["performance"][key])
            if key == "peak_rss_kib":
                value /= 1024
            values[(profile, boundary)] = value
        maximum = max(values.values()) * 1.12
        body.append(text(panel_x, panel_y, label))
        direction = "higher is better" if higher_better else "lower is better"
        body.append(text(panel_x + panel_width, panel_y, direction, "muted", "end"))
        chart_top = panel_y + 28
        label_width = 168
        bar_max_width = panel_width - label_width - 130
        for index, (profile, boundary) in enumerate(BOUNDARY_SERIES):
            row_y = chart_top + index * 44
            value = values[(profile, boundary)]
            body.append(text(panel_x, row_y + 22, f"{profile} · {boundary}"))
            bar_x = panel_x + label_width
            bar_width = bar_max_width * value / maximum if maximum else 0
            body.append(
                rect(
                    bar_x,
                    row_y + 6,
                    bar_width,
                    24,
                    BOUNDARY_COLORS[(profile, boundary)],
                )
            )
            rendered = metric_value(value, unit)
            body.append(text(bar_x + bar_max_width + 8, row_y + 24, f"{rendered} {unit}"))
        axis_y = chart_top + len(BOUNDARY_SERIES) * 44
        body.append(
            f'<line class="axis" x1="{panel_x + label_width}" y1="{axis_y}" '
            f'x2="{panel_x + label_width + bar_max_width}" y2="{axis_y}"/>'
        )
    return svg_document(
        width,
        height,
        "kfind performance by lexicon profile and boundary",
        "Four horizontal bar panels compare throughput, initialization time, p95 latency, and peak RSS across six kfind configurations.",
        body,
    )


def render_human_untagged_quality(report: dict[str, object]) -> str:
    width, height = 1120, 620
    left, right, top, bottom = 80, 32, 86, 116
    plot_width = width - left - right
    plot_height = height - top - bottom
    metrics = (
        ("Precision", "precision_percent"),
        ("Recall", "recall_percent"),
        ("F1", "f1_percent"),
    )
    human = report["human_untagged"]
    body = [
        text(left, 38, "Human untagged search quality"),
        text(
            left,
            62,
            "1,000 balanced lemma-presence cases · no POS option or atom tag",
            "muted",
        ),
    ]
    for tick in range(0, 101, 20):
        y = top + plot_height * (1 - tick / 100)
        body.append(
            f'<line class="grid" x1="{left}" y1="{y:.1f}" '
            f'x2="{width-right}" y2="{y:.1f}"/>'
        )
        body.append(text(left - 10, y + 5, tick, "muted", "end"))
    group_width = plot_width / len(metrics)
    bar_width = 56
    gap = 14
    for group_index, (label, key) in enumerate(metrics):
        center = left + group_width * (group_index + 0.5)
        start = center - (
            len(HUMAN_UNTAGGED_SERIES) * bar_width
            + (len(HUMAN_UNTAGGED_SERIES) - 1) * gap
        ) / 2
        for series_index, (profile, boundary) in enumerate(HUMAN_UNTAGGED_SERIES):
            value = float(
                human["profiles"][profile]["boundaries"][boundary]["quality"][key]
            )
            x = start + series_index * (bar_width + gap)
            bar_height = plot_height * value / 100
            y = top + plot_height - bar_height
            body.append(
                rect(x, y, bar_width, bar_height, BOUNDARY_COLORS[(profile, boundary)])
            )
            body.append(text(x + bar_width / 2, y - 8, f"{value:.2f}", anchor="middle"))
        body.append(text(center, top + plot_height + 28, label, anchor="middle"))
    for index, (profile, boundary) in enumerate(HUMAN_UNTAGGED_SERIES):
        legend_x = left + (index % 2) * 430
        legend_y = height - 48 + (index // 2) * 28
        body.append(
            rect(
                legend_x,
                legend_y - 13,
                16,
                16,
                BOUNDARY_COLORS[(profile, boundary)],
                2,
            )
        )
        body.append(text(legend_x + 24, legend_y, f"{profile} · {boundary}"))
    return svg_document(
        width,
        height,
        "Human untagged search quality",
        "Grouped bars compare precision, recall, and F1 for untagged queries across two lexicon profiles and two boundary policies.",
        body,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--prefix", default="")
    args = parser.parse_args()
    report = json.loads(args.report.read_text(encoding="utf-8"))
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / f"{args.prefix}morphology-quality.svg").write_text(
        render_quality(report), encoding="utf-8"
    )
    (args.output / f"{args.prefix}morphology-performance.svg").write_text(
        render_performance(report), encoding="utf-8"
    )
    if "product_workflows" in report:
        (args.output / f"{args.prefix}product-workflows.svg").write_text(
            render_product_workflows(report), encoding="utf-8"
        )
    if "product_use_cases" in report:
        (args.output / f"{args.prefix}product-use-cases.svg").write_text(
            render_product_use_cases(report), encoding="utf-8"
        )
    if "boundary_comparison" in report:
        (args.output / f"{args.prefix}boundary-quality.svg").write_text(
            render_boundary_quality(report), encoding="utf-8"
        )
        (args.output / f"{args.prefix}boundary-performance.svg").write_text(
            render_boundary_performance(report), encoding="utf-8"
        )
    if "human_untagged" in report:
        (args.output / f"{args.prefix}human-untagged-quality.svg").write_text(
            render_human_untagged_quality(report), encoding="utf-8"
        )


if __name__ == "__main__":
    main()
