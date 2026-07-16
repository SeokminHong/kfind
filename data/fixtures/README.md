# Real-corpus fixtures

[한국어](README.ko.md)

Entries whose feature starts with `corpus.*` in `morphology_cases.tsv` are short regression cases from public corpora. Identifiers use `corpus.<source>.<split>.<id>`.

## Sources

- `corpus.klue.*`: [KLUE revision `349481e`](https://huggingface.co/datasets/klue/klue/tree/349481ec73fff722f88e0453ca05c77a447d967c), CC BY-SA 4.0. Rows were selected by row index or guid from the `dp`, `ynat`, and `wos` train splits through the Hugging Face Dataset Viewer.
- `corpus.nsmc.*`: [NSMC commit `cc0670e`](https://github.com/e9t/nsmc/tree/cc0670e872d4ac27bfe36c87456783004b39ef6c), CC0 1.0. Rows were selected by review id from `ratings_test.txt`.

Sentences preserve the source spacing and spelling. The `no-match` cases verify that the `smart` boundary does not treat unspaced inflections or concatenated news-title suffixes as partial tokens.

`walk_hang_stress.txt` is a constructed regression fixture. It verifies the
product contract for homographic verb inflections, productive ending coverage,
predicate nominalizations inside compounds, auxiliary continuations, and
derived-lemma negatives. `verify-gold` requires 97 logical starts for
`v:걷다` and 21 for `v:걸다` with the full resources.
