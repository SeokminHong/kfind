# kfind

[English](README.md) | [한국어](README.ko.md)

한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI입니다.

`kfind`는 한국어 표제어나 짧은 구를 개수가 제한된 표면형 앵커로 컴파일한
다음, 말뭉치 전체에 형태소 분석기를 실행하지 않고 파일을 검색합니다.

```console
$ kfind 걷다 src docs
docs/guide.md:12: 길을 걸어 갔다.
src/example.txt:8: 손님이 오래 걸었습니다.
```

## 설치

Homebrew 릴리스는 개인 tap을 통해 배포됩니다.

```sh
brew install seokminhong/brew/kfind
```

Rust 1.85 이상으로 현재 checkout을 빌드하려면 다음 명령을 실행합니다.

```sh
cargo install --locked --path crates/kfind-cli
```

## 사용법

```text
kfind [OPTIONS] <QUERY> [PATH]...
```

쿼리에 품사 태그를 명시할 수 있습니다.

```sh
kfind 'n:사용자 v:검증하다' .
kfind 'lit:걸어' data.txt
kfind 걷다 --expand inflection --json .
```

주요 검색 옵션으로 `--glob`, `--type`, `--hidden`, `--no-ignore`,
`--encoding`, 문맥 출력 옵션(`-A`, `-B`, `-C`), `--count`,
`--files-with-matches`, `--quiet`, `--sort path`가 있습니다. 전체 CLI 규약은
`kfind --help`에서 확인할 수 있습니다.

`--explain-query`는 추론한 분석 결과와 컴파일된 앵커를 출력합니다.
`--explain-match`는 각 일치 항목의 표제어와 규칙 경로를 출력합니다.

## 표시 언어

사람이 읽는 도움말, 오류, 진단, `--explain-*` 출력은 `LC_ALL`,
`LC_MESSAGES`, `LANG` 중 비어 있지 않은 첫 값을 따릅니다. `ko` locale이면
한국어를 사용하고 나머지 값은 영어를 사용합니다. 옵션명, 허용 값, JSON 필드,
종료 코드는 locale에 따라 바뀌지 않습니다.

## 사전 데이터

핵심 불규칙 용언과 규칙은 바이너리에 포함됩니다. Homebrew는 고정된 전체 품사
사전도 `share/kfind/lexicon.bin`에 설치하므로 실행 중 네트워크 접근이 필요하지
않습니다.

전체 품사 파일이 없어도 핵심 사전과 휴리스틱을 사용해 검색을 계속합니다.
`--explain-query`는 이 프리뷰 상태를 표시합니다. `--data-dir` 또는
`KFIND_DATA_DIR`로 데이터 디렉터리를 직접 선택할 수 있습니다.

고정되고 체크섬 검증을 거친 `mecab-ko-dic` 소스에서 전체 품사 산출물을
재현할 수 있습니다.

```sh
scripts/build-full-pos.sh
cargo run --locked -p kfind-testkit --bin verify-gold -- \
  data/generated/full-pos/lexicon.bin
```

## 개발

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo bench -p kfind-testkit --bench query_matcher
scripts/compare-morphology.sh
```

형태론 fixture에는 일치·불일치 사례 413개가 있습니다. Docker 비교 도구는 실제 코퍼스
사례를 `kfind`, Kiwi, Lindera로 실행합니다. 쿼리 파싱과 잘못된 matcher
입력을 위한 fuzz target은 `fuzz/`에 있습니다.

구현 규약과 릴리스 인수 기준은
[`specs/kfind.md`](specs/kfind.md)에 있습니다.

## 릴리스

일치하는 `vX.Y.Z` 태그를 push하면 릴리스 workflow가 실행됩니다. 전체 품사
resource를 다시 빌드하고 검증한 뒤 소스·데이터·CLI 산출물을 게시하고
`SeokminHong/homebrew-brew`에 Formula PR을 엽니다. tap의 `pr-pull` label은
Formula test가 통과한 뒤에만 적용됩니다.

릴리스 workflow에는 tap 쓰기 권한이 있는 `TAP_GITHUB_TOKEN` secret이
필요합니다. Cargo package metadata에 프로젝트 license가 없으면 배포를
거부합니다.
