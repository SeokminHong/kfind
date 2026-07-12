# 생성 데이터

[English](README.md) | [한국어](README.ko.md)

`lexicon.bin`은 검증된 품사 항목으로 `kfind_data::encode_pos_lexicon`을 사용해
생성합니다. 생성된 바이너리는 릴리스 산출물이므로 직접 편집하지 않습니다.

어느 디렉터리에서든 `scripts/build-full-pos.sh`를 실행하면 고정된 소스를
다운로드하고 두 체크섬을 검증한 뒤 `data/generated/full-pos`를 생성합니다.
출력에는 생성 manifest와 upstream Apache-2.0 라이선스도 포함됩니다.
`STATS.toml`은 전체 entry·고유 표제어·품사별 entry와 품사 충돌 표제어 수를 기록합니다.
