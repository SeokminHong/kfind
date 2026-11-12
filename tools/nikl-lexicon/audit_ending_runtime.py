#!/usr/bin/env python3

"""Extract independently attested predicate/ending pairs for a runtime coverage audit."""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records
from nikl_endings import krdict_endings, normalize_ending, opendict_endings, stdict_endings
from nikl_import import (
    KRDICT_SHA256,
    OPENDICT_SHA256,
    STDICT_SHA256,
    direct_feat,
    krdict_record,
    normalize_text,
    stdict_record,
)

WORD = re.compile(r"[가-힣]+")
EXAMPLE_MARK = re.compile(r"\{([^{}]+)\}")
FINALS = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ"
PREDICATE_POS = {"VV": "verb", "VA": "adjective"}
ABBREVIATION = re.compile(r"‘(-[^’]+)’의 준말\.")


def krdict_item(entry):
    for predicate in krdict_record(entry):
        yield ("predicate", predicate)
    for ending in krdict_endings(entry):
        for sense in entry.findall("./Sense"):
            definition = normalize_text(direct_feat(sense, "definition") or "")
            if definition.startswith("→"):
                yield ("redirect", (ending, definition))
                continue
            examples = [
                example
                for item in sense.findall("./SenseExample")
                if (example := direct_feat(item, "example"))
            ]
            yield ("ending", (ending, examples, None))


def stdict_item(item):
    for predicate in stdict_record(item):
        if predicate.lexical_status == "일반어":
            yield ("predicate", predicate)
    endings = {ending.source_id.partition(":")[2]: ending for ending in stdict_endings(item)}
    for pos in item.findall("./word_info/pos_info"):
        ending = endings.get(pos.findtext("./pos_code", "").strip())
        if ending is None:
            continue
        for sense in pos.findall(".//sense_info"):
            if sense.findtext("./type") != "일반어":
                continue
            definition = normalize_text(sense.findtext("./definition", ""))
            if definition.startswith("→"):
                yield ("redirect", (ending, definition))
                continue
            abbreviated = ABBREVIATION.fullmatch(definition)
            examples = [node.text for node in sense.findall("./example_info/example") if node.text]
            yield ("ending", (ending, examples, abbreviated[1] if abbreviated else None))


def opendict_item(item):
    for ending in opendict_endings(item):
        for sense in item.findall("./senseInfo"):
            if sense.findtext("./pos") != "어미" or sense.findtext("./type") != "일반어":
                continue
            examples = [node.text for node in sense.findall(".//example") if node.text]
            yield ("ending", (ending, examples, None))


def separate_ending(word: str, ending: str) -> str | None:
    first, rest = ending[0], ending[1:]
    if first not in FINALS[1:]:
        return word.removesuffix(ending) if word.endswith(ending) else None
    if not word.endswith(rest):
        return None
    attached = word.removesuffix(rest)
    if not attached:
        return None
    final = (ord(attached[-1]) - 0xAC00) % 28
    if FINALS[final] != first:
        return None
    return attached[:-1] + chr(ord(attached[-1]) - final)


def annotated_host(tokens, word, stem, predicates):
    suffix_start = word.start() + len(stem)
    hosts = []
    for index, token in enumerate(tokens):
        if token.start < word.start() or token.start > suffix_start:
            continue
        tag = token.tag.partition("-")[0]
        if tag == "VCP" and token.start + token.len <= suffix_start:
            visible_copula = token.len == 1 and word.string[token.start] == "이"
            start = token.start if visible_copula else word.start()
            hosts.append(("이다", "adjective", start))
        if token.start >= suffix_start:
            continue
        lemma = token.form + "다"
        if tag in ("XSV", "XSA") and index:
            previous = tokens[index - 1]
            if previous.start >= word.start() and previous.start + previous.len == token.start:
                lemma = previous.form + lemma
        pos = {"VV": "verb", "VA": "adjective", "XSV": "verb", "XSA": "adjective"}.get(tag)
        if pos and pos in predicates.get(lemma, ()):
            hosts.append((lemma, pos, word.start()))
    return hosts[-1] if hosts else None


