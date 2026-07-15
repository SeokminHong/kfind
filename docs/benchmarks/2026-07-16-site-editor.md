# Playground 대용량 편집기

## 결론

1 MiB preset을 편집기에 반영하고 두 animation frame을 그리는 시간 중앙값은
1,926.2 ms에서 24.6 ms로 98.7% 줄었다. 기준선은 편집 text와 highlight layer에 각각
1,048,516자를 렌더링했다. Candidate는 같은 크기의 document에서 viewport의 14행,
903자만 DOM에 렌더링했다.

기준선은 기본 예제의 UTF-16 offset 12에서 실제 caret layer와 표시 layer의 가로 좌표가
2.94 px 달랐다. Candidate는 세 highlight를 실제 editable surface의 decoration으로
표시했고 caret을 첫 highlight 안에 둔 상태에서도 별도 highlight layer가 없었다.

Playground route의 JavaScript gzip 크기는 27,748 bytes에서 100,498 bytes로 72,750 bytes
늘었다. Playground는 route 단위로 지연 로드하므로 다른 문서 route의 초기 bundle에는 이
비용이 포함되지 않는다.

## 측정 조건

| 항목               | 값                                          |
| ------------------ | ------------------------------------------- |
| baseline revision  | `68d02bd8ba90feaaae3a006f59c2930a62463ca1`  |
| candidate revision | `b7d245833da59a2f1e3259fd54463439fff4a587`  |
| OS                 | macOS 26.4.1 (25E253)                       |
| machine            | MacBookPro18,2, Apple M1 Max, 32 GiB        |
| Node.js            | 24.5.0                                      |
| pnpm               | 10.33.0                                     |
| Playwright CLI     | 0.1.17                                      |
| browser            | HeadlessChrome 150.0.0.0                    |
| viewport           | 1440×1000, device pixel ratio 1             |
| build              | `pnpm --dir site run build`                 |
| 반복               | 같은 browser session에서 warm-up 1회 뒤 5회 |

입력은 Playground의 `대용량 1 MiB · literal` preset이다.

| 항목        | 값                                                                 |
| ----------- | ------------------------------------------------------------------ |
| UTF-16 길이 | 1,048,516                                                          |
| UTF-8 크기  | 1,048,576 bytes                                                    |
| SHA-256     | `3db1fcc7bd437a59cd9e3ec90fa69d7e4f0c1284a83a61542998d4fe42e01f10` |

각 revision을 별도 worktree에서 build하고 다른 port에서 preview했다.

```console
cd site
pnpm install --frozen-lockfile
pnpm run build
pnpm run preview --host 127.0.0.1 --port PORT

playwright-cli open http://127.0.0.1:PORT/playground
playwright-cli resize 1440 1000

gzip -c dist/assets/page-*.js | wc -c
```

측정 구간은 preset option의 click 직전부터 byte label이 `1,048,576 bytes`로 바뀐 뒤 두 번째
`requestAnimationFrame`까지다. WASM 검색은 120 ms debounce 뒤 별도로 실행되므로 이 값에는
query compile과 scan 시간이 포함되지 않는다. 매 반복 전에 `용언 활용 · smart` preset으로
돌아가 byte label이 `106 bytes`가 된 것을 확인했다.

## 결과

값은 `median [min, max]`다.

| metric                   |                      baseline |            candidate |            변화 |
| ------------------------ | ----------------------------: | -------------------: | --------------: |
| 1 MiB editor 반영        | 1,926.2 [1,893.2, 2,166.9] ms | 24.6 [24.0, 24.7] ms |          -98.7% |
| editable DOM text        |                   1,048,516자 |                903자 | viewport 렌더링 |
| 별도 highlight DOM text  |                   1,048,516자 |                  0자 |            제거 |
| 렌더링 line              |                     전체 입력 |                 14행 | viewport 렌더링 |
| Playground route JS      |                  78,132 bytes |        304,648 bytes |  +226,516 bytes |
| Playground route JS gzip |                  27,748 bytes |        100,498 bytes |   +72,750 bytes |

개별 측정값은 다음과 같다.

| revision  | samples (ms)                                |
| --------- | ------------------------------------------- |
| baseline  | 1,926.2, 2,166.9, 1,896.7, 2,027.9, 1,893.2 |
| candidate | 24.6, 24.5, 24.0, 24.6, 24.7                |

## 동작 검증

- 기본 예제에서 highlight 3개가 editable surface 안에 생성되고 별도 highlight layer는 0개다.
- caret을 첫 highlight 내부로 이동했을 때 selection의 focus node가 해당 decoration text node다.
- 한글 입력 뒤 `Cmd+Z`로 원래 text와 UTF-16 길이가 복원된다.
- 연속 빈 줄, emoji와 trailing newline을 포함한 8 UTF-16 code unit 입력이 그대로 유지된다.
- iPhone 15 touch emulation에서 입력, undo와 highlight layout을 다시 확인했다.
- Desktop과 mobile browser console의 error와 warning은 0개였다.
