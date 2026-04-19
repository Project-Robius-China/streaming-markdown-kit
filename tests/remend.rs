//! Integration tests for `remend()`. Each `#[test]` here binds to one
//! scenario in `specs/m1-remend-rs.spec.md` via the scenario's `Test:`
//! selector. Corpus-level properties are tested in `tests/corpus_properties.rs`.

use std::borrow::Cow;
use streaming_markdown_kit::remend;

// --- 1. Unclosed bold at tail gets closing asterisks ---
#[test]
fn test_remend_closes_trailing_bold() {
    let out = remend("Hello **world");
    assert_eq!(out, "Hello **world**");
    assert!(matches!(out, Cow::Owned(_)));
}

// --- 2. Unclosed inline code at tail gets closing backtick ---
#[test]
fn test_remend_closes_trailing_inline_code() {
    let out = remend("see `foo");
    assert_eq!(out, "see `foo`");
    assert!(matches!(out, Cow::Owned(_)));
}

// --- 3. Unclosed fenced code at tail gets closing fence on a new line ---
#[test]
fn test_remend_closes_trailing_fenced_code() {
    let out = remend("```rust\nfn main() {");
    assert_eq!(out, "```rust\nfn main() {\n```");
    assert!(matches!(out, Cow::Owned(_)));
}

// --- 4. Unclosed link at tail gets closing parenthesis ---
#[test]
fn test_remend_closes_trailing_link() {
    let out = remend("see [the docs](https://example.com/path");
    assert_eq!(out, "see [the docs](https://example.com/path)");
}

// --- 5. Already-closed bold is left unchanged ---
#[test]
fn test_remend_idempotent_on_closed_bold() {
    let out = remend("fully **closed** text");
    assert_eq!(out, "fully **closed** text");
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 6. Intraword underscore does not trigger italic synthesis ---
#[test]
fn test_remend_ignores_intraword_underscore() {
    let out = remend("call snake_case_identifier here");
    assert_eq!(out, "call snake_case_identifier here");
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 7. Unclosed emphasis inside a closed fenced code block is not touched ---
#[test]
fn test_remend_does_not_descend_into_closed_fence() {
    let src = "prefix\n```\nthis **is not bold\n```\nok";
    let out = remend(src);
    assert_eq!(out, src);
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 8. Mermaid block content is protected from inline rules ---
#[test]
fn test_remend_protects_mermaid_block_content() {
    let src = "```mermaid\nflowchart LR\n    A[**not bold**] --> B";
    let out = remend(src);
    assert!(
        out.ends_with("\n```"),
        "expected fenced-code closer at end, got {:?}",
        out.as_ref()
    );
    assert!(
        out.contains("A[**not bold**]"),
        "mermaid payload should be passed through unchanged"
    );
    // Rule should only have appended the fence closer, nothing else.
    let expected = format!("{src}\n```");
    assert_eq!(out, expected);
}

// --- 9. Triple emphasis takes priority over double and single ---
#[test]
fn test_remend_triple_emphasis_priority() {
    let out = remend("***all three");
    assert_eq!(out, "***all three***");
    assert!(!out.contains("****"));
}

// --- 10. Content before the last block boundary is not mutated ---
#[test]
fn test_remend_touches_only_tail_after_last_boundary() {
    let src = "closed paragraph **ok**\n\nunclosed **bold";
    let out = remend(src);
    assert_eq!(out, "closed paragraph **ok**\n\nunclosed **bold**");
    // The substring before "\n\nunclosed" must be byte-identical.
    let boundary = src.find("\n\nunclosed").unwrap();
    assert_eq!(&out[..boundary], &src[..boundary]);
}

// --- 11 (deferred to corpus test). Idempotency — remend(remend(x)) equals remend(x) ---
#[test]
fn test_remend_idempotent_on_simple_inputs() {
    for input in &[
        "Hello",
        "Hello **world",
        "see `foo",
        "```rust\nfn main() {",
        "closed **bold** and more",
        "",
        "  ",
    ] {
        let once = remend(input);
        let twice = remend(&once);
        assert_eq!(once, twice, "idempotency failed for input {input:?}");
        assert!(
            matches!(twice, Cow::Borrowed(_)),
            "second application should be Borrowed for input {input:?}"
        );
    }
}

// --- 12. Well-formed input returns Cow::Borrowed for the whole input ---
#[test]
fn test_remend_borrowed_on_wellformed() {
    let src = "Just some well-formed prose.\n\nAnother paragraph with `inline code`.";
    let out = remend(src);
    assert_eq!(out, src);
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 13. Malformed pathological input does not panic and surfaces no error ---
#[test]
fn test_remend_no_panic_on_pathological_input() {
    // Combine several unclosed constructs + CJK + cursor glyph.
    let src = "***三重 **嵌套 `代码 [link](url $math \n```unclosed\n文本▋";
    let out = remend(src);
    // Must return something — either Borrowed or Owned — without panic.
    assert!(out.is_char_boundary(0));
    // UTF-8 validity is guaranteed by Rust's String type; just exercise the path.
    let _ = out.as_bytes();
}

// --- 14. Invalid construct is rejected — underscore flanked by digits is not an italic opener ---
#[test]
fn test_remend_rejects_intradigit_underscore_as_italic() {
    let out = remend("version 1_0_beta");
    assert_eq!(out, "version 1_0_beta");
    assert!(!out.ends_with('_'));
}

// --- 15. Intraword double-asterisk at the tail is not synthesised (remend-tightened) ---
#[test]
fn test_remend_tightened_intraword_bold() {
    let out = remend("foo**bar");
    assert_eq!(out, "foo**bar");
    assert!(!out.ends_with("**"));
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 16. Space-left emphasis at tail closes even when the payload is CJK ---
#[test]
fn test_remend_space_flanked_bold_at_tail_with_cjk_body() {
    let out = remend("文本 **粗体");
    assert_eq!(out, "文本 **粗体**");
    assert!(matches!(out, Cow::Owned(_)));
    assert!(out.contains("粗体"));
}

// --- 17. CJK-flanked intraword double-asterisk is tightened out by remend ---
#[test]
fn test_remend_tightened_cjk_intraword_bold() {
    let out = remend("中文**加粗");
    assert_eq!(out, "中文**加粗");
    assert!(matches!(out, Cow::Borrowed(_)));
}

// --- 18. Partial HTML tag at tail is never closed — regression guard for injection-vector avoidance ---
#[test]
fn test_remend_never_synthesises_html_closer() {
    let src = "partial <img onerror=\"evil()";
    let out = remend(src);
    assert_eq!(out, src);
    assert!(!out.contains("</img>"));
    // The returned length must equal input length (no `>` appended).
    assert_eq!(out.len(), src.len());
}

// --- 20. Block-boundary identification is fence-stack aware ---
#[test]
fn test_remend_prefix_preservation_fence_interior_nn_not_boundary() {
    let src = "prose\n\n```rust\nfn a() {}\n\nfn b() {";
    let out = remend(src);
    assert!(out.ends_with("\n```"), "expected fenced closer, got {out:?}");
    // The byte range [0, len("prose\n\n")) is preserved.
    let prefix_len = "prose\n\n".len();
    assert_eq!(&out[..prefix_len], &src[..prefix_len]);
    // The full input is also byte-identical up to its length (rule only appends).
    assert_eq!(&out[..src.len()], src);
}
