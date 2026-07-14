# Enriched predicate data notice

`predicates.tsv` and `REPORT.tsv` contain normalized predicate metadata derived
from Korean Basic Dictionary, Standard Korean Language Dictionary, and
Urimalsaem snapshots supplied by the National Institute of Korean Language.
The pinned snapshot filenames, checksums, extracted fields, and record counts
are recorded in `MANIFEST.toml` and `STATS.toml`.

The generated layer records reviewed Korean predicate alternations, the
minimum same-POS regular companion analyses needed to preserve homonyms, and
dictionary surfaces that the productive rules cannot generate. Conjugation
surfaces require agreement between the Korean Basic Dictionary and Standard
Korean Language Dictionary. Predicate-to-adverb surfaces require matching
bidirectional Korean Basic Dictionary entry IDs.

These data files are distributed under the Creative Commons
Attribution-ShareAlike 2.0 Korea license (CC BY-SA 2.0 KR). Attribute the
National Institute of Korean Language and preserve the same license for
adapted data.

- Korean Basic Dictionary copyright policy:
  <https://krdict.korean.go.kr/kor/kboardPolicy/copyRightTermsInfo>
- Standard Korean Language Dictionary copyright policy:
  <https://stdict.korean.go.kr/join/copyrightPolicy.do>
- Urimalsaem copyright policy:
  <https://opendict.korean.go.kr/service/copyrightPolicy>
- License terms: <https://creativecommons.org/licenses/by-sa/2.0/kr/>

Only headwords, part-of-speech labels, conjugation forms, related-form surfaces,
and source IDs are used. Dictionary examples, definitions, multimedia, and
pronunciation assets are not redistributed.
