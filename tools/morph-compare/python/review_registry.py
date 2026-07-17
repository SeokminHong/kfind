from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

try:
    from .dataset import (
        apply_sentence_review,
        load_sentence_reviews,
        parse_conllu,
        resolve_source_set,
        sentence_review_path,
        sha256,
    )
except ImportError:
    from dataset import (
        apply_sentence_review,
        load_sentence_reviews,
        parse_conllu,
        resolve_source_set,
        sentence_review_path,
        sha256,
    )


def build_review_registry(
    *,
    manifest_path: Path,
    sources_dir: Path,
    output: Path,
    metadata_path: Path,
) -> dict[str, object]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    sources, _, scoring_status = resolve_source_set(manifest, "canonical")
    if scoring_status != "scored":
        raise ValueError("review registry requires a scored canonical source set")
    review_path = sentence_review_path(manifest_path, manifest, "canonical")
    if review_path is None:
        raise ValueError("canonical source set requires sentence reviews")
    reviews = load_sentence_reviews(review_path)
    seed = str(manifest["seed"])

    rows = []
    source_metadata = []
    reason_counts: Counter[str] = Counter()
    for split_name in sorted(reviews["splits"]):
        for source in sources:
            source_name = str(source["name"])
            split = source["splits"].get(split_name)
            if split is None:
                raise ValueError(f"source {source_name} has no {split_name} split")
            source_path = sources_dir / split["data_file"]
            if sha256(source_path) != split["data_sha256"]:
                raise ValueError(f"source hash mismatch: {source_name}")
            sentences, _ = parse_conllu(source_name, source_path)
            _, review_summary = apply_sentence_review(
                sentences=sentences,
                source_name=source_name,
                split_name=split_name,
                seed=seed,
                reviews=reviews,
                review_file=review_path.name,
            )
            sentence_index = {sentence.sent_id: sentence for sentence in sentences}
            entry = reviews["splits"][split_name][source_name]
            for rejection in entry["rejected"]:
                sent_id = str(rejection["sent_id"])
                sentence = sentence_index[sent_id]
                reason_class = str(rejection["reason_class"])
                reason_counts[reason_class] += 1
                rows.append(
                    {
                        "id": f"review-rejected:{source_name}:{split_name}:{sent_id}",
                        "source": source_name,
                        "split": split_name,
                        "sent_id": sent_id,
                        "text": sentence.text,
                        "reason_class": reason_class,
                        "annotation": str(rejection["annotation"]),
                        "scoring_status": "annotation-required",
                    }
                )
            source_metadata.append(
                {
                    "name": source_name,
                    "split": split_name,
                    "data_file": split["data_file"],
                    "data_sha256": split["data_sha256"],
                    "review": review_summary,
                }
            )

    rows.sort(key=lambda row: str(row["id"]))
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as registry_file:
        for row in rows:
            registry_file.write(
                json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n"
            )
    metadata = {
        "schema_version": 1,
        "fixture_type": "sentence-robustness-candidate",
        "scoring_status": "annotation-required",
        "ud_release": manifest["ud_release"],
        "seed": seed,
        "review_file": review_path.name,
        "review_policy": reviews["review_policy"],
        "fixture_sha256": sha256(output),
        "cases": len(rows),
        "reason_counts": dict(sorted(reason_counts.items())),
        "sources": source_metadata,
    }
    metadata_path.write_text(
        json.dumps(metadata, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return metadata


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    args = parser.parse_args()
    metadata = build_review_registry(
        manifest_path=args.manifest,
        sources_dir=args.sources,
        output=args.output,
        metadata_path=args.metadata,
    )
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
