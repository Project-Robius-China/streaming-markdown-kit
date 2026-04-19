//! Streaming-markdown helpers for Makepad 2.0 apps.
//!
//! This crate contains three things you need to render LLM output live in a
//! Makepad `Markdown` widget without flicker:
//!
//! 1. A [`sanitizer`] module (structural trimming of half-finished code fences,
//!    `$$` pairs, and tables) — originally a separate crate, now bundled here.
//! 2. The [`streaming_display`] helper that formats one frame's body: sanitise,
//!    then append a cursor glyph that rides Makepad's built-in `animated_chars`
//!    fade window.
//! 3. Bundled fonts under `resources/` (LXGW Wen Kai for CJK, Liberation Mono
//!    for Latin monospace). Referenced from Makepad DSL via
//!    `crate_resource("streaming_markdown_kit:resources/...")`.
//!
//! See this crate's README for the 10-line DSL snippet to paste into your
//! app's `script_mod!` block.

#![warn(clippy::all)]

pub mod latex_wrap;
#[cfg(feature = "mermaid")]
pub mod mermaid;
pub mod remend;
pub mod sanitizer;

pub use latex_wrap::wrap_bare_latex;
#[cfg(feature = "mermaid")]
pub use mermaid::{RenderedMermaid, render_mermaid_to_png, render_mermaid_to_svg};
pub use remend::remend;
pub use sanitizer::{
    SanitizeOptions, sanitize_streaming_markdown, sanitize_streaming_markdown_with,
};

/// The default streaming cursor glyph. Appended to the sanitized body so it
/// rides the tail of the `animated_chars` fade window — producing a naturally
/// blinking cursor effect for free.
pub const DEFAULT_TAIL: char = '▋';

/// Build the body string to feed `markdown.set_text(cx, ...)` on each streaming
/// update.
///
/// Pipeline:
/// 1. Sanitize `raw` with the given [`SanitizeOptions`] (trim half-written
///    structures so the downstream renderer's layout doesn't jump).
/// 2. Append [`DEFAULT_TAIL`] so the last character is always inside Makepad's
///    fade-in window while streaming is active.
///
/// Call [`sanitizer::sanitize_streaming_markdown`] directly if you want to
/// skip the cursor glyph.
#[must_use]
pub fn streaming_display(raw: &str, opts: SanitizeOptions) -> String {
    let safe = sanitize_streaming_markdown_with(raw, opts);
    let mut out = String::with_capacity(safe.len() + DEFAULT_TAIL.len_utf8());
    out.push_str(&safe);
    out.push(DEFAULT_TAIL);
    out
}

/// Same as [`streaming_display`] but with [`SanitizeOptions::default`].
#[must_use]
pub fn streaming_display_default(raw: &str) -> String {
    streaming_display(raw, SanitizeOptions::default())
}

/// Like [`streaming_display_with_latex_autowrap`] but runs [`remend::remend`]
/// between the sanitizer and the cursor append. The extra step detects
/// markdown constructs left syntactically open at the tail of the streaming
/// buffer (unclosed `**`, backticks, fenced code, link, math) and appends
/// the speculative closers, so the downstream parser doesn't have to
/// tolerant-recover on every token boundary — which was the root cause of
/// chat-UI flicker with Kimi / DeepSeek / Qwen / Llama output.
#[must_use]
pub fn streaming_display_with_latex_autowrap_remend(raw: &str, opts: SanitizeOptions) -> String {
    let wrapped = wrap_bare_latex(raw);
    let safe = sanitize_streaming_markdown_with(&wrapped, opts);
    let closed = remend::remend(&safe);
    let mut out = String::with_capacity(closed.len() + DEFAULT_TAIL.len_utf8());
    out.push_str(&closed);
    out.push(DEFAULT_TAIL);
    out
}

/// Like [`streaming_display`] but first runs [`wrap_bare_latex`] on the input.
///
/// Use this when your LLM emits LaTeX commands (`\frac{…}{…}`, `\mathbb{Z}`,
/// `\forall`, …) without `$…$` delimiters — common in Kimi, DeepSeek, Qwen,
/// and Llama outputs. The wrap step surrounds each complete command with
/// `$…$` so pulldown-cmark's `ENABLE_MATH` recognises it and the downstream
/// math widget can render it. Incomplete commands mid-stream are left alone
/// until the next chunk completes them.
#[must_use]
pub fn streaming_display_with_latex_autowrap(raw: &str, opts: SanitizeOptions) -> String {
    let wrapped = wrap_bare_latex(raw);
    let safe = sanitize_streaming_markdown_with(&wrapped, opts);
    let mut out = String::with_capacity(safe.len() + DEFAULT_TAIL.len_utf8());
    out.push_str(&safe);
    out.push(DEFAULT_TAIL);
    out
}

/// The recommended options for renderers where code blocks are drawn **inline**
/// as plain monospaced text (Makepad `Markdown` with `use_code_block_widget:
/// false`). Leaves fenced code visible while streaming so each character fades
/// in naturally; still trims half-`$$` math and partial tables.
#[must_use]
pub fn inline_code_options() -> SanitizeOptions {
    SanitizeOptions {
        trim_unclosed_fence: false,
        ..SanitizeOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_display_appends_cursor() {
        let out = streaming_display_default("hello");
        assert_eq!(out, "hello▋");
    }

    #[test]
    fn inline_options_keeps_fences() {
        let opts = inline_code_options();
        assert!(!opts.trim_unclosed_fence);
        assert!(opts.trim_unpaired_block_math);
        assert!(opts.trim_incomplete_table);
    }

    #[test]
    fn streaming_display_trims_unpaired_math() {
        let out = streaming_display_default("before\n$$\nhalf");
        assert_eq!(out, "before\n▋");
    }

    #[test]
    fn inline_options_lets_partial_fence_through() {
        let out = streaming_display("Hi\n```rust\nfn ma", inline_code_options());
        assert_eq!(out, "Hi\n```rust\nfn ma▋");
    }
}
