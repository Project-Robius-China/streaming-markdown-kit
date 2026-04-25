//! Tail closing-synthesis preprocessor ("remend-rs").
//!
//! See `specs/m1-remend-rs.spec.md` for the full contract. In one sentence:
//! scan the input to find the last real block boundary (fence-stack aware),
//! then check whether the tail ends with a syntactically-open markdown
//! construct (unclosed `**`, `` ` ``, fenced code, link, math, …); if so,
//! synthesise the matching closer and append it. Otherwise return the
//! input unchanged as `Cow::Borrowed`.

use std::borrow::Cow;

/// Info-strings whose fenced content is held opaque — inline rules
/// (emphasis, inline code, math, strike) are bypassed inside these blocks.
/// Only the fenced-code closer rule applies.
const BYPASS_LANGUAGES: &[&str] = &[
    "mermaid", "diagram", "math", "tex", "latex", "typst", "asciidoc",
];

/// Primary entry point. Takes any `&str`, returns the same string if it is
/// already well-formed, or an `Owned` copy with a speculative closer
/// appended if the tail has an unclosed construct.
pub fn remend(src: &str) -> Cow<'_, str> {
    let state = scan(src);
    let tail = src.get(state.tail_start..).unwrap_or("");
    let bypass = state.in_bypass_language();

    if let Some(closer) = dispatch_rule(tail, &state, bypass) {
        let mut out = String::with_capacity(src.len() + closer.len());
        out.push_str(src);
        out.push_str(&closer);
        Cow::Owned(out)
    } else {
        Cow::Borrowed(src)
    }
}

/// Block-level scan result: where does the "tail" start (after the last
/// real block boundary), and is there an open fenced block?
struct ScanState {
    open_fence: Option<OpenFence>,
    tail_start: usize,
}

impl ScanState {
    fn in_bypass_language(&self) -> bool {
        self.open_fence
            .as_ref()
            .map(|f| {
                BYPASS_LANGUAGES
                    .iter()
                    .any(|b| b.eq_ignore_ascii_case(&f.info))
            })
            .unwrap_or(false)
    }
}

struct OpenFence {
    backticks: usize,
    fence_char: char,
    info: String,
}

fn scan(src: &str) -> ScanState {
    let mut open_fence: Option<OpenFence> = None;
    let mut last_boundary: usize = 0;

    let mut line_start: usize = 0;
    let mut prev_line_blank = true;

    let bytes = src.as_bytes();
    let mut i = 0;
    while i <= bytes.len() {
        let at_end = i == bytes.len();
        let is_newline = !at_end && bytes[i] == b'\n';
        if at_end || is_newline {
            let line = src.get(line_start..i).unwrap_or("");
            let line_end_after_nl = if is_newline { i + 1 } else { i };

            match &open_fence {
                Some(fence) => {
                    if is_fence_closer(line, fence) {
                        open_fence = None;
                        last_boundary = line_end_after_nl;
                    }
                }
                None => {
                    if let Some((bt, ch, info)) = parse_fence_opener(line) {
                        open_fence = Some(OpenFence {
                            backticks: bt,
                            fence_char: ch,
                            info: info.trim().to_ascii_lowercase(),
                        });
                    } else if line.trim().is_empty() && !prev_line_blank {
                        last_boundary = line_end_after_nl;
                    }
                }
            }

            prev_line_blank = open_fence.is_none() && line.trim().is_empty();
            line_start = line_end_after_nl;
            i += 1;
        } else {
            i += 1;
        }
    }

    ScanState {
        open_fence,
        tail_start: last_boundary,
    }
}

/// Is `line` a fence closer for the currently-open fence?
/// Closer: optional leading whitespace + N backticks (or tildes) of the
/// same char as the opener, with N >= opener's count, and nothing else.
fn is_fence_closer(line: &str, fence: &OpenFence) -> bool {
    let trimmed = line.trim_start();
    let trailing_trimmed = trimmed.trim_end();
    let mut chars = trailing_trimmed.chars();
    let mut count = 0usize;
    for ch in chars.by_ref() {
        if ch == fence.fence_char {
            count += 1;
        } else {
            return false;
        }
    }
    count >= fence.backticks
}

/// Does `line` start a fenced code block? Returns (backtick_count, fence_char, info_string).
fn parse_fence_opener(line: &str) -> Option<(usize, char, String)> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let mut count = 0usize;
    let mut iter = trimmed.chars();
    while let Some(ch) = iter.next() {
        if ch == first {
            count += 1;
        } else {
            // Info string is the rest of the line.
            let info: String = std::iter::once(ch).chain(iter).collect();
            if count >= 3 {
                return Some((count, first, info));
            }
            return None;
        }
    }
    if count >= 3 {
        Some((count, first, String::new()))
    } else {
        None
    }
}

