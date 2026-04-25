//! Public fenced-code scanner for streaming consumers.
//!
//! This is intentionally small and renderer-agnostic: it identifies complete
//! CommonMark-style fenced code blocks and reports an unclosed tail fence.
//! Callers can layer language-specific validation on top.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedCodeBlock<'a> {
    pub info: &'a str,
    pub info_string: &'a str,
    pub body: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenceScanError {
    Unclosed { info: String },
}

#[must_use]
pub fn scan_fenced_code_blocks(src: &str) -> Result<Vec<FencedCodeBlock<'_>>, FenceScanError> {
    let mut blocks = Vec::new();
    let mut open: Option<OpenFence> = None;
    let mut line_start = 0;
    let bytes = src.as_bytes();
    let mut i = 0;

    while i <= bytes.len() {
        let at_end = i == bytes.len();
        let is_newline = !at_end && bytes[i] == b'\n';
        if at_end || is_newline {
            let line = src.get(line_start..i).unwrap_or("");
            let next_line_start = if is_newline { i + 1 } else { i };

            match &open {
                Some(fence) => {
                    if is_fence_closer(line, fence) {
                        blocks.push(FencedCodeBlock {
                            info: fence.info,
                            info_string: fence.info_string,
                            body: src.get(fence.body_start..line_start).unwrap_or(""),
                        });
                        open = None;
                    }
                }
                None => {
                    if let Some((count, fence_char, info_string, info)) = parse_fence_opener(line) {
                        open = Some(OpenFence {
                            count,
                            fence_char,
                            info,
                            info_string,
                            body_start: next_line_start,
                        });
                    }
                }
            }

            line_start = next_line_start;
            i += 1;
        } else {
            i += 1;
        }
    }

    if let Some(fence) = open {
        Err(FenceScanError::Unclosed {
            info: fence.info.to_string(),
        })
    } else {
        Ok(blocks)
    }
}

struct OpenFence<'a> {
    count: usize,
    fence_char: char,
    info: &'a str,
    info_string: &'a str,
    body_start: usize,
}

fn parse_fence_opener(line: &str) -> Option<(usize, char, &str, &str)> {
    let trimmed = line.trim_start().trim_end_matches('\r');
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }

    let count = trimmed.chars().take_while(|ch| *ch == first).count();
    if count < 3 {
        return None;
    }

    let info_string = trimmed[count..].trim();
    let info = info_string.split_ascii_whitespace().next().unwrap_or("");
    Some((count, first, info_string, info))
}

fn is_fence_closer(line: &str, fence: &OpenFence<'_>) -> bool {
    let trimmed = line.trim_start().trim_end_matches('\r');
    let count = trimmed
        .chars()
        .take_while(|ch| *ch == fence.fence_char)
        .count();
    count >= fence.count && trimmed[count..].trim().is_empty()
}
