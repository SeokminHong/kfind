#!/usr/bin/env python3

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


BACKENDS = ("kfind", "kiwi", "lindera")
COLORS = {
    "kfind": "#2563eb",
    "kiwi": "#059669",
    "lindera": "#d97706",
}
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
        start = center - (len(BACKENDS) * bar_width + 2 * gap) / 2
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
        legend_x += 118
    return svg_document(
        width,
        height,
        "Held-out morphology quality",
        "Grouped bars compare accuracy, precision, recall, and F1 for kfind, Kiwi, and Lindera.",
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
        label_width = 68
        bar_max_width = panel_width - label_width - 140
        for index, backend in enumerate(BACKENDS):
            row_y = chart_top + index * 60
            value = values[backend]
            body.append(text(panel_x, row_y + 24, backend))
            bar_x = panel_x + label_width
            bar_width = bar_max_width * value / maximum if maximum else 0
            body.append(rect(bar_x, row_y + 7, bar_width, 25, COLORS[backend]))
            rendered = f"{value:.1f}" if value >= 10 else f"{value:.4f}"
            body.append(text(bar_x + bar_max_width + 8, row_y + 25, f"{rendered} {unit}"))
        axis_y = chart_top + 3 * 60
        body.append(f'<line class="axis" x1="{panel_x + label_width}" y1="{axis_y}" '
                    f'x2="{panel_x + label_width + bar_max_width}" y2="{axis_y}"/>')
    return svg_document(
        width,
        height,
        "End-to-end morphology performance",
        "Four horizontal bar panels compare throughput, initialization time, p95 latency, and peak RSS.",
        body,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text(encoding="utf-8"))
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "morphology-quality.svg").write_text(
        render_quality(report), encoding="utf-8"
    )
    (args.output / "morphology-performance.svg").write_text(
        render_performance(report), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
