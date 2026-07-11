# Morphology comparison image

[한국어](README.ko.md)

This development tool runs the same fixture sentences through `kfind`, Kiwi, and Lindera. External analyzers are not included in the product binary or default search path.

The default run selects the pinned `corpus.*` cases.

```sh
scripts/compare-morphology.sh
```

Results are written to `target/morph-compare/report.json` and `report.md`. After the image is built, the container runs with `--network none`.

To run the image directly:

```sh
docker build -f tools/morph-compare/Dockerfile -t kfind-morph-compare:local .
docker run --rm --network none \
  --user "$(id -u):$(id -g)" \
  -v "$PWD/target/morph-compare:/output" \
  kfind-morph-compare:local
```

`kfind` is checked against both matching and non-matching fixture expectations. Kiwi and Lindera are scored on compatible lemma and POS recovery for positive cases; analyzer output for non-matching cases is recorded but not scored.