def example_candidates(ending, examples, predicates, conjugations, annotate):
    for source, source_id, text, prefinal in examples:
        candidates = []
        tokens = None
        marked = [
            (match.start() - 2 * index, match.end() - 2 * (index + 1))
            for index, match in enumerate(EXAMPLE_MARK.finditer(text))
        ]
        text = EXAMPLE_MARK.sub(r"\1", text)
        for word in WORD.finditer(text):
            if marked and not any(start < word.end() and word.start() < end for start, end in marked):
                continue
            surface = word[0]
            prefixes = (surface[:end] for end in range(1, len(surface))) if prefinal else (surface,)
            analyses = set()
            annotated = set()
            for prefix in prefixes:
                stem = separate_ending(prefix, ending)
                if not stem:
                    continue
                if annotate is not None:
                    if tokens is None:
                        tokens = annotate(text)
                    if host := annotated_host(tokens, word, stem, predicates):
                        annotated.add(host)
                analyses.update(conjugations.get(stem, ()))
                last = ord(stem[-1]) - 0xAC00
                if not (ending.startswith("으") and last % 28 == 8):
                    analyses.update((stem + "다", pos) for pos in predicates.get(stem + "다", ()))
                first = ending[0]
                drops_rieul = first in "ㄴㅂㅅ" or (
                    "가" <= first <= "힣" and (ord(first) - 0xAC00) // 588 in (2, 7, 9)
                )
                if last % 28 == 0 and drops_rieul:
                    rieul_lemma = stem[:-1] + chr(ord(stem[-1]) + 8) + "다"
                    analyses.update((rieul_lemma, pos) for pos in predicates.get(rieul_lemma, ()))
            inferred = annotated or {(lemma, pos, word.start()) for lemma, pos in analyses}
            if len(inferred) == 1:
                query, pos, start = next(iter(inferred))
                candidates.append((query, pos, start, word.end(), bool(annotated)))
        if len(candidates) != 1:
            continue
        query, pos, start, end, annotated = candidates[0]
        yield {
            "ending": ending,
            "query": query,
            "pos": pos,
            "text": text,
            "surface": text[start:end],
            "gold_byte_start": len(text[:start].encode()),
            "gold_byte_end": len(text[:end].encode()),
            "expected": True,
            "source": source,
            "source_id": source_id,
            "alignment": "kiwi" if annotated else "dictionary-orthography",
        }


def abbreviation_cases(ending, equivalent, source_id, cases):
    for case in cases:
        stem = separate_ending(case["surface"], equivalent)
        if not stem:
            continue
        final = FINALS.find(ending[0], 1)
        if final > 0:
            last = ord(stem[-1])
            if (last - 0xAC00) % 28:
                continue
            surface = stem[:-1] + chr(last + final) + ending[1:]
        else:
            surface = stem + ending
        encoded = case["text"].encode()
        start, end = case["gold_byte_start"], case["gold_byte_end"]
        text = (encoded[:start] + surface.encode() + encoded[end:]).decode()
        yield {
            **case,
            "ending": ending,
            "text": text,
            "surface": surface,
            "gold_byte_end": start + len(surface.encode()),
            "abbreviation_relation_id": source_id,
            "derived_from_ending": equivalent,
        }


