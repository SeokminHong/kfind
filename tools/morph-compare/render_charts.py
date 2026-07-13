#!/usr/bin/env python3

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


BACKENDS = ("kfind-embedded", "kfind-full-pos", "kiwi", "lindera")
COLORS = {
    "kfind-embedded": "#2563eb",
    "kfind-full-pos": "#7c3aed",
    "kiwi": "#059669",
    "lindera": "#d97706",
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
    width, height = 1120, 600
    left, right, top, bottom = 80, 32, 86, 76
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
    bar_width = 48
    gap = 12
    for group_index, (label, key) in enumerate(metrics):
        center = left + group_width * (group_index + 0.5)
        start = center - (len(BACKENDS) * bar_width + (len(BACKENDS) - 1) * gap) / 2
        for backend_index, backend in enumerate(BACKENDS):
            value = float(report["quality"][backend]["overall"][key])
            x = start + backend_index * (bar_width + gap)
            bar_height = plot_height * value / 100
            y = top + plot_height - bar_height
            body.append(rect(x, y, bar_width, bar_height, COLORS[backend]))
            body.append(text(x + bar_width / 2, y - 8, f"{value:.2f}", anchor="middle"))
        body.append(text(center, top + plot_height + 28, label, anchor="middle"))
    legend_x = left
    legend_y = height - 22
    for backend in BACKENDS:
        body.append(rect(legend_x, legend_y - 13, 16, 16, COLORS[backend], 2))
        body.append(text(legend_x + 24, legend_y, backend))
        legend_x += 190
    return svg_document(
        width,
        height,
        "Held-out morphology quality",
        "Grouped bars compare accuracy, precision, recall, and F1 for both kfind profiles, Kiwi, and Lindera.",
        body,
    )


def render_performance(report: dict[str, object]) -> str:
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
        for backend in BACKENDS:
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
        for index, backend in enumerate(BACKENDS):
            row_y = chart_top + index * 60
            value = values[backend]
            body.append(text(panel_x, row_y + 24, backend))
            bar_x = panel_x + label_width
            bar_width = bar_max_width * value / maximum if maximum else 0
            body.append(rect(bar_x, row_y + 7, bar_width, 25, COLORS[backend]))
            rendered = metric_value(value, unit)
            body.append(text(bar_x + bar_max_width + 8, row_y + 25, f"{rendered} {unit}"))
        axis_y = chart_top + len(BACKENDS) * 60
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
