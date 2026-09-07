# 문서 진입 구조와 홈 로딩 검증

문서의 읽기 순서·설치 계약·결과 해석 안내와 본문 스타일을 변경했다. 한국어 홈에서
요청한 asset은 1,114 byte 증가했다. DOMContentLoaded 중앙값은 거의 같고 load 시간의
범위는 겹친다. 이 표본에서 뚜렷한 로딩 회귀는 관측하지 않았으며 속도 개선으로도
해석하지 않는다. 검색 엔진 코드와 검색 성능 계약은 변경하지 않았다.

## 측정 조건

- 기준선: `bb44a8db6495781b23dff454a3574df102ac7ec8` (`origin/main`)
- 후보: `b4fe7b38550d4da59abc2ba4ab5bd6371f1e7566`
- 환경: macOS Darwin arm64, 빌드 Node.js 24.5.0, pnpm 10.33.0, Vite 8.1.4,
  Python 3.14.6, Chrome 152.0.7977.76, viewport 1440 × 1000
- 빌드: 각 revision에서 `pnpm --dir site install --frozen-lockfile`과
  `pnpm --dir site run build`. 동일한 lockfile·release WASM 설정을 사용했다.
- 입력: 두 빌드의 한국어 `/`. HTML SHA-256과 원시 표본은
  [측정 snapshot](2026-09-08-documentation-loading.json)에 보존했다.
- 각 profile warm-up 1회 뒤 5회. 매 회 기준선·후보 순서로 번갈아 새 browser context에서
  실행했다. Context cache는 공유하지 않으며 browser process와 OS cache는 공유한다.
- 대표값: median/min/max. navigation timing은 navigation 시작부터 각 event 종료까지다.
  Asset 크기는 요청한 `/assets/` resource의 encoded body 합계이며 HTTP header는 제외한다.

## 결과

| 항목                       | 기준선 median / min / max         | 후보 median / min / max           | median 증감 |
| -------------------------- | --------------------------------- | --------------------------------- | ----------- |
| DOMContentLoaded (ms)      | 93.50 / 89.70 / 94.20             | 93.60 / 90.40 / 97.10             | +0.11%      |
| load (ms)                  | 146.20 / 135.30 / 147.90          | 135.40 / 133.30 / 158.70          | -7.39%      |
| 홈에서 요청한 asset (byte) | 863649.00 / 863649.00 / 863649.00 | 864763.00 / 864763.00 / 864763.00 | +0.13%      |
| 홈 HTML (byte)             | 15688.00 / 15688.00 / 15688.00    | 14751.00 / 14751.00 / 14751.00    | -5.97%      |

홈 HTML을 같은 Python gzip 기본 설정으로 압축한 크기는 5,520 → 5,566 byte
(+0.83%)다. 원문 HTML 감소와 압축 크기 감소는 같은 지표가 아니다.

localhost의 Python 정적 서버로 제공했으며 네트워크 제한·CDN 압축은 적용하지 않았다.
정적 서버에는 `/api/docs-versions`가 없어 두 profile 모두 해당 요청에 404를 받는다.
이 측정은 문서 HTML·asset 로딩만 비교하며 배포 환경의 버전 API, Core Web Vitals나
실제 네트워크 지연을 평가하지 않는다. 형태 품질·CLI 성능 측정은 변경 경로가 아니므로
생략했다. 검색의 품질 보정 지표는 문서 로딩 지표에 적용하지 않는다.

## 재현

기준선 빌드의 `site/build/client`를 `output/playwright/baseline-site`에 보존하고
후보를 같은 도구 설정으로 빌드한다. 두 서버는 별도 터미널에서 실행한다.

```sh
python3 -m http.server 4174 --bind 127.0.0.1 --directory output/playwright/baseline-site
python3 -m http.server 4173 --bind 127.0.0.1 --directory site/build/client
```

Playwright CLI에서 같은 Chrome session을 사용한다. 다음 함수는 `run-code` 인자로
전달하고, 실행 후 `eval 'globalThis.kfindDocMetrics'`로 원시 결과를 읽는다.

```js
async (page) => {
  const browser = page.context().browser();
  const results = {
    browser: browser.version(),
    viewport: { width: 1440, height: 1000 },
    runs: [],
  };
  for (let iteration = 0; iteration < 6; iteration++) {
    for (const [profile, port] of [
      ["baseline", 4174],
      ["candidate", 4173],
    ]) {
      const context = await browser.newContext({ viewport: results.viewport });
      const tab = await context.newPage();
      await tab.goto("http://127.0.0.1:" + port + "/", {
        waitUntil: "networkidle",
      });
      const metrics = await tab.evaluate(() => {
        const navigation = performance.getEntriesByType("navigation")[0];
        return {
          domContentLoadedMs: navigation.domContentLoadedEventEnd,
          loadMs: navigation.loadEventEnd,
          resourceBytes: performance
            .getEntriesByType("resource")
            .filter((r) => r.name.includes("/assets/"))
            .reduce((sum, r) => sum + r.encodedBodySize, 0),
          htmlBytes: navigation.encodedBodySize,
        };
      });
      results.runs.push({ profile, iteration, ...metrics });
      await context.close();
    }
  }
  await page.evaluate((data) => {
    globalThis.kfindDocMetrics = data;
  }, results);
};
```

## 화면과 문서 검증

- 한국어·영어 65개 문서 route의 구조와 66개 전체 route의 정적 HTML·SEO 검사 통과.
- TypeScript, ESLint, Prettier 검사 통과. 문서 내부 링크·fragment 213개 확인.
- Desktop와 320px 홈을 두 차례 시각 검토. 영어 전환·설치 페이지 이동·모바일 메뉴와
  절 링크 확인. 320px 문서 전체 가로 overflow 없음.
- 설치된 네이티브 1.0.1에서 첫 검색 입력과 `--explain-query --pos verb 걷다` 실행 확인.
  특정 구조 상한 초과 입력의 누락 재현이나 npm CLI 재배포 검증은 포함하지 않는다.
