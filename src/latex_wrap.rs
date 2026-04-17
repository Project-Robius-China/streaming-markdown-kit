//! Auto-wrap bare LaTeX commands with `$…$` so a markdown math renderer can
//! pick them up.
//!
//! Many open-source LLMs (Kimi, DeepSeek, Qwen, Llama) emit LaTeX commands
//! (`\frac{…}{…}`, `\mathbb{Z}`, `\forall`, etc.) **without** surrounding them
//! with `$…$` delimiters. pulldown-cmark's `ENABLE_MATH` only recognises the
//! delimited form, so those commands render as literal text. This module runs
//! a streaming-safe preprocessing pass that wraps every complete, bare LaTeX
//! token with `$…$`.
//!
//! Rules the wrapper obeys:
//! - Skip anything inside a fenced code block (``` or ~~~).
//! - Skip anything already inside `$…$` or `$$…$$` so double-wrapping doesn't
//!   happen.
//! - A LaTeX token is `\<ascii-letters>` optionally followed by one or more
//!   `{balanced}` or `[balanced]` argument groups.
//! - During streaming, if a token's command name or `{…}` group appears
//!   unterminated at the end of input, it's left untouched — the next chunk
//!   will complete it and `wrap_bare_latex` is re-run on the full buffer each
//!   frame.

use std::borrow::Cow;

/// Return `s` with every bare LaTeX command wrapped in `$…$`. Non-LaTeX input
/// and input where the LaTeX is already delimited are returned as borrowed.
#[must_use]
pub fn wrap_bare_latex(s: &str) -> Cow<'_, str> {
    let ranges = find_wrap_ranges(s);
    if ranges.is_empty() {
        return Cow::Borrowed(s);
    }

    let mut out = String::with_capacity(s.len() + ranges.len() * 2);
    let mut last = 0;
    for (start, end) in &ranges {
        out.push_str(&s[last..*start]);
        out.push('$');
        out.push_str(&s[*start..*end]);
        out.push('$');
        last = *end;
    }
    out.push_str(&s[last..]);
    Cow::Owned(out)
}

fn find_wrap_ranges(s: &str) -> Vec<(usize, usize)> {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut ranges: Vec<(usize, usize)> = Vec::new();

    let mut in_fence = false;
    let mut fence_char: u8 = 0;
    let mut fence_len: usize = 0;
    let mut in_math = false;
    let mut at_line_start = true;
    let mut i = 0usize;

    while i < n {
        // Fence handling — check once per line start.
        if at_line_start {
            let leading = count_leading_spaces(&bytes[i..]);
            if leading <= 3 {
                let rest = i + leading;
                if let Some((ch, count)) = fence_run(&bytes[rest..]) {
                    if !in_fence {
                        in_fence = true;
                        fence_char = ch;
                        fence_len = count;
                        // skip the entire fence line
                        i = advance_to_eol(bytes, rest + count);
                        at_line_start = false;
                        continue;
                    } else if ch == fence_char && count >= fence_len {
                        // must be whitespace-only after
                        let after = rest + count;
                        if bytes[after..advance_to_eol(bytes, after)]
                            .iter()
                            .all(|b| *b == b' ' || *b == b'\t')
                        {
                            in_fence = false;
                            i = advance_to_eol(bytes, after);
                            at_line_start = false;
                            continue;
                        }
                    }
                }
            }
            at_line_start = false;
        }

        let c = bytes[i];

        if c == b'\n' {
            at_line_start = true;
            i += 1;
            continue;
        }

        if in_fence {
            i += 1;
            continue;
        }

        // `$` toggles math state. Handle $$ as a single toggle.
        if c == b'$' {
            let is_escaped = i > 0 && bytes[i - 1] == b'\\';
            if !is_escaped {
                in_math = !in_math;
            }
            if i + 1 < n && bytes[i + 1] == b'$' {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if in_math {
            i += 1;
            continue;
        }

        // Bare LaTeX detection.
        if c == b'\\'
            && let Some(len) = latex_token_len(bytes, i)
        {
            ranges.push((i, i + len));
            i += len;
            continue;
        }

        i += 1;
    }

    ranges
}

fn count_leading_spaces(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|&&b| b == b' ').count()
}

fn fence_run(bytes: &[u8]) -> Option<(u8, usize)> {
    let first = *bytes.first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let count = bytes.iter().take_while(|&&b| b == first).count();
    if count >= 3 { Some((first, count)) } else { None }
}

fn advance_to_eol(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| from + p)
        .unwrap_or(bytes.len())
}

