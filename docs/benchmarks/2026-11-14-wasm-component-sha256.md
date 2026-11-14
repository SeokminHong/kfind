# WASM 구조 리소스의 SHA-256 계산

Node의 CPU 프로파일에서 full POS와 구조 리소스를 읽을 때 SHA-256 압축 함수에
약 870 ms, payload 레코드 검사에 약 12 ms가 잡혔다. 큰 section을 압축 함수 한
번으로 처리하면 호출이 끝날 때까지 최적화된 코드로 넘어가지 못할 수 있다.
WASM에서 section을 64 KiB씩 같은 SHA-256 상태에 넣으면 호출 사이에 런타임이
최적화된 코드를 사용할 수 있다.

기준 `faa27e83`, 후보 `f37e615b`. Apple M1 Max arm64, macOS 26.6.2,
Rust/Cargo 1.97.0, wasm-pack 0.15.0, Node 26.11.1을 사용했다.
release 빌드와 동일 리소스로 프로필별 워밍업 1회 후 각각 새 프로세스로
5회 측정하며 실행 순서를 교대했다. 값은 중앙값 [최소, 최대]이며
RSS는 초기화 후 Node가 보고한 값이다.

| 프로필 | 버전 | 구조 리소스 ms | 전체 초기화 ms | RSS MiB |
| --- | --- | --- | --- | --- |
| embedded | 기준 | — | 18.82 [18.49, 19.18] | 53.99 [52.77, 55.30] |
| embedded | 후보 | — | 18.93 [18.43, 19.17] | 55.05 [53.60, 55.44] |
| embedded + component | 기준 | 911.33 [906.07, 915.39] | 930.22 [925.01, 935.37] | 140.24 [139.33, 140.69] |
| embedded + component | 후보 | 153.86 [152.06, 158.13] | 172.66 [170.89, 177.44] | 139.06 [138.78, 140.10] |
| full POS | 기준 | — | 62.10 [61.30, 62.37] | 92.35 [91.13, 92.89] |
| full POS | 후보 | — | 61.94 [61.85, 62.25] | 92.16 [91.03, 92.97] |
| full POS + component | 기준 | 907.87 [905.28, 930.01] | 969.11 [967.11, 992.61] | 165.41 [164.17, 165.71] |
| full POS + component | 후보 | 152.25 [150.69, 153.43] | 213.81 [212.41, 215.29] | 165.67 [164.58, 165.78] |

full POS와 구조 리소스를 함께 사용할 때 구조 리소스 로딩은 83.2%, 전체
초기화는 77.9% 감소했다. KAIST test query matrix 2,592건을 Node의 두
WASM 빌드에서 실행해 반환 범위, 분석 근거, 진단을 비교했으며 차이는 0건이었다.
패키지 테스트에서 payload 마지막 바이트가 바뀐 입력의 digest 오류도 확인했다.

```sh
wasm-pack build crates/kfind-wasm --target nodejs --out-dir "$module_dir" --out-name kfind --release --locked
node --expose-gc scripts/benchmark-npm-startup.cjs --probe +  "$module_dir/kfind.js" "$component" "$full_pos" "$profile"
```

[full POS와 구조 리소스 체크섬](2026-11-14-gu-exclamative.md#재현) ·
[KAIST test 입력 체크섬](2026-11-14-copula-continuation-sharing.md).

구조 리소스 크기는 37,103,813 bytes이며 WASM 파일 SHA-256은 기준
`8dfe25ec53fac66a294bb11be1ba0e0d38f759b459e365d52edba45acce80d23`,
후보 `e012e9e974488ce68a3141adc1147ee14ebe0ec7e74950d633d5b39627d31dbb`다.

패키지 CI와 같은 Node 24 계열(24.5.0)에서 동일한 WASM 두 파일로 full POS +
component를 다시 측정했다. 구조 리소스는 877.13 [873.41, 1112.36] ms에서
149.73 [146.92, 151.17] ms로, 전체 초기화는 938.54 [936.61, 1172.69] ms에서
210.58 [207.17, 213.27] ms로 줄었다. RSS는 210.58 [210.00, 213.73] MiB에서
211.38 [210.63, 211.86] MiB였다.

같은 Node 24와 npm CLI 진입점으로 `node bin/kfind.js --json 권한 "$input"`을
실행했다. 입력은 `사용자권한을 확인했습니다.`와 줄바꿈이며 SHA-256은
`84e3651f04ec48851b46d805fb39374bc3c2c430bd1c5cfafd204c9fba57a1be`다.
프로세스 시작부터 JSON 출력 완료까지 워밍업 1회 후 5회씩 교대로 측정했다.
전체 실행은 951.97 [951.88, 953.30] ms에서 223.13 [222.64, 223.52] ms로
줄었고 출력은 바이트 단위로 일치했다.
