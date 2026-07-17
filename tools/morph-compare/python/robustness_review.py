from __future__ import annotations

import hashlib
import json
from collections import Counter
from dataclasses import dataclass, replace
from pathlib import Path

from dataset import GoldCandidate, Sentence, review_pool_sha256


NOISE_CLASSES = {
    "foreign-text-typo",
    "hangul-typo",
    "nonstandard-morphology",
    "nonstandard-syntax",
    "repetition",
    "spacing-merge",
    "spacing-split",
}
QUERY_POS = {
    "adjective",
    "adverb",
    "determiner",
    "noun",
    "numeral",
    "pronoun",
    "verb",
}
REVIEW_VERDICTS = {"clean", "source-artifact"}


@dataclass(frozen=True)
class SourceSignal:
    sent_id: str
    text: str
    typo_token_ids: tuple[str, ...]
    typo_forms: tuple[str, ...]
    goeswith_token_ids: tuple[str, ...]
    goeswith_forms: tuple[str, ...]

    @property
    def marked_token_ids(self) -> frozenset[str]:
        return frozenset((*self.typo_token_ids, *self.goeswith_token_ids))


@dataclass(frozen=True)
class SentenceNoiseReview:
    sent_id: str
    primary_noise_class: str
    noise_classes: tuple[str, ...]
    annotation: str
    marked_token_ids: frozenset[str]


def parse_source_signals(path: Path) -> dict[str, SourceSignal]:
    signals: dict[str, SourceSignal] = {}
    for block in path.read_text(encoding="utf-8").strip().split("\n\n"):
        lines = block.splitlines()
        sent_id = next(
            line.removeprefix("# sent_id = ")
            for line in lines
            if line.startswith("# sent_id = ")
        )
        text = next(
            line.removeprefix("# text = ")
            for line in lines
            if line.startswith("# text = ")
        )
        rows = [line.split("\t") for line in lines if line and not line.startswith("#")]
        typo_rows = [
            row
            for row in rows
            if "Typo=Yes" in row[5].split("|")
            or "Typo=Yes" in row[9].split("|")
        ]
        goeswith_rows = [row for row in rows if row[7] == "goeswith"]
        if not typo_rows and not goeswith_rows:
            continue
        if sent_id in signals:
            raise ValueError(f"robustness source signal is duplicated: {sent_id}")
        signals[sent_id] = SourceSignal(
            sent_id=sent_id,
            text=text,
            typo_token_ids=tuple(row[0] for row in typo_rows),
            typo_forms=tuple(row[1] for row in typo_rows),
            goeswith_token_ids=tuple(row[0] for row in goeswith_rows),
            goeswith_forms=tuple(row[1] for row in goeswith_rows),
        )
    return signals


def source_signal_rows(
    source_name: str, signals: dict[str, SourceSignal]
) -> list[dict[str, str]]:
    return [
        {"source": source_name, "sent_id": signal.sent_id, "text": signal.text}
        for signal in signals.values()
    ]


def _validated_noise_review(entry: object, *, field: str) -> SentenceNoiseReview:
    if not isinstance(entry, dict):
        raise ValueError(f"robustness {field} entry must be an object")
    sent_id = entry.get("sent_id")
    primary = entry.get("primary_noise_class")
    noise_classes = entry.get("noise_classes")
    annotation = entry.get("annotation")
    if not isinstance(sent_id, str) or not sent_id:
        raise ValueError(f"robustness {field} entry requires sent_id")
    if primary not in NOISE_CLASSES:
        raise ValueError(f"invalid robustness noise class: {primary}")
    if (
        not isinstance(noise_classes, list)
        or not noise_classes
        or primary not in noise_classes
        or any(noise_class not in NOISE_CLASSES for noise_class in noise_classes)
    ):
        raise ValueError(f"invalid robustness noise_classes: {sent_id}")
    if not isinstance(annotation, str) or not annotation:
        raise ValueError(f"robustness {field} entry requires annotation: {sent_id}")
    return SentenceNoiseReview(
        sent_id=sent_id,
        primary_noise_class=str(primary),
        noise_classes=tuple(str(noise_class) for noise_class in noise_classes),
        annotation=annotation,
        marked_token_ids=frozenset(),
    )


def _default_signal_review(signal: SourceSignal) -> SentenceNoiseReview:
    noise_classes = []
    if signal.goeswith_forms:
        noise_classes.append("spacing-split")
    if signal.typo_forms:
        typo_class = (
            "foreign-text-typo"
            if any(
                any(character.isascii() and character.isalpha() for character in form)
                for form in signal.typo_forms
            )
            else "hangul-typo"
        )
        noise_classes.append(typo_class)
    primary = noise_classes[0]
    marked = ", ".join((*signal.typo_forms, *signal.goeswith_forms))
    return SentenceNoiseReview(
        sent_id=signal.sent_id,
        primary_noise_class=primary,
        noise_classes=tuple(dict.fromkeys(noise_classes)),
        annotation=f"수동 검토로 오류 표기를 확인했다: {marked}",
        marked_token_ids=signal.marked_token_ids,
    )


