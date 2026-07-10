# Generated data

`lexicon.bin` is produced from validated POS entries with
`kfind_data::encode_pos_lexicon`. Generated binaries are release artifacts and
are not edited by hand.

Run `scripts/build-full-pos.sh` from any directory to download the pinned
source, verify both checksums, and create `data/generated/full-pos`. The output
also contains a generation manifest and the upstream Apache-2.0 license.
