//! Trim dangling Markdown structures from a streaming buffer.
//!
//! During LLM streaming the buffer often ends mid-structure (half-open code fence,
//! unpaired `$$`, half-written table). Feeding that to a Markdown renderer causes
//! visual flicker when the closing token finally arrives. This crate takes the
//! current buffer and returns the largest prefix that is safe to render.
//!
//! Everything is a pure function over `&str` — no state, no allocations, just a
//! `Cow::Borrowed` slice back.
//!
//! ```
//! use streaming_markdown_kit::sanitize_streaming_markdown;
//!
//! let chunk = "Hello\n```rust\nfn main() {";
//! let safe = sanitize_streaming_markdown(chunk);
//! assert_eq!(&*safe, "Hello\n");
//! ```

#![warn(clippy::all)]

use std::borrow::Cow;

/// Which dangling structures to trim. Each flag can be turned off independently
/// when the downstream renderer already handles that structure gracefully on
/// partial input.
///
/// Typical settings:
/// - Default (all `true`): renderer uses sub-widgets for code/math/table (e.g.
///   Makepad `Markdown` with `use_code_block_widget: true`, or x-markdown-style
///   custom React components).
/// - `trim_unclosed_fence = false`: renderer draws code blocks inline as plain
///   monospaced text that already fades in with the surrounding prose — no need
///   to hide partial fences.
#[derive(Clone, Copy, Debug)]
pub struct SanitizeOptions {
    pub trim_unclosed_fence: bool,
    pub trim_unpaired_block_math: bool,
    pub trim_incomplete_table: bool,
}

impl Default for SanitizeOptions {
    fn default() -> Self {
        Self {
            trim_unclosed_fence: true,
            trim_unpaired_block_math: true,
            trim_incomplete_table: true,
        }
    }
}

/// Return the largest prefix of `s` that is safe to hand to a Markdown renderer
/// without causing layout flicker when later chunks complete a dangling structure.
///
/// Cuts at the earliest of:
/// - the start line of an unclosed fenced code block (``` or ~~~),
/// - the byte position of an unpaired `$$` block-math delimiter,
/// - the start of an incomplete trailing table (only header, or header plus
///   separator but no data row yet).
///
/// Never allocates. Always returns a borrow of the input.
#[must_use]
pub fn sanitize_streaming_markdown(s: &str) -> Cow<'_, str> {
    sanitize_streaming_markdown_with(s, SanitizeOptions::default())
}

/// Like [`sanitize_streaming_markdown`] but lets the caller disable individual
/// trim categories.
#[must_use]
pub fn sanitize_streaming_markdown_with(s: &str, opts: SanitizeOptions) -> Cow<'_, str> {
    let (fence_pos, math_pos) = scan_fences_and_block_math(s);
    // Table detection must be fence-aware: a `|…|` line inside an open fenced
    // code block is code content (Rust `match`, closures `|a, b|`, doc-comment
    // markdown samples), not a GFM table header. Without this guard, streaming
    // a long Rust code block that contains such patterns trims the buffer
    // back to a `\n\n` inside the fence on every chunk, causing visible
    // mid-stream content disappearance.
    let table_pos = find_incomplete_table_tail(s, fence_pos);

    let cut = [
        opts.trim_unclosed_fence.then_some(fence_pos).flatten(),
        opts.trim_unpaired_block_math.then_some(math_pos).flatten(),
        opts.trim_incomplete_table.then_some(table_pos).flatten(),
    ]
    .into_iter()
    .flatten()
    .min();

    match cut {
        Some(pos) => Cow::Borrowed(&s[..pos]),
        None => Cow::Borrowed(s),
    }
}

/// Single-pass scanner that returns `(unclosed_fence_start, unpaired_block_math_start)`.
///
/// Block-math delimiters inside a closed fenced code block are ignored — we don't
/// want `$$` in code samples to mis-count.
fn scan_fences_and_block_math(s: &str) -> (Option<usize>, Option<usize>) {
    let bytes = s.as_bytes();
    let mut fence: Option<(u8, usize, usize)> = None; // (fence_char, fence_len, line_start)
    let mut math_open: Option<usize> = None;
    let mut line_start = 0usize;

    while line_start <= bytes.len() {
        let line_end = next_newline(bytes, line_start);
        let line = &s[line_start..line_end];

        if let Some((open_ch, open_len, _)) = fence {
            if is_closing_fence(line, open_ch, open_len) {
                fence = None;
            }
            // $$ inside a fence is ignored — it's code content, not math.
        } else if let Some(open_info) = opening_fence(line) {
            let (ch, count) = open_info;
            fence = Some((ch, count, line_start));
        } else {
            scan_block_math_in_line(line, line_start, &mut math_open);
        }

        if line_end >= bytes.len() {
            break;
        }
        line_start = line_end + 1;
    }

    let fence_pos = fence.map(|(_, _, start)| start);
    (fence_pos, math_open)
}

