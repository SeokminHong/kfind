from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


ISO_DATE = re.compile(r"(?<!\d)20\d{2}-\d{2}-\d{2}(?!\d)")
PR_REFERENCE = re.compile(
    r"(?:\b(?:PR|pull request)\s*#\d+\b|github\.com/[^\s)]+/pull/\d+)",
    re.IGNORECASE,
)
REVISION_SNAPSHOT = re.compile(
    r"\b(?:revision|commit)\s+`?[0-9a-f]{7,40}`?\b",
    re.IGNORECASE,
)
WORK_LOG_PHRASE = re.compile(
    r"(?:"
    r"\blatest\s+(?:benchmark|comparison|measurement|result|figures?)\b|"
    r"최신\s*(?:benchmark|비교|측정|결과|수치)|"
    r"\bimprovement handoff\b|개선 핸드오프|후속 작업|폐기된|"
    r"\b(?:decreased|increased|regressed|recovered|reduced)\b|"
    r"(?:줄었|늘었|낮아졌|높아졌|감소했|증가했)"
    r")",
    re.IGNORECASE,
)

ROOT_READMES = {Path("README.md")}
CURRENT_INFORMATION_READMES = ROOT_READMES | {Path("docs/benchmarks/README.md")}


@dataclass(frozen=True)
class Violation:
    path: Path
    line: int
    reason: str
    text: str


def tracked_readmes(repository: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "*README*.md"],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    )
    return [Path(line) for line in result.stdout.splitlines() if line]


def benchmark_section(text: str, heading: str) -> str:
    marker = f"## {heading}\n"
    start = text.find(marker)
    if start < 0:
        return ""
    body_start = start + len(marker)
    next_heading = text.find("\n## ", body_start)
    if next_heading < 0:
        return text[body_start:]
    return text[body_start:next_heading]


def check_readme(path: Path, text: str) -> list[Violation]:
    violations: list[Violation] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        if ISO_DATE.search(line):
            violations.append(Violation(path, line_number, "dated history", line))
        if PR_REFERENCE.search(line):
            violations.append(Violation(path, line_number, "PR history", line))
        if path in CURRENT_INFORMATION_READMES and REVISION_SNAPSHOT.search(line):
            violations.append(Violation(path, line_number, "revision snapshot", line))
        if path in CURRENT_INFORMATION_READMES and WORK_LOG_PHRASE.search(line):
            violations.append(Violation(path, line_number, "work-log wording", line))

    if path in ROOT_READMES:
        heading = "벤치마크"
        section = benchmark_section(text, heading)
        for marker, reason in (
            ("\n### ", "benchmark result subsection"),
            ("\n|", "benchmark result table"),
            ("![", "benchmark result image"),
        ):
            if marker in section:
                line_number = text[: text.find(section) + section.find(marker)].count("\n") + 1
                violations.append(Violation(path, line_number, reason, marker.strip()))
    return violations


def check_repository(repository: Path) -> list[Violation]:
    violations: list[Violation] = []
    for path in tracked_readmes(repository):
        readme = repository / path
        if not readme.is_file():
            continue
        text = readme.read_text(encoding="utf-8")
        violations.extend(check_readme(path, text))
    return violations
