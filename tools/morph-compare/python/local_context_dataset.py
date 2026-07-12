from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path

from dataset import (
    BenchmarkCase,
    GoldCandidate,
    Sentence,
    parse_conllu,
    positive_case,
    rank,
    sha256,
)


COPULA_TAGS = {"jp", "vcp", "vcn"}


@dataclass(frozen=True)
class TargetAnalysis:
    source: str
    raw_tag: str
    raw_lemma: str
    query: str
    pos: str
    negative_surface_cues: tuple[str, ...]
    positive_cases: int
    negative_cases: int

    @classmethod
    def from_manifest(cls, value: dict[str, object]) -> TargetAnalysis:
        return cls(
            source=str(value["source"]),
            raw_tag=str(value["raw_tag"]).lower(),
            raw_lemma=str(value["raw_lemma"]),
            query=str(value["query"]),
            pos=str(value["pos"]),
            negative_surface_cues=tuple(
                str(surface) for surface in value["negative_surface_cues"]
            ),
            positive_cases=int(value["positive_cases"]),
            negative_cases=int(value["negative_cases"]),
        )

    def matches(self, candidate: GoldCandidate) -> bool:
        return (
            candidate.source == self.source
            and candidate.raw_tag.lower() == self.raw_tag
            and candidate.raw_lemma == self.raw_lemma
            and candidate.query == self.query
            and candidate.pos == self.pos
        )


def with_target(
    case: BenchmarkCase, target: TargetAnalysis, slice_name: str
) -> dict[str, object]:
    record = asdict(case)
    record.update(
        {
            "slice": slice_name,
            "target_group": f"{target.source}/{target.raw_tag}",
            "target_raw_tag": target.raw_tag,
            "target_raw_lemma": target.raw_lemma,
        }
    )
    return record


def positive_cases(
    sentences: list[Sentence], target: TargetAnalysis
) -> list[dict[str, object]]:
    return [
        with_target(positive_case(candidate), target, "gold-copula")
        for sentence in sentences
        for candidate in sentence.candidates
        if target.matches(candidate)
    ]


def negative_cases(
    sentences: list[Sentence], target: TargetAnalysis
) -> list[dict[str, object]]:
    selected = []
    for sentence in sentences:
        contains_surface_cue = any(
            surface in sentence.text for surface in target.negative_surface_cues
        )
        if not sentence.fully_aligned or not contains_surface_cue:
            continue
        gold = {(candidate.query, candidate.pos) for candidate in sentence.candidates}
        if (target.query, target.pos) in gold:
            continue
        case = BenchmarkCase(
            id=f"local-neg:{target.source}:{target.raw_tag}:{sentence.sent_id}",
            source=target.source,
            sent_id=sentence.sent_id,
            query=target.query,
            pos=target.pos,
            text=sentence.text,
            expected=False,
            gold_byte_start=None,
            gold_byte_end=None,
            gold_token_id=None,
            gold_raw_lemma=None,
            gold_raw_tag=None,
            paired_positive_id=None,
        )
        selected.append(with_target(case, target, "surface-without-gold"))
    return selected


def excluded_copulas(
    sentences: list[Sentence], targets: list[TargetAnalysis]
) -> Counter[str]:
    excluded: Counter[str] = Counter()
    for sentence in sentences:
        for candidate in sentence.candidates:
            if candidate.raw_tag.lower() not in COPULA_TAGS:
                continue
            if any(target.matches(candidate) for target in targets):
                continue
            key = ":".join(
                [
                    candidate.source,
                    candidate.raw_tag.lower(),
                    candidate.raw_lemma,
                    candidate.query,
                ]
            )
            excluded[key] += 1
    return excluded


def build_local_context_dataset(
    manifest_path: Path,
    sources_dir: Path,
    output: Path,
    metadata_path: Path,
) -> dict[str, object]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 2:
        raise ValueError("unsupported source manifest schema")
    config = manifest["local_context"]
    split_name = str(config["split"])
    seed = str(config["seed"])
    targets = [TargetAnalysis.from_manifest(item) for item in config["analyses"]]
    if not targets or any(not target.negative_surface_cues for target in targets):
        raise ValueError("local-context analyses require negative surface cues")
    if len(set(targets)) != len(targets):
        raise ValueError("local-context analyses are not unique")
    source_names = {str(source["name"]) for source in manifest["sources"]}
    unknown_sources = sorted({target.source for target in targets} - source_names)
    if unknown_sources:
        raise ValueError(
            f"local-context analyses reference unknown sources: {unknown_sources}"
        )

    all_cases: list[dict[str, object]] = []
    group_counts = []
    source_metadata = []
    excluded: Counter[str] = Counter()
    for source in manifest["sources"]:
        split = source["splits"].get(split_name)
        if split is None:
            raise ValueError(f"source {source['name']} has no {split_name} split")
        source_path = sources_dir / split["data_file"]
        if sha256(source_path) != split["data_sha256"]:
            raise ValueError(f"source hash mismatch: {source['name']}")
        sentences, parsing = parse_conllu(source["name"], source_path)
        source_targets = [target for target in targets if target.source == source["name"]]
        excluded.update(excluded_copulas(sentences, source_targets))
        source_positive_count = source_negative_count = 0
        for target in source_targets:
            positives = positive_cases(sentences, target)
            negatives = negative_cases(sentences, target)
            if len(positives) != target.positive_cases or len(negatives) != target.negative_cases:
                raise ValueError(
                    f"local-context count mismatch for {target.source}/{target.raw_tag}: "
                    f"expected {target.positive_cases}/{target.negative_cases}, "
                    f"got {len(positives)}/{len(negatives)}"
                )
            all_cases.extend(positives)
            all_cases.extend(negatives)
            source_positive_count += len(positives)
            source_negative_count += len(negatives)
            group = asdict(target)
            group["negative_surface_cues"] = list(target.negative_surface_cues)
            group_counts.append(group)
        source_metadata.append(
            {
                "name": source["name"],
                "split": split_name,
                "data_file": split["data_file"],
                "data_url": split["data_url"],
                "data_sha256": split["data_sha256"],
                "license": source["license"],
                "license_file": source["license_file"],
                "parsing": parsing,
                "positive_cases": source_positive_count,
                "negative_cases": source_negative_count,
            }
        )

    if sum(excluded.values()) != int(config["expected_excluded_candidates"]):
        raise ValueError(
            "local-context excluded candidate count mismatch: "
            f"expected {config['expected_excluded_candidates']}, got {sum(excluded.values())}"
        )
    all_cases.sort(key=lambda case: rank(seed, "local-context-order", case["id"]))
    case_ids = {str(case["id"]) for case in all_cases}
    if len(case_ids) != len(all_cases):
        raise ValueError("local-context case IDs are not unique")

    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as fixture_file:
        for case in all_cases:
            fixture_file.write(json.dumps(case, ensure_ascii=False, sort_keys=True) + "\n")
    metadata = {
        "schema_version": 1,
        "split": "dev-local-context",
        "ud_release": manifest["ud_release"],
        "seed": seed,
        "fixture_sha256": sha256(output),
        "cases": len(all_cases),
        "positive_cases": sum(bool(case["expected"]) for case in all_cases),
        "negative_cases": sum(not case["expected"] for case in all_cases),
        "group_counts": group_counts,
        "excluded_candidates": dict(sorted(excluded.items())),
        "sources": source_metadata,
    }
    metadata_path.write_text(
        json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return metadata


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    args = parser.parse_args()
    metadata = build_local_context_dataset(
        args.manifest, args.sources, args.output, args.metadata
    )
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
