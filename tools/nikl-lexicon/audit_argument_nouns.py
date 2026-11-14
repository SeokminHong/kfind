#!/usr/bin/env python3
"""Extract nominal argument evidence from the pinned Korean Basic Dictionary."""

import argparse
import re
from collections import defaultdict
from pathlib import Path

from audit_predicate_frames import direct_feat, iter_entries, normalize_headword

PLACE_CATEGORIES = {
    "개념 > 지역",
    "교육 > 교육 기관",
    "교육 > 학교 시설",
    "사회 생활 > 교통 이용 장소",
    "삶 > 치료 시설",
    "삶 > 여가 시설",
    "주생활 > 건물 종류",
    "주생활 > 주거 지역",
}
FINANCIAL_CATEGORY = "경제 생활 > 경제 수단"
PERSON_CATEGORIES = {"인간 > 사람의 종류", "교육 > 교수 학습 주체"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    places = defaultdict(set)
    financial = defaultdict(set)
    people = defaultdict(set)
    rights = defaultdict(set)
    scheduled = set()
    for entry in iter_entries(args.archive):
        lemma, _ = normalize_headword(direct_feat(entry.find("./Lemma"), "writtenForm") or "")
        category = direct_feat(entry, "semanticCategory")
        source_id = entry.get("val")
        if direct_feat(entry, "partOfSpeech") == "명사":
            if category in PLACE_CATEGORIES:
                places[lemma].add(source_id)
            if category == FINANCIAL_CATEGORY:
                financial[lemma].add(source_id)
            if category in PERSON_CATEGORIES:
                people[lemma].add(source_id)
            # Require a rights sense and an example using that headword as the
            # object of 가지다 or its short form 갖다 in the same sense.
            for sense in entry.findall("./Sense"):
                definition = direct_feat(sense, "definition") or ""
                if not definition.endswith(" 권리."):
                    continue
                if any(
                    re.search(rf"(?<![가-힣]){re.escape(lemma)}[을를] (?:가지|갖)", feature.get("val", ""))
                    for feature in sense.findall('.//feat[@att="example"]')
                ):
                    rights[lemma].add(source_id)
        if lemma == "늦다" and direct_feat(entry, "partOfSpeech") == "동사":
            for sense in entry.findall("./Sense"):
                if not any(
                    feature.get("val") == "1이 2에 늦다"
                    for feature in sense.findall('.//feat[@att="syntacticPattern"]')
                ):
                    continue
                for feature in sense.findall('.//feat[@att="example"]'):
                    example = feature.get("val", "")
                    if example.endswith("에 늦다."):
                        scheduled.add(example.removesuffix("에 늦다."))
                    scheduled.update(re.findall(r"([가-힣]+ 시간)에 (?:항상 )?늦", example))
    print("// audit_argument_nouns.py로 생성한 한국어기초사전 표제어 및 문형 용례.")
    for name, entries in [
        ("PLACE_NOUNS", places),
        ("FINANCIAL_NOUNS", financial),
        ("POSSESSED_RIGHTS", rights),
        ("PERSON_NOUNS", people),
        ("SCHEDULED_COMPLEMENTS", {word: {"61181"} for word in scheduled}),
    ]:
        print(f'const {name}: &[&str] = &[')
        for word, ids in sorted(entries.items()):
            print(f'    "{word}", // {"|".join(sorted(ids))}')
        print("];")


if __name__ == "__main__":
    main()
