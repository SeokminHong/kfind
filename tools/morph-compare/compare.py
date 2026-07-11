#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import importlib.metadata
import json
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from operator import attrgetter
from pathlib import Path
from typing import Iterable

from kiwipiepy import Kiwi


DEFAULT_FIXTURES = Path("/opt/kfind/data/fixtures/morphology_cases.tsv")
PREDICATE_TAGS = {"VV", "VX", "XSV", "VA", "VCP", "VCN", "XSA"}


@dataclass(frozen=True)
class Case:
    query: str
    pos: str
    text: str
    expected: str
    feature: str


@dataclass(frozen=True)
class Candidate:
    lemma: str
    pos: str
    raw_tag: str


def coarse_pos(raw_tag: str) -> str | None:
    tag = raw_tag.split("+", 1)[0].split("-", 1)[0]
    if tag in {"NNG", "NNP", "NNB", "NNBC"}:
        return "noun"
    if tag == "NP":
        return "pronoun"
    if tag in {"NR", "SN"}:
        return "numeral"
    if tag in {"VV", "VX", "XSV"}:
        return "verb"
    if tag in {"VA", "VCP", "VCN", "XSA"}:
        return "adjective"
    if tag == "MM":
        return "determiner"
    if tag in {"MAG", "MAJ"}:
        return "adverb"
    if tag.startswith("J"):
        return "particle"
    if tag == "IC":
        return "interjection"
    return None


def candidate(form: str, raw_tag: str) -> Candidate | None:
    pos = coarse_pos(raw_tag)
    if pos is None:
        return None
    tag = raw_tag.split("+", 1)[0].split("-", 1)[0]
    lemma = f"{form}다" if tag in PREDICATE_TAGS and not form.endswith("다") else form
    return Candidate(lemma=lemma, pos=pos, raw_tag=raw_tag)


def add_productive_candidates(
    morphemes: list[tuple[str, str, int | None]], candidates: set[Candidate]
) -> None:
    for (left_form, left_tag, left_word), (right_form, right_tag, right_word) in zip(
        morphemes, morphemes[1:]
    ):
        if left_tag.split("+", 1)[0] not in {"NNG", "NNP"}:
            continue
        right_base = right_tag.split("+", 1)[0].split("-", 1)[0]
        if right_base not in {"XSV", "XSA"} or right_form != "하":
            continue
        if left_word is not None and right_word is not None and left_word != right_word:
            continue
        pos = "verb" if right_base == "XSV" else "adjective"
        candidates.add(Candidate(f"{left_form}하다", pos, f"{left_tag}+{right_tag}"))


def kiwi_candidates(tokens: Iterable[object]) -> set[Candidate]:
    candidates: set[Candidate] = set()
    morphemes: list[tuple[str, str, int | None]] = []
    for token in tokens:
        form = str(token.form)
        tag = str(token.tag)
        word_position = int(token.word_position)
        morphemes.append((form, tag, word_position))
        normalized = candidate(form, tag)
        if normalized is not None:
            candidates.add(normalized)
    add_productive_candidates(morphemes, candidates)
    return candidates


def lindera_morphemes(
    tokens: Iterable[dict[str, object]],
) -> list[tuple[str, str, int | None]]:
    morphemes: list[tuple[str, str, int | None]] = []
    for word_position, token in enumerate(tokens):
        expression = str(token.get("expression", "*"))
        if expression not in {"", "*"}:
            parsed = []
            for part in expression.split("+"):
                fields = part.split("/")
                if len(fields) >= 2 and fields[0] and fields[1] != "*":
                    parsed.append((fields[0], fields[1], word_position))
            if parsed:
                morphemes.extend(parsed)
                continue
        surface = str(token.get("surface", ""))
        tag = str(token.get("part_of_speech_tag", ""))
        if surface and tag:
            morphemes.append((surface, tag, word_position))
    return morphemes


def lindera_candidates(tokens: Iterable[dict[str, object]]) -> set[Candidate]:
    morphemes = lindera_morphemes(tokens)
    candidates = {
        normalized
        for form, tag, _ in morphemes
        if (normalized := candidate(form, tag)) is not None
    }
    add_productive_candidates(morphemes, candidates)
    return candidates


def candidate_matches(query: str, pos: str, candidates: set[Candidate]) -> bool:
    return any(item.lemma == query and item.pos == pos for item in candidates)


def load_cases(path: Path, feature_prefix: str, limit: int | None) -> list[Case]:
    with path.open(encoding="utf-8", newline="") as fixture_file:
        rows = csv.DictReader(fixture_file, delimiter="\t")
        cases = [
            Case(**row)
            for row in rows
            if row["feature"].startswith(feature_prefix)
            and row["pos"] != "literal"
            and " " not in row["query"]
        ]
    if limit is not None:
        cases = cases[:limit]
    if not cases:
        raise ValueError(f"no fixture cases matched feature prefix {feature_prefix!r}")
    return cases


def checked_output(command: list[str]) -> str:
    return subprocess.run(command, check=True, text=True, capture_output=True).stdout.strip()


def run_kfind(case: Case) -> tuple[bool, float]:
    pos = "adjective" if case.pos == "copula" else case.pos
    started = time.perf_counter()
    result = subprocess.run(
        ["kfind", "--quiet", "--pos", pos, case.query, "-"],
        input=f"{case.text}\n",
        text=True,
        capture_output=True,
    )
    elapsed = time.perf_counter() - started
    if result.returncode not in {0, 1}:
        raise RuntimeError(
            f"kfind failed for {case.feature}: exit={result.returncode}, stderr={result.stderr.strip()}"
        )
    return result.returncode == 0, elapsed