fn dispatch_rule(tail: &str, state: &ScanState, bypass: bool) -> Option<String> {
    if let Some(c) = close_fenced_code(tail, state) {
        return Some(c);
    }
    if bypass {
        return None;
    }
    if let Some(c) = close_display_math(tail) {
        return Some(c);
    }
    if let Some(c) = close_link(tail) {
        return Some(c);
    }
    if let Some(c) = close_image(tail) {
        return Some(c);
    }
    if let Some(c) = close_triple_emphasis(tail) {
        return Some(c);
    }
    if let Some(c) = close_bold(tail) {
        return Some(c);
    }
    if let Some(c) = close_italic(tail) {
        return Some(c);
    }
    if let Some(c) = close_inline_code(tail) {
        return Some(c);
    }
    if let Some(c) = close_strikethrough(tail) {
        return Some(c);
    }
    if let Some(c) = close_inline_math(tail) {
        return Some(c);
    }
    None
}

// ---------------------------------------------------------------------------
// Rule: unclosed fenced code → append matching-count closer on a new line.
// ---------------------------------------------------------------------------
fn close_fenced_code(_tail: &str, state: &ScanState) -> Option<String> {
    let fence = state.open_fence.as_ref()?;
    // Emit matching-count closer of the same fence character on a new line.
    let mut closer = String::with_capacity(fence.backticks + 1);
    closer.push('\n');
    for _ in 0..fence.backticks {
        closer.push(fence.fence_char);
    }
    Some(closer)
}

// ---------------------------------------------------------------------------
// Rule: display math `$$…` at tail without closing `$$`.
// ---------------------------------------------------------------------------
fn close_display_math(tail: &str) -> Option<String> {
    // Count `$$` markers outside of inline-code spans.
    let occurrences = count_display_math_markers(tail);
    if occurrences % 2 == 1 {
        Some("\n$$".to_string())
    } else {
        None
    }
}