/// Return the byte length of a complete LaTeX token starting at `bytes[pos]`
/// (which must be `\`). Returns `None` if the token is absent, malformed, or
/// appears truncated by streaming.
fn latex_token_len(bytes: &[u8], pos: usize) -> Option<usize> {
    debug_assert_eq!(bytes[pos], b'\\');
    let n = bytes.len();
    let mut p = pos + 1;
    if p >= n {
        return None;
    }

    // Command name: one or more ASCII letters.
    let cmd_start = p;
    while p < n && (bytes[p] as char).is_ascii_alphabetic() {
        p += 1;
    }
    let cmd_len = p - cmd_start;
    if cmd_len == 0 {
        return None;
    }

    // If command name butts up against EOF without any argument group and no
    // terminator, assume streaming is mid-word and bail.
    if p == n {
        return None;
    }

    // Optional argument groups: {…} or […]
    let mut had_group = false;
    loop {
        if p >= n {
            break;
        }
        let next = bytes[p];
        if next == b'{' {
            match find_matching_brace(bytes, p, b'{', b'}') {
                Some(end) => {
                    had_group = true;
                    p = end + 1;
                }
                None => return None, // unclosed — wait for more streaming data
            }
        } else if next == b'[' {
            match find_matching_brace(bytes, p, b'[', b']') {
                Some(end) => {
                    had_group = true;
                    p = end + 1;
                }
                None => return None,
            }
        } else {
            break;
        }
    }

    // The terminator must not glue the command onto the following text in a
    // way that would make `$…$` visually wrong. We accept EOF only if the
    // command had at least one `{…}` group (that's the signal this really is
    // LaTeX — a bare `\text` at EOF is probably mid-stream).
    if p == n && !had_group {
        return None;
    }

    Some(p - pos)
}