def load_reviewed_noisy_sentences(
    *,
    source_name: str,
    sentences: list[Sentence],
    source_path: Path,
    review_path: Path,
) -> tuple[list[Sentence], dict[str, SentenceNoiseReview], dict[str, object]]:
    document = json.loads(review_path.read_text(encoding="utf-8"))
    if document.get("schema_version") != 1:
        raise ValueError("unsupported robustness review schema")
    if document.get("source") != source_name or document.get("split") != "test":
        raise ValueError("robustness review source or split mismatch")
    policy = document.get("review_policy")
    if not isinstance(policy, str) or not policy:
        raise ValueError("robustness review requires review_policy")

    signals = parse_source_signals(source_path)
    signal_pool = document.get("source_signal_pool")
    if not isinstance(signal_pool, dict):
        raise ValueError("robustness review requires source_signal_pool")
    actual_signal_sha256 = review_pool_sha256(source_signal_rows(source_name, signals))
    if signal_pool.get("pool_sentences") != len(signals):
        raise ValueError("robustness source signal count mismatch")
    if signal_pool.get("pool_sha256") != actual_signal_sha256:
        raise ValueError("robustness source signal hash mismatch")

    excluded = document.get("excluded")
    if not isinstance(excluded, list):
        raise ValueError("robustness review requires excluded")
    excluded_ids = set()
    excluded_counts: Counter[str] = Counter()
    for entry in excluded:
        if not isinstance(entry, dict):
            raise ValueError("robustness exclusion must be an object")
        sent_id = entry.get("sent_id")
        verdict = entry.get("verdict")
        annotation = entry.get("annotation")
        if not isinstance(sent_id, str) or sent_id not in signals:
            raise ValueError(f"robustness exclusion is outside source signals: {sent_id}")
        if sent_id in excluded_ids:
            raise ValueError(f"robustness exclusion is duplicated: {sent_id}")
        if verdict not in REVIEW_VERDICTS:
            raise ValueError(f"invalid robustness exclusion verdict: {verdict}")
        if not isinstance(annotation, str) or not annotation:
            raise ValueError(f"robustness exclusion requires annotation: {sent_id}")
        excluded_ids.add(sent_id)
        excluded_counts[str(verdict)] += 1

    reviews = {
        sent_id: _default_signal_review(signal)
        for sent_id, signal in signals.items()
        if sent_id not in excluded_ids
    }
    supplements = document.get("supplements")
    if not isinstance(supplements, list):
        raise ValueError("robustness review requires supplements")
    for entry in supplements:
        review = _validated_noise_review(entry, field="supplement")
        if review.sent_id in signals or review.sent_id in reviews:
            raise ValueError(f"robustness supplement is not unique: {review.sent_id}")
        reviews[review.sent_id] = review

    overrides = document.get("class_overrides")
    if not isinstance(overrides, list):
        raise ValueError("robustness review requires class_overrides")
    overridden_ids = set()
    for entry in overrides:
        override = _validated_noise_review(entry, field="class override")
        if override.sent_id not in reviews or override.sent_id in overridden_ids:
            raise ValueError(f"invalid robustness class override: {override.sent_id}")
        reviews[override.sent_id] = override
        overridden_ids.add(override.sent_id)

    indexed_sentences = {sentence.sent_id: sentence for sentence in sentences}
    unknown = sorted(set(reviews) - set(indexed_sentences))
    if unknown:
        raise ValueError(f"robustness review sentences are missing from source: {unknown}")
    reviewed_sentences = [indexed_sentences[sent_id] for sent_id in reviews]
    class_counts = Counter(
        review.primary_noise_class for review in reviews.values()
    )
    metadata = {
        "review_file": review_path.name,
        "review_policy": policy,
        "source_signal_sentences": len(signals),
        "source_signal_sha256": actual_signal_sha256,
        "supplement_sentences": len(supplements),
        "review_pool_sentences": len(signals) + len(supplements),
        "review_pool_sha256": review_pool_sha256(
            [
                *source_signal_rows(source_name, signals),
                *[
                    {
                        "source": source_name,
                        "sent_id": str(entry["sent_id"]),
                        "text": indexed_sentences[str(entry["sent_id"])].text,
                    }
                    for entry in supplements
                ],
            ]
        ),
        "accepted_noisy_sentences": len(reviews),
        "excluded_sentences": len(excluded_ids),
        "excluded_verdict_counts": dict(sorted(excluded_counts.items())),
        "primary_noise_class_counts": dict(sorted(class_counts.items())),
    }
    return reviewed_sentences, reviews, metadata


def _candidate_id(candidate: GoldCandidate) -> str:
    return ":".join(
        (
            "pos",
            candidate.source,
            candidate.sent_id,
            candidate.token_id,
            str(candidate.morph_index),
        )
    )


