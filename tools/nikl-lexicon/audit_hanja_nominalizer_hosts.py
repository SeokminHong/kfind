#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import re
import sys
from collections import defaultdict
from collections.abc import Iterator
from pathlib import Path
from xml.etree.ElementTree import Element

sys.path.insert(0, str(Path(__file__).parent / "python"))

from nikl_catalog import load_catalog_records  # noqa: E402
from nikl_import import STDICT_SHA256, normalize_headword  # noqa: E402


HANJA = re.compile(r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]+")
NOMINAL_TAGS = {"NNG", "NNP", "NNB", "NNBC", "NP"}


def mecab_compounds(directory: Path) -> set[str]:
    hosts = set()
    for path in directory.glob("*.csv"):
        with path.open(encoding="utf-8", newline="") as file:
            for row in csv.reader(file):
                if row[4] not in NOMINAL_TAGS or row[8] != "Compound":
                    continue
                head, tag, _ = row[11].split("+", 1)[0].split("/")
                if (
                    len(head) == 1
                    and "가" <= head <= "힣"
                    and (ord(head) - ord("가")) % 28 in (16, 26)
                    and tag in NOMINAL_TAGS
                    and row[0].startswith(head)
                ):
                    hosts.add(row[0])
    return hosts


def attestation(
    entry: Element, hosts: set[str], heads: set[str]
) -> Iterator[tuple[str, str, str | None]]:
    info = entry.find("word_info")
    if info is None:
        return
    raw = info.findtext("word", "")
    word, _ = normalize_headword(raw)
    if word not in hosts and word not in heads:
        return
    origins = [node.text or "" for node in info.findall("./original_language_info/original_language")]
    source_id = entry.findtext("target_code", "")
    if word in heads:
        if len(origins) == 1 and len(origins[0]) == 1 and HANJA.fullmatch(origins[0]):
            yield word, source_id, origins[0]
        return
    explicit_first_hanja = (
        len(origins) > 1
        and len(origins[0]) == 1
        and HANJA.fullmatch(origins[0]) is not None
    ) or (
        len(origins) == 1
        and len(origins[0]) == len(word)
        and HANJA.fullmatch(origins[0]) is not None
    )
    positions = {node.text for node in info.findall("pos_info/pos")}
    if raw.startswith(word[0] + "-") and explicit_first_hanja and "명사" in positions:
        yield word, source_id, origins[0][0]
    else:
        yield word, source_id, None


def main() -> None:
    parser = argparse.ArgumentParser(
        description="용언 명사형과 겹칠 수 있는 합성 명사의 한자 첫 성분을 확인합니다."
    )
    parser.add_argument("--stdict", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--mecab", type=Path, required=True)
    args = parser.parse_args()

    hosts = mecab_compounds(args.mecab)
    if not hosts:
        parser.error("MeCab 사전에서 해당 합성 명사를 찾지 못했습니다.")
    heads = {host[0] for host in hosts}
    head_readings = defaultdict(set)
    entries = defaultdict(list)
    for word, source_id, origin in load_catalog_records(
        "stdict", args.stdict, STDICT_SHA256, "item",
        lambda entry: attestation(entry, hosts, heads), args.cache_dir,
    ):
        if word in heads:
            head_readings[word].add(origin)
        else:
            entries[word].append((source_id, origin))

    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    writer.writerow(("word", "stdict_ids"))
    for word, readings in sorted(entries.items()):
        if all(origin in head_readings[word[0]] for _, origin in readings):
            writer.writerow((word, "|".join(sorted(source_id for source_id, _ in readings))))
    print(f"MeCab {len(hosts)}개, 표준국어대사전 {len(entries)}개 대조", file=sys.stderr)


if __name__ == "__main__":
    main()
