# pdf_to_md

PDF에서 텍스트를 추출해 Markdown(`.md`) 파일로 저장하는 CLI 도구입니다.

## 빌드

```sh
cargo build --release
```

빌드 결과물: `target/release/pdf_to_md(.exe)`

## 기본 사용법

```sh
pdf_to_md <입력.pdf>
```

입력 파일과 같은 위치에 같은 이름의 `.md` 파일이 생성됩니다.
예: `book.pdf` → `book.md`

## 옵션

| 옵션 | 설명 |
|---|---|
| `-o, --output <파일>` | 출력 파일 경로 지정 |
| `-f, --from <N>` | 추출 시작 페이지 (1부터, 기본: 첫 페이지) |
| `-t, --to <N>` | 추출 종료 페이지 (포함, 기본: 마지막 페이지) |
| `--no-page-markers` | 페이지 헤딩(`## Page N`)을 출력하지 않음 |
| `--keep-linebreaks` | PDF의 줄바꿈을 그대로 유지 (기본은 문단 단위로 한 줄 reflow) |

## 예시

특정 페이지 범위만 추출:

```sh
pdf_to_md book.pdf -f 221 -t 274 -o chapter6.md
```

페이지 구분 헤딩 없이 본문만:

```sh
pdf_to_md book.pdf --no-page-markers -o plain.md
```

PDF의 줄바꿈을 보존(원본 레이아웃 유지):

```sh
pdf_to_md book.pdf --keep-linebreaks
```

## 출력 형식

페이지 구분은 Markdown H2 헤딩으로 표시됩니다.

```markdown
## Page 1

본문 내용...

## Page 2

본문 내용...
```

## 동작 메모

- 기본 동작은 PDF의 줄바꿈을 제거하고 **문단 단위로 한 줄로 reflow**합니다.
- 영문 끝 하이픈 줄바꿈(`under-` + `standing` → `understanding`)은 자동으로 이어 붙입니다.
- 한중일(CJK) 문자가 이어지는 경우에는 공백을 넣지 않고 그대로 이어 붙입니다.
- 실행 중 `unknown glyph name ...`, `Unicode mismatch ...` 같은 메시지가 stderr로 출력될 수 있는데, 이는 PDF 폰트 매핑 과정의 정보성 메시지이며 추출 결과에는 영향이 없습니다.

## 제한 사항

- 스캔본 PDF(텍스트 레이어가 없는 이미지 PDF)는 추출되지 않습니다. OCR이 별도로 필요합니다.
- 다단/표 레이아웃은 텍스트가 모두 추출되지만 순서가 섞일 수 있습니다.
- 이미지/벡터 그래픽은 추출하지 않습니다.