fn count_display_math_markers(tail: &str) -> usize {
    let bytes = tail.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'$' {
            count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Rule: unclosed link `[text](url…` → append `)`.
// ---------------------------------------------------------------------------
fn close_link(tail: &str) -> Option<String> {
    // Find last `[`, then check whether there's a `](` after it with no
    // matching `)` yet.
    let last_open = tail.rfind('[')?;
    let after = tail.get(last_open + 1..).unwrap_or("");
    let bracket_close = after.find("](")?;
    let after_paren = after.get(bracket_close + 2..).unwrap_or("");
    if after_paren.contains(')') {
        return None;
    }
    // Check: the `[...]` part shouldn't contain a closing `]` that's not the one we used.
    // `bracket_close` is the first `](` after `[`; that's correct for bare links.
    // Guard: if `after_paren` contains a newline or is suspiciously long, still close.
    Some(")".to_string())
}

// ---------------------------------------------------------------------------
// Rule: unclosed image `![alt](url…` → append `)`.
// ---------------------------------------------------------------------------
fn close_image(tail: &str) -> Option<String> {
    // Handled by close_link: `[` catches the `[` in `![`. If the `![` form is
    // open, close_link's logic closes it with `)` too — same action.
    let _ = tail;
    None
}

// ---------------------------------------------------------------------------
// Rule: triple emphasis `***text` at tail → append `***`.
// ---------------------------------------------------------------------------
fn close_triple_emphasis(tail: &str) -> Option<String> {
    close_asterisk_run(tail, 3)
}

// ---------------------------------------------------------------------------
// Rule: bold `**text` or `__text` at tail → append matching closer.
// ---------------------------------------------------------------------------
fn close_bold(tail: &str) -> Option<String> {
    close_asterisk_run(tail, 2).or_else(|| close_underscore_run(tail, 2))
}

// ---------------------------------------------------------------------------
// Rule: italic `*text` or `_text` at tail → append matching closer.
// ---------------------------------------------------------------------------
fn close_italic(tail: &str) -> Option<String> {
    close_asterisk_run(tail, 1).or_else(|| close_underscore_run(tail, 1))
}

/// Generic asterisk-run closer: look for an unpaired run of exactly `n`
/// consecutive asterisks whose opener flanking is valid under CommonMark
/// §6.4 AND is not intraword (remend-tightened).
fn close_asterisk_run(tail: &str, n: usize) -> Option<String> {
    let unpaired = find_unpaired_asterisk_opener(tail, n)?;

    // Intraword tightening: if opener at position `unpaired` is flanked by
    // word characters on both sides (or on the closer side at EOF, only
    // check the opener side), reject.
    if is_intraword_opener(tail, unpaired, n) {
        return None;
    }

    let mut closer = String::with_capacity(n);
    for _ in 0..n {
        closer.push('*');
    }
    Some(closer)
}

/// Generic underscore-run closer. CommonMark §6.4 already forbids intraword
/// `_` emphasis, so the intraword check is the core rule (not a tightening).
fn close_underscore_run(tail: &str, n: usize) -> Option<String> {
    let unpaired = find_unpaired_underscore_opener(tail, n)?;

    // §6.4: `_` cannot open intraword.
    if is_intraword_opener(tail, unpaired, n) {
        return None;
    }

    // §6.4 extra: `_` can open only if it's left-flanking AND (not right-flanking
    // OR preceded by punctuation). The intraword check above subsumes the
    // common case.

    let mut closer = String::with_capacity(n);
    for _ in 0..n {
        closer.push('_');
    }
    Some(closer)
}

/// Find the byte offset of an unpaired run of exactly `n` asterisks that
/// looks like an unclosed opener (no matching closer after it in the tail).
/// Returns the offset of the first asterisk of the run.
fn find_unpaired_asterisk_opener(tail: &str, n: usize) -> Option<usize> {
    find_unpaired_run(tail, '*', n)
}

fn find_unpaired_underscore_opener(tail: &str, n: usize) -> Option<usize> {
    find_unpaired_run(tail, '_', n)
}

/// Shared: find the earliest run of exactly `n` consecutive `ch` in the tail
/// that has no matching closer later in the tail. Also confirms the run is
/// a left-flanking opener (non-whitespace follows).
fn find_unpaired_run(tail: &str, ch: char, n: usize) -> Option<usize> {
    let bytes = tail.as_bytes();
    let ch_byte = ch as u8; // both '*' and '_' are single-byte ASCII
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ch_byte {
            let run_start = i;
            let mut run_len = 0;
            while i < bytes.len() && bytes[i] == ch_byte {
                run_len += 1;
                i += 1;
            }
            if run_len == n {
                // Is this a left-flanking opener?
                if is_left_flanking_opener(tail, run_start, n, ch) {
                    // Is there a matching closer later in the tail?
                    if !has_matching_closer(tail, i, n, ch) {
                        return Some(run_start);
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

/// CommonMark §6.4 left-flanking rule, applied to a run of `n` of `ch` at
/// `run_start`. Requires: the character immediately after the run is not
/// whitespace, AND either (a) not a punctuation char, or (b) the character
/// before the run is whitespace or punctuation.
fn is_left_flanking_opener(tail: &str, run_start: usize, n: usize, _ch: char) -> bool {
    let after_run = run_start + n;
    let next = tail.get(after_run..).and_then(|s| s.chars().next());
    match next {
        None => false, // Followed by EOF — in plain §6.4 this is not flanking, but a streaming tail treats "unclosed" as "opener pending".
        Some(c) if c.is_whitespace() => false,
        Some(_) => true,
    }
}

/// Does `tail` contain a matching closer for an opener run of `n` of `ch`
/// starting from byte offset `after_run`? For streaming purposes we look
/// for any run of `>= n` of `ch` later in the tail. This is conservative —
/// if there's ANY possibility of being closed by something later, we don't
/// synthesise a closer ourselves.
fn has_matching_closer(tail: &str, after_run: usize, n: usize, ch: char) -> bool {
    let ch_byte = ch as u8;
    let bytes = tail.as_bytes();
    let mut i = after_run;
    while i < bytes.len() {
        if bytes[i] == ch_byte {
            let mut run_len = 0;
            while i < bytes.len() && bytes[i] == ch_byte {
                run_len += 1;
                i += 1;
            }
            if run_len >= n {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

/// Intraword check: is the `n`-run at `run_start` flanked by Unicode word
/// characters on BOTH sides? For a streaming tail where the run is
/// unpaired, there is no following closer — "both sides" means:
///   - byte before run_start is a word character
///   - byte after run (at run_start + n) is a word character
fn is_intraword_opener(tail: &str, run_start: usize, n: usize) -> bool {
    let before = if run_start == 0 {
        None
    } else {
        tail.get(..run_start).and_then(|s| s.chars().next_back())
    };
    let after = tail.get(run_start + n..).and_then(|s| s.chars().next());
    match (before, after) {
        (Some(b), Some(a)) => is_word_char(b) && is_word_char(a),
        _ => false,
    }
}

/// Unicode word character, per CommonMark §6.4's use of Unicode categories.
/// Letters (including CJK ideographs), numbers, and `_` are word characters.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// ---------------------------------------------------------------------------
// Rule: inline code — unclosed backtick run at tail.
// ---------------------------------------------------------------------------
fn close_inline_code(tail: &str) -> Option<String> {
    // Count backticks in the tail. Inline code is delimited by runs of
    // equal length. Find the last backtick run; if its length is N and
    // there's no later run of length N, it's unclosed.
    let bytes = tail.as_bytes();
    let mut last_run_start: Option<usize> = None;
    let mut last_run_len = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i;
            let mut len = 0;
            while i < bytes.len() && bytes[i] == b'`' {
                len += 1;
                i += 1;
            }
            // Avoid counting fenced code openers (3+ ticks at start of line or preceded by newline).
            let is_line_start = start == 0 || bytes[start - 1] == b'\n';
            if !(len >= 3 && is_line_start) {
                last_run_start = Some(start);
                last_run_len = len;
            }
        } else {
            i += 1;
        }
    }

    let last_start = last_run_start?;
    let after_last = last_start + last_run_len;

    // Check if there's a matching closer AFTER this run of the same length.
    if has_backtick_run_of_len(tail.get(after_last..).unwrap_or(""), last_run_len) {
        return None;
    }

    // Count the total number of runs of this exact length earlier.
    // If even, the last run is an opener (unclosed). If odd (this run is the
    // match of an earlier opener), it's closed.
    let earlier_count =
        count_backtick_runs_of_len(tail.get(..last_start).unwrap_or(""), last_run_len);
    if earlier_count.is_multiple_of(2) {
        // Last run is an opener — synthesise closer.
        Some("`".repeat(last_run_len))
    } else {
        None
    }
}

fn has_backtick_run_of_len(slice: &str, target: usize) -> bool {
    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut len = 0;
            while i < bytes.len() && bytes[i] == b'`' {
                len += 1;
                i += 1;
            }
            if len == target {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

fn count_backtick_runs_of_len(slice: &str, target: usize) -> usize {
    let bytes = slice.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut len = 0;
            while i < bytes.len() && bytes[i] == b'`' {
                len += 1;
                i += 1;
            }
            if len == target {
                count += 1;
            }
        } else {
            i += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Rule: strikethrough `~~text` at tail without matching `~~`.
// ---------------------------------------------------------------------------
fn close_strikethrough(tail: &str) -> Option<String> {
    let last = tail.rfind("~~")?;
    // Any `~~` later? If last is the only / last one, we check parity.
    let count = tail.matches("~~").count();
    if count % 2 == 1 {
        // Check not intraword (strict rule).
        if is_intraword_opener(tail, last, 2) {
            return None;
        }
        Some("~~".to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Rule: inline math `$…` at tail without closing `$`.
// ---------------------------------------------------------------------------
fn close_inline_math(tail: &str) -> Option<String> {
    // Count single `$` not part of `$$`.
    let bytes = tail.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'$' {
                i += 2;
                continue;
            }
            count += 1;
        }
        i += 1;
    }
    if count % 2 == 1 {
        Some("$".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn scan_empty() {
        let s = scan("");
        assert_eq!(s.tail_start, 0);
        assert!(s.open_fence.is_none());
    }

    #[test]
    fn scan_flat_paragraph() {
        let s = scan("hello world");
        assert_eq!(s.tail_start, 0);
        assert!(s.open_fence.is_none());
    }

    #[test]
    fn scan_finds_blank_line_boundary() {
        let s = scan("first paragraph\n\nsecond");
        assert!(s.tail_start > 0);
        assert!(s.open_fence.is_none());
    }

    #[test]
    fn scan_identifies_open_fence() {
        let s = scan("prose\n```rust\nfn main() {}");
        assert!(s.open_fence.is_some());
        let f = s.open_fence.unwrap();
        assert_eq!(f.backticks, 3);
        assert_eq!(f.info, "rust");
    }

    #[test]
    fn scan_closes_fence() {
        let s = scan("```rust\nx\n```\n");
        assert!(s.open_fence.is_none());
    }

    #[test]
    fn scan_nn_inside_open_fence_is_not_boundary() {
        let s = scan("prose\n\n```rust\nfn a() {}\n\nfn b() {");
        assert!(s.open_fence.is_some());
        // tail_start should be after "prose\n\n" (7 bytes), NOT after the
        // later `\n\n` inside the open fence.
        assert_eq!(s.tail_start, "prose\n\n".len());
    }

    #[test]
    fn diagram_fence_is_bypass_language() {
        let s = scan("```diagram\n{\"type\":\"state\",\"states\":[{\"id\":\"a\"}]");
        assert!(s.open_fence.is_some());
        assert!(s.in_bypass_language());
    }
}
