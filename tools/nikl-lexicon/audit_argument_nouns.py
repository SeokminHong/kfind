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
SPATIAL_CATEGORIES = {
    "개념 > 위치 및 방향",
    "교육 > 학교 시설",
    "사회 생활 > 교통 이용 장소",
    "주생활 > 건물 종류",
    "주생활 > 주거 지역",
    "주생활 > 주택 구성",
}
SPATIAL_DEFINITION_HEAD = re.compile(r"(?:장소|곳|공간|건물|지역|지방|도시|마을|시설|구역|방|자리)\.$")


def has_nominal_role(entry, lemma, role):
    category = direct_feat(entry, "semanticCategory")
    if role == "temporal":
        return category == "개념 > 시간"
    if role == "action":
        forms = {lemma + "을 하다.", lemma + "를 하다."}
        return any(
            feat.get("val") in forms
            for feat in entry.findall('.//feat[@att="example"]')
        )
    if role == "schedule":
        forms = {f"{lemma} 시간.", f"{lemma}에 참석하다.", f"{lemma}에 참가하다."}
        return category != "개념 > 시간" and (
            any(feat.get("val") in forms for feat in entry.findall('.//feat[@att="example"]'))
            or any(
                re.search(r"(?:모임|시험|기한(?:의 끝)?)\.$", direct_feat(sense, "definition") or "")
                for sense in entry.findall("./Sense")
            )
        )
    if role == "person":
        return category in PERSON_CATEGORIES
    definition = direct_feat(entry.find("./Sense"), "definition") or ""
    return category in SPATIAL_CATEGORIES and SPATIAL_DEFINITION_HEAD.search(definition)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--nominal-role", choices=("spatial", "temporal", "action", "person", "schedule"))
    args = parser.parse_args()
    if args.nominal_role:
        entries = defaultdict(set)
        for entry in iter_entries(args.archive):
            if direct_feat(entry, "partOfSpeech") != "명사":
                continue
            lemma, _ = normalize_headword(direct_feat(entry.find("./Lemma"), "writtenForm") or "")
            if (
                lemma and all("가" <= char <= "힣" for char in lemma)
                and has_nominal_role(entry, lemma, args.nominal_role)
            ):
                entries[lemma].add(entry.get("val"))
        print("# 한국어기초사전 명사와 표제어 ID")
        for lemma, ids in sorted(entries.items()):
            print(lemma, "|".join(sorted(ids)), sep="\t")
        return
    places = defaultdict(set)
    financial = defaultdict(set)
    people = defaultdict(set)
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
        ("PERSON_NOUNS", people),
        ("SCHEDULED_COMPLEMENTS", {word: {"61181"} for word in scheduled}),
    ]:
        print(f'const {name}: &[&str] = &[')
        for word, ids in sorted(entries.items()):
            print(f'    "{word}", // {"|".join(sorted(ids))}')
        print("];")


if __name__ == "__main__":
    main()
