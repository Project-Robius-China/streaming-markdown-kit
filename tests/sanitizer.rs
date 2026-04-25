use streaming_markdown_kit::sanitize_streaming_markdown as san;

// --- baseline: nothing to trim -----------------------------------------------

#[test]
fn empty_input() {
    assert_eq!(&*san(""), "");
}

#[test]
fn plain_prose_passthrough() {
    let s = "Hello, this is just text.\n\nAnother paragraph.";
    assert_eq!(&*san(s), s);
}

#[test]
fn complete_doc_with_various_structures() {
    let s = "# Title\n\nPara with **bold** and *italic*.\n\n\
             ```rust\nfn main() {}\n```\n\n\
             $$E = mc^2$$\n\n\
             | a | b |\n| - | - |\n| 1 | 2 |\n";
    assert_eq!(&*san(s), s);
}

// --- fenced code block -------------------------------------------------------

#[test]
fn unclosed_backtick_fence_trims_to_fence_start() {
    let s = "Hello\n```rust\nfn main() {";
    assert_eq!(&*san(s), "Hello\n");
}

#[test]
fn unclosed_tilde_fence_also_trimmed() {
    let s = "Intro\n~~~python\nprint(";
    assert_eq!(&*san(s), "Intro\n");
}

#[test]
fn closed_fence_preserved() {
    let s = "```\ncode\n```\n";
    assert_eq!(&*san(s), s);
}

#[test]
fn fence_closes_with_longer_marker() {
    // Opening ``` can be closed by ```` (longer is OK).
    let s = "```\ncode\n````\n";
    assert_eq!(&*san(s), s);
}

#[test]
fn fence_does_not_close_with_shorter_marker() {
    // Opening ```` (4 backticks) cannot close with ``` (3). So this is unclosed.
    let s = "````\ncode\n```\nstill inside";
    assert_eq!(&*san(s), "");
}

#[test]
fn fence_does_not_close_with_different_char() {
    let s = "```\ncode\n~~~\n";
    assert_eq!(&*san(s), "");
}

#[test]
fn backticks_inside_tilde_fence_ignored() {
    // A tilde fence contains literal backticks; they must not be mistaken for
    // an inner fence.
    let s = "~~~\n```inside```\n~~~\n";
    assert_eq!(&*san(s), s);
}

#[test]
fn closing_fence_must_be_whitespace_after() {
    // "``` extra" is not a valid closing fence per CommonMark — still open.
    let s = "```\ncode\n``` extra trailing text\nstill inside";
    assert_eq!(&*san(s), "");
}

#[test]
fn fence_ignores_4_space_indent() {
    // "    ```" is indented code, not a fence.
    let s = "Text\n    ```\n    not a fence\n";
    assert_eq!(&*san(s), s);
}

// --- block math --------------------------------------------------------------

#[test]
fn paired_block_math_preserved() {
    let s = "Before\n$$\nx = y\n$$\nAfter";
    assert_eq!(&*san(s), s);
}

#[test]
fn unpaired_block_math_trimmed() {
    let s = "Before\n$$\nx = ";
    assert_eq!(&*san(s), "Before\n");
}

#[test]
fn inline_block_math_on_one_line_paired() {
    let s = "See $$a + b$$ here.";
    assert_eq!(&*san(s), s);
}

#[test]
fn dollar_in_code_block_ignored() {
    // Inside a closed code fence, $$ must not count.
    let s = "```\nlet price = $$10;\n```\nmore";
    assert_eq!(&*san(s), s);
}

#[test]
fn escaped_dollar_not_counted() {
    // \$$ is an escaped dollar followed by a single $, not a block-math delimiter.
    let s = "Price: \\$$5.00 each.";
    assert_eq!(&*san(s), s);
}

#[test]
fn single_dollars_not_treated_as_block_math() {
    // Inline math $...$ is harmless even if unpaired; we only guard $$.
    let s = "Cost is $5 today, $$10 tomorrow ";
    // One `$$` = unpaired → trim.
    let out = san(s);
    assert!(out.len() < s.len(), "should trim unpaired $$");
    assert!(!out.contains("$$"));
}

// --- incomplete table --------------------------------------------------------

#[test]
fn table_header_only_trimmed() {
    let s = "Intro\n\n| Col1 | Col2 |";
    assert_eq!(&*san(s), "Intro\n\n");
}

#[test]
fn table_header_plus_separator_only_trimmed() {
    let s = "Intro\n\n| a | b |\n| --- | --- |";
    assert_eq!(&*san(s), "Intro\n\n");
}

#[test]
fn table_header_plus_partial_separator_trimmed() {
    let s = "Intro\n\n| a | b |\n| --";
    assert_eq!(&*san(s), "Intro\n\n");
}

#[test]
fn complete_table_preserved() {
    let s = "Intro\n\n| a | b |\n| --- | --- |\n| 1 | 2 |";
    assert_eq!(&*san(s), s);
}

#[test]
fn pipe_text_without_separator_not_a_table() {
    // "| foo | bar |" followed by ordinary prose is just text with pipes.
    let s = "Intro\n\n| foo | bar |\nAnd some prose follows.";
    assert_eq!(&*san(s), s);
}

