# 생성 데이터

`lexicon.bin`은 검증된 품사 항목을 `kfind_data::encode_pos_lexicon`으로 encode한
릴리즈 산출물입니다. 생성된 binary는 직접 편집하지 않습니다.

어느 디렉터리에서든 `scripts/build-full-pos.sh`를 실행할 수 있습니다. script는
고정한 source를 내려받고 두 checksum을 검증한 뒤 `data/generated/full-pos`를
생성합니다. 출력에는 생성 manifest와 upstream Apache-2.0 라이선스가 포함됩니다.
`STATS.toml`은 전체 entry, 고유 표제어, 품사별 entry와 품사 충돌 표제어 수를
기록합니다.
