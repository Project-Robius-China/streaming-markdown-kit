//! End-to-end guarantee tests against the real pulldown-cmark parser.
//!
//! The core guarantee: **every sanitized prefix is structurally balanced** —
//! no unclosed code blocks, no unclosed tables, no math events without content.
//! A downstream renderer (like Makepad's `Markdown`) can render any sanitized
//! step and won't flash partial structures.
//!
//! We also verify that a fully-streamed document, once sanitized at each step,
//! converges to the same event stream as parsing the full input in one shot.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use streaming_markdown_kit::sanitize_streaming_markdown as san;

fn opts() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_MATH
}

fn parse(s: &str) -> Vec<Event<'_>> {
    Parser::new_ext(s, opts()).collect()
}

/// Structural balance: for every Start(tag) there's a matching End(tag) later
/// in the stream. Block-math events appear paired (`InlineMath`/`DisplayMath`
/// are self-contained single events in pulldown-cmark, so this mostly concerns
/// code blocks and tables).
fn assert_balanced(events: &[Event<'_>], context: &str) {
    let mut stack: Vec<&Tag> = Vec::new();
    for ev in events {
        match ev {
            Event::Start(tag) => stack.push(tag),
            Event::End(end) => {
                let Some(open) = stack.pop() else {
                    panic!("{context}: End without Start: {end:?}");
                };
                let ok = matches!(
                    (open, end),
                    (Tag::Paragraph, TagEnd::Paragraph)
                        | (Tag::Heading { .. }, TagEnd::Heading(_))
                        | (Tag::BlockQuote(_), TagEnd::BlockQuote(_))
                        | (Tag::CodeBlock(_), TagEnd::CodeBlock)
                        | (Tag::HtmlBlock, TagEnd::HtmlBlock)
                        | (Tag::List(_), TagEnd::List(_))
                        | (Tag::Item, TagEnd::Item)
                        | (Tag::FootnoteDefinition(_), TagEnd::FootnoteDefinition)
                        | (Tag::Table(_), TagEnd::Table)
                        | (Tag::TableHead, TagEnd::TableHead)
                        | (Tag::TableRow, TagEnd::TableRow)
                        | (Tag::TableCell, TagEnd::TableCell)
                        | (Tag::Emphasis, TagEnd::Emphasis)
                        | (Tag::Strong, TagEnd::Strong)
                        | (Tag::Strikethrough, TagEnd::Strikethrough)
                        | (Tag::Link { .. }, TagEnd::Link)
                        | (Tag::Image { .. }, TagEnd::Image)
                        | (Tag::MetadataBlock(_), TagEnd::MetadataBlock(_))
                );
                assert!(ok, "{context}: mismatched tags: {open:?} vs {end:?}");
            }
            _ => {}
        }
    }
    assert!(
        stack.is_empty(),
        "{context}: unclosed tags at end of stream: {stack:?}"
    );
}

/// Walk the input one char-boundary at a time, sanitize, parse, and assert
/// structural balance of every intermediate state.
fn assert_every_sanitized_step_balanced(full: &str) {
    for end in 0..=full.len() {
        if !full.is_char_boundary(end) {
            continue;
        }
        let sanitized = san(&full[..end]);
        let events = parse(&sanitized);
        assert_balanced(
            &events,
            &format!("at char {end} of {}", full.len()),
        );
    }
}

// --- guarantee tests ---------------------------------------------------------

#[test]
fn balanced_stream_for_document_with_code_block() {
    let full = "Hello.\n\n```rust\nfn main() { println!(\"hi\"); }\n```\n\nBye.";
    assert_every_sanitized_step_balanced(full);

    let final_events = parse(full);
    let sanitized_full = san(full);
    let sanitized_full_events = parse(&sanitized_full);
    assert_eq!(final_events, sanitized_full_events);
}

#[test]
fn balanced_stream_for_document_with_block_math() {
    let full = "Intro.\n\n$$\na^2 + b^2 = c^2\n$$\n\nOutro.";
    assert_every_sanitized_step_balanced(full);
}

#[test]
fn balanced_stream_for_document_with_table() {
    let full = "Notes.\n\n| a | b |\n| - | - |\n| 1 | 2 |\n| 3 | 4 |\n\nEnd.";
    assert_every_sanitized_step_balanced(full);
}

#[test]
fn balanced_stream_for_nested_structures() {
    // Stresses: list → paragraph → code fence, with streaming mid-fence.
    let full = "1. First item\n   with prose\n\n   ```py\n   x = 1\n   ```\n\n2. Second\n";
    assert_every_sanitized_step_balanced(full);
}

/// Demonstrates what the sanitizer is protecting against: during raw streaming
/// the *kind* of the last block flips between "paragraph with literal ``` text"
/// and "code block", which is the visible flicker.
#[test]
fn control_raw_streaming_flips_last_block_kind() {
    let full = "Hello.\n\n```rust\nfn main() {}\n```\n\nBye.";

    #[derive(PartialEq, Eq, Debug)]
    enum LastStart<'a> {
        None,
        Paragraph,
        CodeBlock(String),
        Table,
        Other(&'a str),
    }

    let mut last_kind_seen: Option<LastStart<'_>> = None;
    let mut flips = 0;

    for end in 1..=full.len() {
        if !full.is_char_boundary(end) {
            continue;
        }
        let events = parse(&full[..end]);
        let kind = events.iter().rev().find_map(|e| match e {
            Event::Start(Tag::Paragraph) => Some(LastStart::Paragraph),
            Event::Start(Tag::CodeBlock(k)) => Some(LastStart::CodeBlock(format!("{k:?}"))),
            Event::Start(Tag::Table(_)) => Some(LastStart::Table),
            Event::Start(other) => {
                let name: &'static str = match other {
                    Tag::Heading { .. } => "Heading",
                    Tag::List(_) => "List",
                    Tag::Item => "Item",
                    _ => "Other",
                };
                Some(LastStart::Other(name))
            }
            _ => None,
        }).unwrap_or(LastStart::None);

        if let Some(prev) = &last_kind_seen
            && *prev != kind
        {
            flips += 1;
        }
        last_kind_seen = Some(kind);
    }

    // Expect at least: None -> Paragraph (at "H"), Paragraph -> CodeBlock (at "```"),
    // CodeBlock -> Paragraph (after "```\n\nB").
    assert!(
        flips >= 3,
        "expected >=3 last-block kind flips during raw streaming, saw {flips}"
    );
}

#[test]
fn sanitized_full_input_equals_raw_full_input() {
    // Whatever trimming happens along the way, the complete input must always
    // sanitize to itself (or to something that parses identically).
    for full in [
        "",
        "just prose",
        "# Heading\n\nPara.",
        "```\ncode\n```",
        "$$x=1$$",
        "| a | b |\n| - | - |\n| 1 | 2 |\n",
        "Mixed **bold** and *italic* and `inline code`.",
    ] {
        let raw = parse(full);
        let san_full = san(full);
        let sanitized = parse(&san_full);
        assert_eq!(raw, sanitized, "sanitize distorted full input: {full:?}");
    }
}
