from __future__ import annotations

import html


RAW_COLOR = "#64748b"
CONTRACT_COLOR = "#059669"
PROFILE_COLORS = {
    "kfind-embedded": "#2563eb",
    "kfind-full-pos": "#7c3aed",
}
STYLE = """
<style>
  .background { fill: #ffffff; }
  text { fill: #111827; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .muted { fill: #4b5563; }
  .grid { stroke: #d1d5db; stroke-width: 1; }
  @media (prefers-color-scheme: dark) {
    .background { fill: #0d1117; }
    text { fill: #f3f4f6; }
    .muted { fill: #9ca3af; }
    .grid { stroke: #374151; }
  }
</style>
"""


def text(
    x: float,
    y: float,
    value: object,
    css_class: str = "",
    anchor: str = "start",
) -> str:
    class_attribute = f' class="{css_class}"' if css_class else ""
    return (
        f'<text x="{x:.1f}" y="{y:.1f}" text-anchor="{anchor}"'
        f'{class_attribute}>{html.escape(str(value))}</text>'
    )


def rect(
    x: float, y: float, width: float, height: float, fill: str, radius: int = 3
) -> str:
    return (
        f'<rect x="{x:.1f}" y="{y:.1f}" width="{max(width, 0):.1f}" '
        f'height="{height:.1f}" rx="{radius}" fill="{fill}"/>'
    )


def render_query_matrix_quality(report: dict[str, object]) -> str:
    explicit = report["query_matrix"]["explicit_pos"]
    dataset = explicit["dataset"]
    review = dataset["contract_review"]
    width, height = 1280, 720
    body = [
        text(52, 38, "kfind query matrix · raw vs contract-adjusted"),
        text(
            52,
            62,
            f"{dataset['cases']} cases · {review['confirmed_cases']} confirmed · "
            f"{review['reclassified_cases']} reclassified · {review['excluded_cases']} excluded",
            "muted",
        ),
        rect(930, 27, 16, 16, RAW_COLOR, 2),
        text(954, 40, "Raw corpus gold"),
        rect(1090, 27, 16, 16, CONTRACT_COLOR, 2),
        text(1114, 40, "Contract"),
    ]
    profiles = ("kfind-embedded", "kfind-full-pos")
    panel_width = 560
    for profile_index, profile in enumerate(profiles):
        panel_x = 52 + profile_index * 628
        raw = explicit["quality"][profile]["overall"]
        contract = explicit["quality"][profile]["contract_adjusted"]["overall"]
        body.append(
            rect(panel_x, 96, 8, 28, PROFILE_COLORS[profile], 2)
        )
        body.append(text(panel_x + 20, 117, profile))
        body.append(
            text(
                panel_x + panel_width,
                117,
                f"contract denominator {contract['cases']}",
                "muted",
                "end",
            )
        )

        metric_rows = (
            (
                "Precision",
                float(raw["precision_percent"]),
                float(contract["contract_precision_percent"]),
            ),
            (
                "Recall",
                float(raw["recall_percent"]),
                float(contract["contract_recall_percent"]),
            ),
        )
        label_width = 90
        bar_width = 360
        for row_index, (label, raw_value, contract_value) in enumerate(metric_rows):
            row_y = 164 + row_index * 116
            body.append(text(panel_x, row_y + 26, label))
            for value_index, (series, value, color) in enumerate(
                (
                    ("Raw", raw_value, RAW_COLOR),
                    ("Contract", contract_value, CONTRACT_COLOR),
                )
            ):
                bar_y = row_y + value_index * 40
                body.append(text(panel_x + label_width, bar_y + 24, series, "muted"))
                bar_x = panel_x + label_width + 72
                body.append(rect(bar_x, bar_y + 7, bar_width * value / 100, 24, color))
                body.append(
                    text(
                        panel_x + panel_width,
                        bar_y + 25,
                        f"{value:.2f}%",
                        anchor="end",
                    )
                )

        body.append(text(panel_x, 420, "Confusion counts"))
        body.append(
            text(
                panel_x,
                454,
                f"Raw   TP {raw['tp']} · FP {raw['fp']} · TN {raw['tn']} · FN {raw['fn']}",
            )
        )
        body.append(
            text(
                panel_x,
                486,
                f"Contract   TPᶜ {contract['contract_tp']} · FPᶜ {contract['contract_fp']} · "
                f"TNᶜ {contract['contract_tn']} · FNᶜ {contract['contract_fn']}",
            )
        )
        body.append(
            text(
                panel_x,
                530,
                f"Reviewed {contract['reviewed_cases']} · confirmed {contract['confirmed_cases']} · "
                f"reclassified {contract['reclassified_cases']} · excluded {contract['excluded_cases']}",
                "muted",
            )
        )

    body.extend(
        [
            text(
                52,
                624,
                "Raw preserves source corpus gold for comparison.",
                "muted",
            ),
            text(
                52,
                652,
                "FPᶜ, FNᶜ, precisionᶜ and recallᶜ apply the versioned kfind contract registry; they are not aliases of raw values.",
                "muted",
            ),
        ]
    )
    return "\n".join(
        [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
            '<title id="title">kfind query matrix raw and contract-adjusted quality</title>',
            '<desc id="desc">Raw corpus-gold and reviewed kfind contract precision, recall, and confusion counts are compared for embedded and full-POS profiles.</desc>',
            STYLE,
            f'<rect class="background" width="{width}" height="{height}"/>',
            *body,
            "</svg>",
            "",
        ]
    )