#[test]
fn table_at_document_start() {
    let s = "| a | b |\n| --- | --- |";
    assert_eq!(&*san(s), "");
}

// --- regression: table detection must respect fence state --------------------

#[test]
fn pipe_line_inside_unclosed_fence_is_not_a_table() {
    // Regression for "mid-stream content disappears during long Rust code
    // block" (P2 test, 2026-04-19). Before the fix, the \n\n inside the
    // fenced block + the `|…|` match arm line on the tail triggered the
    // incomplete-table detector, which trimmed the stream back to the \n\n
    // inside the fence — visibly making the code content "disappear" until
    // the next chunk arrived. After the fix, fence-aware table detection
    // skips this case entirely.
    //
    // Note: trim_unclosed_fence is left at its default (true) here because
    // this test is specifically about the table code path. In aichat it's
    // set to false, so the fence content would pass through.
    let opts = streaming_markdown_kit::SanitizeOptions {
        trim_unclosed_fence: false,
        ..Default::default()
    };
    let s = "```rust\nfn f(x: Option<i32>) {\n\n    match x { | Some(a) | Some(b) => {} }\n    println!(\"| {} | {} |\", 1, 2);";
    let out = streaming_markdown_kit::sanitize_streaming_markdown_with(s, opts);
    assert_eq!(
        &*out, s,
        "content inside an unclosed fence must not be trimmed by table detection"
    );
}

#[test]
fn pipe_line_inside_closed_fence_followed_by_real_table() {
    // If a closed fenced block contains pipe lines AND after the fence a real
    // incomplete table appears, trimming should only target the real table.
    let s = "```rust\nlet r = |x| x + 1;\n```\n\n| a | b |\n| -";
    let out = san(s);
    // The closed fence is preserved; the incomplete table after it is trimmed.
    // Trim position is `rfind("\n\n") + 2`, so the boundary `\n\n` is kept.
    assert_eq!(&*out, "```rust\nlet r = |x| x + 1;\n```\n\n");
}

// --- combined / priority -----------------------------------------------------

#[test]
fn fence_wins_over_later_math() {
    // Unclosed fence at byte 6, but the $$ after it is "inside the fence" and
    // should be ignored; result is still "trim at fence start".
    let s = "Hi\n\n```\nfoo $$ bar";
    assert_eq!(&*san(s), "Hi\n\n");
}

#[test]
fn earliest_cut_wins_math_before_table() {
    // $$ at byte 6, table at byte 20. Trim at byte 6.
    let s = "prose\n$$\nstuff...\n\n| a | b |\n| - |";
    let expected = "prose\n";
    assert_eq!(&*san(s), expected);
}

// --- Unicode / UTF-8 ---------------------------------------------------------

#[test]
fn unicode_content_preserved() {
    let s = "你好，世界 🌍\n\n代码：\n```\n中文\n```\n";
    assert_eq!(&*san(s), s);
}

#[test]
fn unicode_before_unclosed_fence_trims_at_fence() {
    let s = "你好世界\n```rust\nfn 中文(";
    let out = san(s);
    assert_eq!(&*out, "你好世界\n");
    // Make sure we cut on a valid UTF-8 boundary.
    assert!(std::str::from_utf8(out.as_bytes()).is_ok());
}

#[test]
fn emoji_inside_math_preserved_when_closed() {
    let s = "Equation: $$🎉 = \\pi$$ done";
    assert_eq!(&*san(s), s);
}

// --- Borrow vs allocation ----------------------------------------------------

#[test]
fn returns_borrowed_always() {
    use std::borrow::Cow;
    let s = "Hello\n```\nunclosed";
    match san(s) {
        Cow::Borrowed(_) => {}
        Cow::Owned(_) => panic!("sanitizer must never allocate"),
    }
}

// --- Streaming progression (the real use case) -------------------------------

#[test]
fn simulate_streaming_chunks() {
    // As each chunk arrives, sanitized output should grow monotonically when
    // dangling structures resolve.
    let full = "Hello world.\n\n```rust\nfn main() {}\n```\n\nDone.";
    let mut last_len = 0usize;
    let mut saw_shrink = 0;
    for end in (1..=full.len()).step_by(3) {
        if !full.is_char_boundary(end) {
            continue;
        }
        let partial = &full[..end];
        let out = san(partial);
        // Sanitized output can shrink (e.g. when a fence opens mid-way) but
        // must never exceed the input length.
        assert!(out.len() <= partial.len());
        if out.len() < last_len {
            saw_shrink += 1;
        }
        last_len = out.len();
    }
    // We expect at least one shrink: when the opening ``` arrives, the tail
    // is dropped until the closing ``` does.
    assert!(
        saw_shrink >= 1,
        "expected to see sanitizer trim when fence opens mid-stream"
    );
}

#[test]
fn final_complete_input_never_trimmed() {
    let full = "Hello world.\n\n```rust\nfn main() {}\n```\n\n$$x=1$$\n\n| a | b |\n| - | - |\n| 1 | 2 |\n";
    assert_eq!(&*san(full), full);
}
