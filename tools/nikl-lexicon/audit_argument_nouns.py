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

FOOD_CATEGORIES = {"과일", "곡류", "채소", "음식", "음료", "식재료", "음식 재료"}
def frame_nominal_roles(entry):
    category = direct_feat(entry, "semanticCategory") or ""
    definitions = [direct_feat(sense, "definition") or "" for sense in entry.findall("./Sense")]
    if category == "동식물 > 동물류":
        yield "animal"
    if any(re.search(r"영상이 나타나는 면\.$|(?:컴퓨터|텔레비전|카메라).*화면\.$|영화.*비추.*막\.$", text) for text in definitions):
        yield "display-surface"
    if category == "주생활 > 생활 용품":
        yield "household-article"
    if category.removeprefix("식생활 > ") in FOOD_CATEGORIES:
        yield "food"
    if category in {"인간 > 생리 현상", "삶 > 병과 증상"} and any(
        re.search(r"(?:피부|몸|눈|코|입).*?(?:나오|흐르|흘러).*?(?:액체|물)\.$", text)
        for text in definitions
    ):
        yield "body-secretion"
    if category in {"사회 생활 > 소통 수단", "사회 생활 > 말", "교육 > 학문 용어"} and any(
        re.search(r"말을 적는 기호\.$|적은 기호 체계\.$|말소리의 단위\.$|글자\.$", text)
        for text in definitions
    ):
        yield "orthographic-unit"
    if any(
        re.search(r"(?:그릇|병)\.$", text) and category == "주생활 > 생활 용품"
        or re.search(r"(?:머물|지내).*(?:곳|방|숙소)\.$", text)
        for text in definitions
    ):
        yield "emptyable"
    if any(
        re.search(r"(?:이야기|글|말이나 일|법률|규정|문장).*(?:부분|한 마디|한 토막)\.$", text)
        for text in definitions
    ):
        yield "content-unit"
    if any(
        re.search(r"음파가 귀에 들리는 것\.$|(?:들리|울리|내는|나는).*소리\.$", text)
        for text in definitions
    ):
        yield "audible-phenomenon"
    if (category == FINANCIAL_CATEGORY and any(
        re.search(r"(?:돈|자금|증권|화폐|재산|금액|권리|돈으로 바꿀 수 있는 것)\.$", text)
        for text in definitions
    )) or any(
        re.search(r"(?:자기의 것으로.*사용하는 권리|옮길 수 없는 재산|금융 기관에 돈을 맡김.*그 돈)\.$", text)
        for text in definitions
    ):
        yield "ownership-asset"


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
        definition = direct_feat(entry.find("./Sense"), "definition") or ""
        return category in PERSON_CATEGORIES or definition.endswith(("사람.", "인물."))
    definition = direct_feat(entry.find("./Sense"), "definition") or ""
    return category in SPATIAL_CATEGORIES and SPATIAL_DEFINITION_HEAD.search(definition)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--nominal-role", choices=("spatial", "temporal", "action", "person", "schedule"))
    parser.add_argument("--frame-nouns", action="store_true")
    args = parser.parse_args()
    if args.frame_nouns:
        entries = defaultdict(set)
        for entry in iter_entries(args.archive):
            if direct_feat(entry, "partOfSpeech") != "명사":
                continue
            lemma, _ = normalize_headword(direct_feat(entry.find("./Lemma"), "writtenForm") or "")
            if lemma and all("가" <= char <= "힣" for char in lemma):
                for role in frame_nominal_roles(entry):
                    entries[role, lemma].add(entry.get("val"))
        print("# 한국어기초사전 문형 명사 부류, 표면, 표제어 ID")
        for (role, lemma), ids in sorted(entries.items()):
            print(role, lemma, "|".join(sorted(ids)), sep="\t")
        return
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