fn find_matching_brace(bytes: &[u8], open_pos: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut p = open_pos;
    while p < bytes.len() {
        let c = bytes[p];
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(p);
            }
        }
        p += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> String {
        wrap_bare_latex(s).into_owned()
    }

    // Basic detection ---------------------------------------------------------

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(&*wrap_bare_latex("hello world"), "hello world");
    }

    #[test]
    fn single_command_with_braces() {
        assert_eq!(w("\\mathbb{Z}"), "$\\mathbb{Z}$");
    }

    #[test]
    fn command_with_multiple_groups() {
        assert_eq!(w("\\frac{1}{2}"), "$\\frac{1}{2}$");
    }

    #[test]
    fn command_with_optional_arg() {
        assert_eq!(w("\\sqrt[3]{8}"), "$\\sqrt[3]{8}$");
    }

    #[test]
    fn multiple_separated_commands() {
        // Trailing `.` acts as a terminator so the streaming-safe wrapper
        // knows the final command is complete.
        assert_eq!(
            w("\\alpha + \\beta."),
            "$\\alpha$ + $\\beta$.",
        );
    }

    #[test]
    fn bare_command_at_eof_without_args_left_alone_streaming_safe() {
        // Streaming-safety promise: the final `\beta` has no terminator yet,
        // so it stays bare until the next chunk proves it's complete. The
        // first `\alpha` is followed by a space, so it wraps.
        assert_eq!(
            &*wrap_bare_latex("\\alpha + \\beta"),
            "$\\alpha$ + \\beta",
        );
    }

    #[test]
    fn command_inside_prose() {
        assert_eq!(
            w("因此 \\frac{a}{b} 成立"),
            "因此 $\\frac{a}{b}$ 成立",
        );
    }

    #[test]
    fn nested_braces_balance() {
        assert_eq!(
            w("\\text{outer {inner} rest}"),
            "$\\text{outer {inner} rest}$",
        );
    }

    // Skip cases --------------------------------------------------------------

    #[test]
    fn already_inline_math_passes_through() {
        assert_eq!(&*wrap_bare_latex("$a^2$"), "$a^2$");
    }

    #[test]
    fn already_block_math_passes_through() {
        assert_eq!(&*wrap_bare_latex("$$\\frac{1}{2}$$"), "$$\\frac{1}{2}$$");
    }

    #[test]
    fn command_inside_inline_math_not_rewrapped() {
        assert_eq!(&*wrap_bare_latex("$\\mathbb{Z}$"), "$\\mathbb{Z}$");
    }

    #[test]
    fn mixed_wrapped_and_bare() {
        assert_eq!(
            w("$\\alpha$ and \\beta."),
            "$\\alpha$ and $\\beta$.",
        );
    }

    #[test]
    fn code_fence_bare_command_untouched() {
        let s = "```\n\\frac{1}{2}\n```";
        assert_eq!(&*wrap_bare_latex(s), s);
    }

    #[test]
    fn code_fence_followed_by_bare_command() {
        let s = "```\ncode\n```\nafter \\alpha.";
        assert_eq!(w(s), "```\ncode\n```\nafter $\\alpha$.");
    }

    // Streaming safety --------------------------------------------------------

    #[test]
    fn incomplete_command_at_eof_left_alone() {
        // Streaming could still extend "fra" to "frac".
        assert_eq!(&*wrap_bare_latex("pre \\fra"), "pre \\fra");
    }

    #[test]
    fn unclosed_brace_left_alone() {
        assert_eq!(
            &*wrap_bare_latex("\\frac{1}{2"),
            "\\frac{1}{2",
        );
    }

    #[test]
    fn command_with_group_at_eof_wrapped() {
        // Having at least one completed {…} is the signal it's really LaTeX.
        assert_eq!(w("\\mathbb{Z}"), "$\\mathbb{Z}$");
    }

    #[test]
    fn progressive_streaming_becomes_wrapped_when_group_closes() {
        let partials = [
            "\\fra",                // bail
            "\\frac",               // still bare, no group
            "\\frac{",              // unclosed
            "\\frac{1",             // unclosed
            "\\frac{1}",            // one group closed, wrap
            "\\frac{1}{",           // second group unclosed
            "\\frac{1}{2",
            "\\frac{1}{2}",         // both groups closed
        ];
        let expected = [
            "\\fra",
            "\\frac",
            "\\frac{",
            "\\frac{1",
            "$\\frac{1}$",
            "\\frac{1}{",
            "\\frac{1}{2",
            "$\\frac{1}{2}$",
        ];
        for (p, e) in partials.iter().zip(expected.iter()) {
            assert_eq!(&*wrap_bare_latex(p), *e, "partial: {p:?}");
        }
    }

    // Unicode / edge cases ----------------------------------------------------

    #[test]
    fn chinese_before_command() {
        assert_eq!(
            w("设 \\mathbb{Z} 为整数集"),
            "设 $\\mathbb{Z}$ 为整数集",
        );
    }

    #[test]
    fn escaped_dollar_does_not_open_math() {
        // `\$` is an escaped literal dollar; math state should not flip.
        assert_eq!(
            w("price \\$ then \\frac{1}{2}"),
            "price \\$ then $\\frac{1}{2}$",
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(&*wrap_bare_latex(""), "");
    }

    #[test]
    fn no_allocation_when_nothing_to_wrap() {
        use std::borrow::Cow;
        match wrap_bare_latex("nothing here") {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("should not allocate when no wrap needed"),
        }
    }
}
