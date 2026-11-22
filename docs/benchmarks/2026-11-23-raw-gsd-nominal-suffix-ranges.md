# 원문 GSD에서 명사 파생 접미사 앞의 정답 범위

[앞선 범위 검토](2026-11-23-raw-gsd-nominal-head-ranges.md)의
끝점 차이 548행 중, 첫 명사가 두 음절 이상의 한글이고 원문에 한 번
나타나며 품사열이 `NNG+XSN`으로 시작하는 미검토 37행을 확인했다.
`-계(界)` 6행, `-상(上)` 7행, `-성(性)` 4행, `-화(化)` 3행의
원문 문맥과 분절을 사전·국립국어원의 품사 근거와 대조했다.
[국립국어원 형태 분석 말뭉치 보고서](https://www.korean.go.kr/common/download.do?c_file_name=313036ed-dca8-473e-9ab9-e0bd7bf07565_0.pdf&file_path=reportData) ·
[한국어기초사전 -계](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=72359&nation=eng) ·
[한국어기초사전 -성](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=70804) ·
[온라인가나다 -상](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=261&pageIndex=1&qna_seq=326996) ·
[온라인가나다 -화](https://korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=309299)

`노동계의`의 `노동` [0, 6), `지구상에서`의 `지구` [89, 95),
`활동성`의 `활동` [7, 13), `온난화`의 `온난` [98, 104)처럼
명사 성분의 UTF-8 범위를 고정했다. `대구은행장상`의 `상(賞)`과
`위안화`의 `화(貨)`는 이 범위 판정에 포함되지 않는다.
[원문 품사열과 정답 범위](../../tools/morph-compare/raw-gsd-source-corpus-reviews.tsv)

정답 범위를 확정한 뒤 `168728af` 실행 파일의 같은 원문 출력으로
두 기준을 비교했다. 추가한 20행에서 반환 범위가 모두 일치했다.
원본 정답으로는 기존과 같이 1,045행에서 적중하지 않았다.
계약 FNc는 888에서 868로 줄었다. 원문 양성 844행의 추가 반환 범위
933개는 미판정이다. 교정 GSD 9,298행의 정답 기준과 반환은 같다.

| 입력과 근거 | 원본 TP, FP, TN, FN / precision, recall, F1 (%) | 계약 TP, FP, TN, FN / precision, recall, F1 (%) | FPc | FNc |
| --- | --- | --- | ---: | ---: |
| 원문 GSD 기존 | 8,290, 0, 0, 1,045 / 100, 88.81, 94.07 | 8,447, 0, 0, 888 / 100, 90.49, 95.01 | 2 | 888 |
| 원문 GSD 검토 추가 | 8,290, 0, 0, 1,045 / 100, 88.81, 94.07 | 8,467, 0, 0, 868 / 100, 90.70, 95.12 | 2 | 868 |
| 교정 GSD | 9,288, 0, 0, 10 / 100, 99.89, 99.95 | 9,288, 0, 10, 0 / 100, 100, 100 | 2 | 0 |

## 수상·통화와 표기 검토가 필요한 접미사

나머지 17행 중 13행을 같은 입력에서 확인했다. `-권`은 문맥에 따라
권한을 나타내는 `權` 3행, 자격을 증명하는 표를 나타내는 `券` 1행,
범위를 나타내는 `圈` 1행이다. `-형(型)` 2행과 `-기(氣)`, `-꾼`,
`-류(類)`, `-별(別)`, `-산(産)` 각 1행의 어기 범위를 확정했다.
`간호사분들이`의 `-분`은 빼도 원문 문장이 성립하는 높임 접미사다.
`대구은행장상`, `위안화`, `요드제`, `알바생` 네 행은 개별 결합과
표기 근거가 더 필요하다.
[국립국어원 형태 분석 말뭉치 보고서 표 8](https://www.korean.go.kr/common/download.do?c_file_name=313036ed-dca8-473e-9ab9-e0bd7bf07565_0.pdf&file_path=reportData&o_file_name=%ED%98%95%ED%83%9C+%EB%B6%84%EC%84%9D+%EB%A7%90%EB%AD%89%EC%B9%98+%EA%B5%AC%EC%B6%95.pdf) ·
[온라인가나다 -분](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=329851)

`54a32880`의 원문 출력에 두 정답 기준을 적용했다. 새로 확정한 13행의
범위가 모두 일치했다. 원문 양성 831행의 추가 반환 920개는 미판정이다.
교정 GSD의 정답 기준과 반환 목록은 같다.

| 원문 GSD 근거 | 원본 TP, FP, TN, FN / precision, recall, F1 (%) | 계약 TP, FP, TN, FN / precision, recall, F1 (%) | FPc | FNc |
| --- | --- | --- | ---: | ---: |
| 기존 | 8,290, 0, 0, 1,045 / 100, 88.81, 94.07 | 8,467, 0, 0, 868 / 100, 90.70, 95.12 | 2 | 868 |
| 13행 추가 | 8,290, 0, 0, 1,045 / 100, 88.81, 94.07 | 8,480, 0, 0, 855 / 100, 90.84, 95.20 | 2 | 855 |