def reviewed_cases(path: Path | None, catalog):
    if path is None:
        return {}
    result = defaultdict(list)
    with path.open(encoding="utf-8") as source:
        for row in csv.DictReader(source, delimiter="\t"):
            ending = row["ending"]
            text = row["text"]
            surface = row["surface"]
            start = text.find(surface)
            if ending not in catalog or not surface or start < 0:
                raise ValueError(f"invalid reviewed example for {ending!r}")
            byte_start = len(text[:start].encode())
            result[ending].append({
                **row,
                "source": "stdict",
                "gold_byte_start": byte_start,
                "gold_byte_end": byte_start + len(surface.encode()),
                "expected": True,
            })
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("krdict", "stdict", "cache-dir", "catalog", "output-dir"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    parser.add_argument("--opendict", type=Path)
    parser.add_argument("--reviewed", type=Path, help="optional TSV of manually aligned examples")
    parser.add_argument("--kiwi", action="store_true", help="also resolve stems in context with Kiwi")
    args = parser.parse_args()
    annotate = None
    if args.kiwi:
        from kiwipiepy import Kiwi

        annotate = Kiwi().tokenize
    with args.catalog.open(encoding="utf-8") as source:
        catalog = {row["surface"] for row in csv.DictReader(source, delimiter="\t")}
    reviewed = reviewed_cases(args.reviewed, catalog)
    predicates = defaultdict(set)
    conjugations = defaultdict(set)
    examples = defaultdict(list)
    aliases = defaultdict(dict)
    primary_endings = set()
    redirects = defaultdict(list)
    sources = (
        ("krdict", args.krdict, KRDICT_SHA256, "LexicalEntry", krdict_item),
        ("stdict", args.stdict, STDICT_SHA256, "item", stdict_item),
    )
    if args.opendict is not None:
        sources += (("opendict", args.opendict, OPENDICT_SHA256, "item", opendict_item),)
    for source, path, sha256, tag, adapter in sources:
        for kind, record in load_catalog_records(source, path, sha256, tag, adapter, args.cache_dir):
            if kind == "predicate":
                pos = PREDICATE_POS[record.pos]
                predicates[record.lemma].add(pos)
                for form in record.conjugations:
                    if WORD.fullmatch(form):
                        conjugations[form].add((record.lemma, pos))
                continue
            if kind == "redirect":
                ending, definition = record
                redirects[ending.surface].append({
                    "source": source, "source_id": ending.source_id, "reference": definition,
                })
                continue
            ending, sentences, expanded = record
            if ending.surface not in catalog:
                continue
            if source != "opendict":
                primary_endings.add(ending.surface)
            examples[ending.surface].extend(
                (source, ending.source_id, normalize_text(sentence), ending.headword.endswith("-"))
                for sentence in sentences
                if not any(character in sentence for character in "\r\n")
            )
            if expanded:
                aliases[ending.surface][normalize_ending(expanded)[1]] = ending.source_id

    aligned = {
        ending: list(example_candidates(ending, examples[ending], predicates, conjugations, annotate))
        for ending in catalog
        if ending in primary_endings or not redirects[ending]
    }
    if reviewed.keys() - aligned.keys():
        raise ValueError("reviewed examples include a nonstandard redirect-only ending")
    related = defaultdict(dict)
    for short, entries in aliases.items():
        for full, source_id in entries.items():
            if full in aligned and short in aligned and full != short:
                related[short][full] = source_id
                related[full][short] = source_id

    args.output_dir.mkdir(parents=True, exist_ok=True)
    inventory = {}
    with (args.output_dir / "cases.jsonl").open("w", encoding="utf-8") as output:
        for ending in sorted(catalog):
            seen = set()
            sources = examples[ending]
            direct_cases = aligned.get(ending, [])
            cases = direct_cases or [
                case
                for equivalent, source_id in sorted(related[ending].items())
                for case in abbreviation_cases(ending, equivalent, source_id, aligned[equivalent])
            ]
            for case in [*cases, *reviewed.get(ending, [])]:
                identity = (case["query"], case["pos"], case["surface"], case["text"])
                if identity in seen:
                    continue
                seen.add(identity)
                case["id"] = f"{ending}:{len(seen)}"
                output.write(json.dumps(case, ensure_ascii=False) + "\n")
            inventory[ending] = {
                "dictionary_examples": len(sources),
                "aligned_examples": len(seen),
                "uses_abbreviation_relation": bool(cases and not direct_cases),
                "reviewed_examples": len(reviewed.get(ending, [])),
                "abbreviation_of": sorted(aliases[ending]),
                "redirect_only": ending not in aligned,
                "dictionary_redirects": redirects[ending],
            }
    (args.output_dir / "inventory.json").write_text(
        json.dumps(inventory, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