def apply_candidate_review(
    sentences: list[Sentence], review_path: Path
) -> tuple[list[Sentence], dict[str, object]]:
    document = json.loads(review_path.read_text(encoding="utf-8"))
    corrections = document.get("candidate_corrections")
    rejections = document.get("candidate_rejections")
    if not isinstance(corrections, list) or not isinstance(rejections, list):
        raise ValueError("robustness review requires candidate review lists")

    indexed = {
        _candidate_id(candidate): candidate
        for sentence in sentences
        for candidate in sentence.candidates
    }
    corrected: dict[str, tuple[str, str]] = {}
    rejected = set()
    for entry in corrections:
        if not isinstance(entry, dict):
            raise ValueError("robustness candidate correction must be an object")
        candidate_id = entry.get("candidate_id")
        query = entry.get("query")
        pos = entry.get("pos")
        annotation = entry.get("annotation")
        if not isinstance(candidate_id, str) or candidate_id not in indexed:
            raise ValueError(f"unknown robustness candidate correction: {candidate_id}")
        if not isinstance(query, str) or not query or pos not in QUERY_POS:
            raise ValueError(f"invalid robustness candidate correction: {candidate_id}")
        if not isinstance(annotation, str) or not annotation:
            raise ValueError(f"robustness candidate correction needs annotation: {candidate_id}")
        if candidate_id in corrected:
            raise ValueError(f"duplicated robustness candidate correction: {candidate_id}")
        corrected[candidate_id] = (query, str(pos))

    for entry in rejections:
        if not isinstance(entry, dict):
            raise ValueError("robustness candidate rejection must be an object")
        candidate_id = entry.get("candidate_id")
        annotation = entry.get("annotation")
        if not isinstance(candidate_id, str) or candidate_id not in indexed:
            raise ValueError(f"unknown robustness candidate rejection: {candidate_id}")
        if not isinstance(annotation, str) or not annotation:
            raise ValueError(f"robustness candidate rejection needs annotation: {candidate_id}")
        if candidate_id in rejected or candidate_id in corrected:
            raise ValueError(f"duplicated robustness candidate review: {candidate_id}")
        rejected.add(candidate_id)

    reviewed_sentences = []
    for sentence in sentences:
        candidates = []
        for candidate in sentence.candidates:
            candidate_id = _candidate_id(candidate)
            if candidate_id in rejected:
                continue
            if candidate_id in corrected:
                query, pos = corrected[candidate_id]
                candidate = replace(candidate, query=query, pos=pos)
            candidates.append(candidate)
        reviewed_sentences.append(replace(sentence, candidates=tuple(candidates)))
    return reviewed_sentences, {
        "corrected_candidates": len(corrected),
        "rejected_candidates": len(rejected),
    }


def load_negative_rejections(review_path: Path, query_mode: str) -> set[str]:
    document = json.loads(review_path.read_text(encoding="utf-8"))
    reviews = document.get("negative_rejections")
    entries = reviews.get(query_mode) if isinstance(reviews, dict) else None
    if not isinstance(entries, list):
        raise ValueError(f"robustness negative review is missing: {query_mode}")
    rejected = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("robustness negative rejection must be an object")
        case_id = entry.get("case_id")
        annotation = entry.get("annotation")
        if not isinstance(case_id, str) or not case_id:
            raise ValueError("robustness negative rejection requires case_id")
        if not isinstance(annotation, str) or not annotation:
            raise ValueError(
                f"robustness negative rejection needs annotation: {case_id}"
            )
        if case_id in rejected:
            raise ValueError(f"duplicated robustness negative rejection: {case_id}")
        rejected.add(case_id)
    return rejected


def case_review_sha256(cases: list[dict[str, object]]) -> str:
    fields = (
        "id",
        "sent_id",
        "text",
        "query",
        "pos",
        "expected",
        "gold_byte_start",
        "gold_byte_end",
        "noise_class",
        "noise_scope",
    )
    serialized = sorted(
        json.dumps(
            {field: case.get(field) for field in fields},
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        for case in cases
    )
    payload = "".join(f"{line}\n" for line in serialized).encode()
    return hashlib.sha256(payload).hexdigest()


def validate_case_review(
    review_path: Path,
    *,
    query_mode: str,
    cases: list[dict[str, object]],
    allow_draft: bool,
) -> dict[str, object]:
    document = json.loads(review_path.read_text(encoding="utf-8"))
    case_reviews = document.get("case_reviews")
    review = case_reviews.get(query_mode) if isinstance(case_reviews, dict) else None
    actual_sha256 = case_review_sha256(cases)
    if review is None and allow_draft:
        return {"cases": len(cases), "sha256": actual_sha256, "status": "draft"}
    if not isinstance(review, dict):
        raise ValueError(f"robustness case review is missing: {query_mode}")
    if review.get("cases") != len(cases) or review.get("sha256") != actual_sha256:
        raise ValueError(f"robustness case review mismatch: {query_mode}")
    return {"cases": len(cases), "sha256": actual_sha256, "status": "reviewed"}
