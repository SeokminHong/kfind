from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


PREDICATE_TAGS = {"VV", "VX", "VA", "VCP", "VCN"}


@dataclass(frozen=True)
class CandidateSpan:
    lemma: str
    pos: str
    raw_tag: str
    byte_start: int
    byte_end: int


def coarse_pos(raw_tag: str) -> str | None:
    tag = raw_tag.split("+", 1)[0].split("-", 1)[0]
    if tag in {"NNG", "NNP", "NNB", "NNBC"}:
        return "noun"
    if tag == "NP":
        return "pronoun"
    if tag in {"NR", "SN"}:
        return "numeral"
    if tag in {"VV", "VX"}:
        return "verb"
    if tag in {"VA", "VCP", "VCN"}:
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


def candidate(
    form: str, raw_tag: str, byte_start: int, byte_end: int
) -> CandidateSpan | None:
    pos = coarse_pos(raw_tag)
    if pos is None:
        return None
    tag = raw_tag.split("+", 1)[0].split("-", 1)[0]
    lemma = f"{form}다" if tag in PREDICATE_TAGS and not form.endswith("다") else form
    return CandidateSpan(lemma, pos, raw_tag, byte_start, byte_end)


def character_to_byte_offsets(text: str) -> list[int]:
    offsets = [0]
    for character in text:
        offsets.append(offsets[-1] + len(character.encode("utf-8")))
    return offsets


def kiwi_candidates(text: str, tokens: Iterable[object]) -> set[CandidateSpan]:
    offsets = character_to_byte_offsets(text)
    candidates: set[CandidateSpan] = set()
    for token in tokens:
        form = str(token.form)
        tag = str(token.tag)
        start = offsets[int(token.start)]
        end = offsets[int(token.end)]
        normalized = candidate(form, tag, start, end)
        if normalized is not None:
            candidates.add(normalized)
    return candidates


def lindera_morphemes(
    tokens: Iterable[dict[str, object]],
) -> list[tuple[str, str, int, int]]:
    morphemes = []
    for token in tokens:
        start = int(token["byte_start"])
        end = int(token["byte_end"])
        details = [str(value) for value in token.get("details", [])]
        surface = str(token.get("surface", ""))
        morphemes.extend(token_morphemes(surface, details, start, end))
    return morphemes


def lindera_candidates(tokens: Iterable[dict[str, object]]) -> set[CandidateSpan]:
    morphemes = lindera_morphemes(tokens)
    candidates = {
        normalized
        for form, tag, start, end in morphemes
        if (normalized := candidate(form, tag, start, end)) is not None
    }
    return candidates


def token_morphemes(
    surface: str, details: list[str], start: int, end: int
) -> list[tuple[str, str, int, int]]:
    expression = details[7] if len(details) > 7 else "*"
    if expression not in {"", "*"}:
        parsed = []
        for part in expression.split("+"):
            fields = part.split("/")
            if len(fields) >= 2 and fields[0] and fields[1] != "*":
                parsed.append((fields[0], fields[1], start, end))
        if parsed:
            return parsed
    tag = details[0] if details else ""
    return [(surface, tag, start, end)] if surface and tag else []


def mecab_candidates(text: str, parsed: str) -> set[CandidateSpan]:
    byte_offsets = character_to_byte_offsets(text)
    character_start = 0
    morphemes = []
    for line in parsed.splitlines():
        if line == "EOS":
            break
        if "\t" not in line:
            raise ValueError(f"invalid MeCab-ko output line {line!r}")
        surface, feature_text = line.split("\t", 1)
        token_start = text.find(surface, character_start)
        if token_start < 0:
            raise ValueError(f"MeCab-ko surface {surface!r} is absent from input")
        token_end = token_start + len(surface)
        morphemes.extend(
            token_morphemes(
                surface,
                feature_text.split(","),
                byte_offsets[token_start],
                byte_offsets[token_end],
            )
        )
        character_start = token_end
    return {
        normalized
        for form, tag, start, end in morphemes
        if (normalized := candidate(form, tag, start, end)) is not None
    }


def komoran_candidates(
    text: str, tokens: Iterable[dict[str, object]]
) -> set[CandidateSpan]:
    byte_offsets = character_to_byte_offsets(text)
    candidates = set()
    for token in tokens:
        begin = int(token["begin"])
        end = int(token["end"])
        if begin < 0 or begin >= end or end >= len(byte_offsets):
            raise ValueError(f"invalid KOMORAN token span {begin}..{end}")
        normalized = candidate(
            str(token["morph"]),
            str(token["pos"]),
            byte_offsets[begin],
            byte_offsets[end],
        )
        if normalized is not None:
            candidates.add(normalized)
    return candidates


def spans_overlap(start: int, end: int, gold_start: int, gold_end: int) -> bool:
    return start < gold_end and gold_start < end


def candidate_prediction(
    query: str,
    pos: str,
    expected: bool,
    gold_start: int | None,
    gold_end: int | None,
    candidates: set[CandidateSpan],
) -> bool:
    matching = [item for item in candidates if item.lemma == query and item.pos == pos]
    if not expected:
        return bool(matching)
    if gold_start is None or gold_end is None:
        raise ValueError("positive case is missing its gold span")
    return any(
        spans_overlap(item.byte_start, item.byte_end, gold_start, gold_end)
        for item in matching
    )
