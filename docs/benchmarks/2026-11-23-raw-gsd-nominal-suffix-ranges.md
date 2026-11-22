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
`위안화`의 `화(貨)`는 이 범위 판정에 포함되지 않는다. 나머지
17행의 접미사와 원문 문맥은 검토가 필요하다.
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