fn next_newline(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| from + i)
        .unwrap_or(bytes.len())
}

/// If `line` (leading whitespace ≤ 3) starts with 3+ backticks or tildes, return
/// `(char, count)`. Info-string after the fence is ignored.
fn opening_fence(line: &str) -> Option<(u8, usize)> {
    let leading = leading_spaces(line);
    if leading > 3 {
        return None;
    }
    let rest = line.as_bytes().get(leading..)?;
    let first = *rest.first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let count = rest.iter().take_while(|&&b| b == first).count();
    if count >= 3 { Some((first, count)) } else { None }
}

/// A closing fence: same character, length >= opening length, and nothing but
/// whitespace after.
fn is_closing_fence(line: &str, open_ch: u8, open_len: usize) -> bool {
    let leading = leading_spaces(line);
    if leading > 3 {
        return false;
    }
    let Some(rest) = line.as_bytes().get(leading..) else {
        return false;
    };
    if rest.first().copied() != Some(open_ch) {
        return false;
    }
    let count = rest.iter().take_while(|&&b| b == open_ch).count();
    if count < open_len {
        return false;
    }
    let after = &line[leading + count..];
    after.bytes().all(|b| b == b' ' || b == b'\t')
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

/// Scan a single line (outside any fence) for `$$` delimiters and toggle
/// `math_open`. An odd total across the document leaves `math_open = Some(pos)`.
fn scan_block_math_in_line(line: &str, line_offset: usize, math_open: &mut Option<usize>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'$' {
            let escaped = i > 0 && bytes[i - 1] == b'\\';
            if !escaped {
                *math_open = match *math_open {
                    Some(_) => None,
                    None => Some(line_offset + i),
                };
            }
            i += 2;
        } else {
            i += 1;
        }
    }
}

/// If the tail block (text after the last `\n\n` or from the start) looks like
/// an incomplete GFM table, return the block's start offset.
///
/// "Incomplete" means: has a `|`-bordered header-shaped first line, and either
///   - no second line, or
///   - second line is only `|`/`-`/`:`/space characters (separator-in-progress), or
///   - valid header+separator but no data row yet.
fn find_incomplete_table_tail(s: &str, open_fence_start: Option<usize>) -> Option<usize> {
    let nn_pos = s.rfind("\n\n").map(|p| p + 2).unwrap_or(0);
    // If the buffer has an open fenced code block and the last `\n\n` falls
    // inside that fence, the tail is code content, not prose — no table can
    // exist here. Skip detection entirely. Without this guard, e.g. a Rust
    // `match` line like `Some(a) | Some(b) => …` inside a streaming code
    // block would be seen as a table header and cause the whole tail to be
    // trimmed away.
    if let Some(fence_pos) = open_fence_start {
        if nn_pos > fence_pos {
            return None;
        }
    }
    let tail_start = nn_pos;
    let tail = &s[tail_start..];
    if tail.is_empty() {
        return None;
    }

    let lines: Vec<&str> = tail.split('\n').collect();
    let first = lines[0].trim();
    if !looks_like_table_header(first) {
        return None;
    }

    let sep = lines.get(1).map(|l| l.trim()).unwrap_or("");

    // No separator line yet → header currently renders as prose; when separator
    // arrives, pulldown-cmark re-tokenises as a table and the line restyles.
    // Trim to avoid that restyle.
    if sep.is_empty() {
        return Some(tail_start);
    }

    if is_valid_separator(sep) {
        let has_data = lines.iter().skip(2).any(|l| !l.trim().is_empty());
        if has_data { None } else { Some(tail_start) }
    } else if looks_like_separator_in_progress(sep) {
        Some(tail_start)
    } else {
        // Second line is ordinary prose — this isn't a table after all.
        None
    }
}

fn looks_like_table_header(s: &str) -> bool {
    // A header must start with `|` and contain at least one more `|` (cell border).
    s.starts_with('|') && s.len() >= 2 && s.matches('|').count() >= 2
}

fn looks_like_separator_in_progress(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with('|')
        && s.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && s.contains('-')
}

fn is_valid_separator(s: &str) -> bool {
    if !s.starts_with('|') || !s.ends_with('|') || s.len() < 3 {
        return false;
    }
    let inner = &s[1..s.len() - 1];
    let cols: Vec<&str> = inner.split('|').map(str::trim).collect();
    if cols.is_empty() {
        return false;
    }
    cols.iter().all(|c| {
        !c.is_empty()
            && c.contains('-')
            && c.chars().all(|ch| matches!(ch, '-' | ':'))
    })
}