def run_lindera(text: str) -> tuple[list[dict[str, object]], float]:
    started = time.perf_counter()
    result = subprocess.run(
        [
            "lindera",
            "tokenize",
            "--dict",
            "embedded://ko-dic",
            "--output",
            "json",
        ],
        input=f"{text}\n",
        text=True,
        capture_output=True,
    )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        raise RuntimeError(f"lindera failed: {result.stderr.strip()}")
    return json.loads(result.stdout), elapsed


def raw_kiwi_tokens(tokens: Iterable[object]) -> list[dict[str, object]]:
    return [
        {
            "form": str(token.form),
            "tag": str(token.tag),
            "start": int(token.start),
            "end": int(token.end),
            "word_position": int(token.word_position),
        }
        for token in tokens
    ]


def sorted_candidates(candidates: set[Candidate]) -> list[dict[str, str]]:
    return [
        asdict(item)
        for item in sorted(candidates, key=attrgetter("lemma", "pos", "raw_tag"))
    ]


def percent(numerator: int, denominator: int) -> float:
    return round(100 * numerator / denominator, 1) if denominator else 0.0


def render_markdown(report: dict[str, object]) -> str:
    metrics = report["metrics"]
    versions = report["versions"]
    lines = [
        "# kfind / Kiwi / Lindera morphology comparison",
        "",
        f"- cases: {metrics['cases']}",
        f"- positive cases: {metrics['positive_cases']}",
        f"- kfind expectation accuracy: {metrics['kfind_accuracy_percent']}%",
        f"- Kiwi positive recall: {metrics['kiwi_positive_recall_percent']}%",
        f"- Lindera positive recall: {metrics['lindera_positive_recall_percent']}%",
        f"- versions: kfind `{versions['kfind']}`, Kiwi `{versions['kiwi']}`, Lindera `{versions['lindera']}`",
        "",
        "| case | query | expected | kfind | Kiwi | Lindera |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for result in report["results"]:
        lines.append(
            "| {feature} | {query} | {expected} | {kfind} | {kiwi} | {lindera} |".format(
                feature=result["feature"],
                query=result["query"],
                expected=result["expected"],
                kfind="match" if result["kfind_match"] else "no-match",
                kiwi="found" if result["kiwi_found"] else "not-found",
                lindera="found" if result["lindera_found"] else "not-found",
            )
        )
    return "\n".join(lines) + "\n"


def compare(cases: list[Case]) -> dict[str, object]:
    kiwi_started = time.perf_counter()
    kiwi = Kiwi()
    kiwi_initialize_seconds = time.perf_counter() - kiwi_started

    results = []
    kfind_correct = 0
    positive_cases = 0
    kiwi_positive_hits = 0
    lindera_positive_hits = 0
    tool_seconds = {"kfind": 0.0, "kiwi": kiwi_initialize_seconds, "lindera": 0.0}

    for case in cases:
        kfind_match, kfind_seconds = run_kfind(case)
        tool_seconds["kfind"] += kfind_seconds

        kiwi_started = time.perf_counter()
        kiwi_tokens = kiwi.tokenize(case.text)
        tool_seconds["kiwi"] += time.perf_counter() - kiwi_started
        kiwi_normalized = kiwi_candidates(kiwi_tokens)

        lindera_tokens, lindera_seconds = run_lindera(case.text)
        tool_seconds["lindera"] += lindera_seconds
        lindera_normalized = lindera_candidates(lindera_tokens)

        expected_match = case.expected == "match"
        kiwi_found = candidate_matches(case.query, case.pos, kiwi_normalized)
        lindera_found = candidate_matches(case.query, case.pos, lindera_normalized)
        kfind_correct += kfind_match == expected_match
        if expected_match:
            positive_cases += 1
            kiwi_positive_hits += kiwi_found
            lindera_positive_hits += lindera_found

        results.append(
            {
                **asdict(case),
                "kfind_match": kfind_match,
                "kiwi_found": kiwi_found,
                "lindera_found": lindera_found,
                "kiwi_candidates": sorted_candidates(kiwi_normalized),
                "lindera_candidates": sorted_candidates(lindera_normalized),
                "kiwi_tokens": raw_kiwi_tokens(kiwi_tokens),
                "lindera_tokens": lindera_tokens,
            }
        )

    return {
        "versions": {
            "kfind": checked_output(["kfind", "--version"]),
            "kiwi": importlib.metadata.version("kiwipiepy"),
            "lindera": checked_output(["lindera", "--version"]),
        },
        "metrics": {
            "cases": len(cases),
            "positive_cases": positive_cases,
            "kfind_correct": kfind_correct,
            "kfind_accuracy_percent": percent(kfind_correct, len(cases)),
            "kiwi_positive_hits": kiwi_positive_hits,
            "kiwi_positive_recall_percent": percent(kiwi_positive_hits, positive_cases),
            "lindera_positive_hits": lindera_positive_hits,
            "lindera_positive_recall_percent": percent(lindera_positive_hits, positive_cases),
            "elapsed_seconds": {name: round(value, 4) for name, value in tool_seconds.items()},
        },
        "results": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixtures", type=Path, default=DEFAULT_FIXTURES)
    parser.add_argument("--feature-prefix", default="corpus.")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        cases = load_cases(args.fixtures, args.feature_prefix, args.limit)
        report = compare(cases)
        markdown = render_markdown(report)
        print(markdown, end="")
        if args.output is not None:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(
                json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
            args.output.with_suffix(".md").write_text(markdown, encoding="utf-8")
        if report["metrics"]["kfind_correct"] != report["metrics"]["cases"]:
            return 1
        return 0
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"comparison failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
