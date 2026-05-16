use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "pdf_to_md",
    version,
    about = "PDF에서 텍스트를 추출하여 보기 좋게 정리된 .md 파일로 저장합니다."
)]
struct Cli {
    /// 입력 PDF 파일 경로
    input: PathBuf,

    /// 출력 md 파일 경로 (생략하면 입력 파일과 같은 위치에 .md로 저장)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// 추출 시작 페이지 (1부터 시작, 기본은 첫 페이지)
    #[arg(short = 'f', long)]
    from: Option<usize>,

    /// 추출 종료 페이지 (포함, 기본은 마지막 페이지)
    #[arg(short = 't', long)]
    to: Option<usize>,

    /// 페이지 구분 헤딩(## Page N)을 출력하지 않음
    #[arg(long)]
    no_page_markers: bool,

    /// 줄바꿈을 그대로 유지 (기본은 문단 단위로 한 줄로 다시 흘려보냄)
    #[arg(long)]
    keep_linebreaks: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.input.exists() {
        anyhow::bail!("입력 파일을 찾을 수 없습니다: {}", cli.input.display());
    }

    if let (Some(f), Some(t)) = (cli.from, cli.to) {
        if f == 0 || t == 0 {
            anyhow::bail!("페이지 번호는 1 이상이어야 합니다.");
        }
        if f > t {
            anyhow::bail!("--from({f})은 --to({t})보다 클 수 없습니다.");
        }
    }
    if cli.from == Some(0) || cli.to == Some(0) {
        anyhow::bail!("페이지 번호는 1 이상이어야 합니다.");
    }

    let pages = pdf_extract::extract_text_by_pages(&cli.input)
        .with_context(|| format!("PDF 텍스트 추출 실패: {}", cli.input.display()))?;

    let formatted = format_pages(
        &pages,
        !cli.no_page_markers,
        !cli.keep_linebreaks,
        cli.from,
        cli.to,
    )?;

    let output_path = cli.output.unwrap_or_else(|| default_output(&cli.input));
    fs::write(&output_path, &formatted)
        .with_context(|| format!("출력 파일 쓰기 실패: {}", output_path.display()))?;

    eprintln!("완료: {}", output_path.display());
    Ok(())
}

fn default_output(input: &Path) -> PathBuf {
    let mut out = input.to_path_buf();
    out.set_extension("md");
    out
}

/// `from` / `to`는 1-indexed, inclusive 페이지 범위 (None이면 처음/끝까지).
fn format_pages(
    pages: &[String],
    page_markers: bool,
    reflow: bool,
    from: Option<usize>,
    to: Option<usize>,
) -> Result<String> {
    let total = pages.len();

    let start = from.unwrap_or(1);
    let end = to.unwrap_or(total);

    if start > total {
        anyhow::bail!("--from={start}: PDF의 페이지 수({total})를 넘었습니다.");
    }
    let end = end.min(total);

    let mut out = String::with_capacity(pages.iter().map(|p| p.len()).sum());
    for (idx, page) in pages.iter().enumerate() {
        let page_num = idx + 1;
        if page_num < start || page_num > end {
            continue;
        }
        if page.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        if page_markers {
            out.push_str(&format!("## Page {page_num}\n\n"));
        }
        out.push_str(&clean_page(page, reflow));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn clean_page(page: &str, reflow: bool) -> String {
    let lines: Vec<String> = page.lines().map(|l| normalize_spaces(l.trim())).collect();

    // 빈 줄을 문단 경계로 보고 paragraph로 묶음 (연속 빈 줄은 한 개로 축소)
    let mut paragraphs: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in lines {
        if line.is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }

    paragraphs
        .into_iter()
        .map(|p| if reflow { reflow_paragraph(&p) } else { p.join("\n") })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 탭, NBSP, zero-width 공백 등을 일반 공백으로 정규화하고 연속 공백을 하나로 줄임.
fn normalize_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        let is_space =
            ch.is_whitespace() || ch == '\u{00A0}' || ch == '\u{200B}' || ch == '\u{FEFF}';
        if is_space {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// 한 문단의 여러 줄을 한 줄로 합친다.
/// - 영문 끝의 하이픈(-) 줄바꿈은 단어를 이어 붙임
/// - CJK 문자끼리 이어지는 경우엔 공백을 끼우지 않음
fn reflow_paragraph(lines: &[String]) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            out.push_str(line);
            continue;
        }
        if is_soft_hyphen(out.chars().last())
            && out.chars().rev().nth(1).is_some_and(|c| c.is_alphabetic())
        {
            out.pop();
            out.push_str(line);
            continue;
        }
        if needs_space_between(out.chars().last(), line.chars().next()) {
            out.push(' ');
        }
        out.push_str(line);
    }
    out
}

fn needs_space_between(prev: Option<char>, next: Option<char>) -> bool {
    let (Some(a), Some(b)) = (prev, next) else {
        return false;
    };
    !(is_cjk(a) && is_cjk(b))
}

fn is_soft_hyphen(c: Option<char>) -> bool {
    matches!(
        c,
        Some('-')        // U+002D HYPHEN-MINUS
            | Some('\u{00AD}') // SOFT HYPHEN
            | Some('\u{2010}') // HYPHEN
            | Some('\u{2011}') // NON-BREAKING HYPHEN
    )
}

fn is_cjk(c: char) -> bool {
    let v = c as u32;
    (0x3040..=0x30FF).contains(&v)       // Hiragana / Katakana
        || (0x3400..=0x4DBF).contains(&v) // CJK Ext A
        || (0x4E00..=0x9FFF).contains(&v) // CJK Unified
        || (0xAC00..=0xD7AF).contains(&v) // Hangul Syllables
        || (0x3000..=0x303F).contains(&v) // CJK punctuation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflow_joins_english_with_space() {
        let lines = vec!["Hello".to_string(), "world".to_string()];
        assert_eq!(reflow_paragraph(&lines), "Hello world");
    }

    #[test]
    fn reflow_joins_korean_without_space() {
        let lines = vec!["안녕하".to_string(), "세요".to_string()];
        assert_eq!(reflow_paragraph(&lines), "안녕하세요");
    }

    #[test]
    fn reflow_unhyphenates_english() {
        let lines = vec!["under-".to_string(), "standing".to_string()];
        assert_eq!(reflow_paragraph(&lines), "understanding");
    }

    #[test]
    fn reflow_unhyphenates_unicode_hyphen() {
        let lines = vec!["intro\u{2010}".to_string(), "duced".to_string()];
        assert_eq!(reflow_paragraph(&lines), "introduced");
    }

    #[test]
    fn normalize_collapses_spaces() {
        assert_eq!(normalize_spaces("a\u{00A0}b   c"), "a b c");
    }

    fn pages(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn page_markers_appended() {
        let p = pages(&["page one text", "page two text"]);
        let out = format_pages(&p, true, false, None, None).unwrap();
        assert!(out.contains("## Page 1"));
        assert!(out.contains("## Page 2"));
    }

    #[test]
    fn page_range_filters_correctly() {
        let p = pages(&["first", "second", "third", "fourth"]);
        let out = format_pages(&p, true, false, Some(2), Some(3)).unwrap();
        assert!(!out.contains("first"));
        assert!(out.contains("second"));
        assert!(out.contains("third"));
        assert!(!out.contains("fourth"));
        assert!(out.contains("## Page 2"));
        assert!(out.contains("## Page 3"));
    }

    #[test]
    fn page_range_past_end_errors() {
        let p = pages(&["only one page"]);
        let err = format_pages(&p, true, false, Some(5), None);
        assert!(err.is_err());
    }
}
