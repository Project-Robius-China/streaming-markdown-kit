spec: task
name: "M3 (deferred) — Tarnawski-style event-stream parser"
tags: [parser, streaming, m3, roadmap, deferred]
estimate: 3d
---

## Intent

**Status: roadmap — not yet started, but expected to ship.** This spec
was originally drafted as M1, then revised downward in sequence after
deciding that the dominant LLM instability mode (unclosed markdown
constructs — "misinterpretation flicker") is better addressed by a
cheaper, stateless preprocessor. That preprocessor is M1
(`specs/m1-remend-rs.spec.md`).

M1 + M2 (commit/tail double-buffer) eliminate misinterpretation flicker
and visual tearing. They do **not** eliminate **re-render flicker** —
the downstream Makepad Markdown widget still re-parses the whole buffer
every chunk and rebuilds the widget tree. That is cheap enough in a
browser (DOM diff, layer caching, CSS GPU paint) but expensive in
Makepad specifically because:

1. **Syntax highlighting re-runs per chunk.** A 500-line Rust fenced
   block gets re-tokenised and re-coloured on every streaming token. An
   event-stream parser's `EnterBlock(FencedCode { info, … })` →
   `ExitBlock` boundary is the natural cache key for "highlight once
   when the block closes".
2. **Mermaid layout re-runs per chunk.** rusty-mermaid's layout is not
   cheap; running it every frame during streaming is wasted work.
   `ExitBlock` on a mermaid fence is the natural "layout now" trigger.
3. **LaTeX (KaTeX-equivalent) re-compiles per chunk** with the same
   argument.
4. **Makepad widget lifecycle.** Widgets keep state via live IDs.
   Full-buffer `set_text` forces identity churn; an append-only event
   stream lets us say "this is the same CodeBlock widget, just append
   more text into its accumulator", preserving scroll, selection, and
   highlight caches.

Therefore: **M3 is likely required**, not optional, in the Makepad
context. The trigger to start M3 is empirical: once M1 + M2 are in
production, measure whether per-chunk re-rendering of fenced code /
mermaid / latex is acceptable on target hardware. If not, M3 is the
answer. This spec remains the roadmap stub until that measurement is
in hand.

Open design gaps that must be resolved before this spec can be
implemented: `issues/003-makepad-widget-identity-and-lifecycle-in-streaming.md`
(widget identity allocation, selection preservation, syntax-highlight
caching across streaming boundaries).

**Decisions pending revision before this spec is activated** (surfaced in
earlier review but deliberately left unresolved — they depend on M1
corpus evidence and Makepad-side measurements not yet in hand):

- `OuterWrapperFence` as a `BlockKind` variant is aichat-domain leakage
  into a general parser. Should become a parser `Config { absorb_outer_markdown_fence: bool }`
  flag, default off, rather than an enum variant every downstream has
  to know about.
- Cursor sentinel `U+258B` is hard-coded. Should become
  `Config { cursor_sentinel: Option<char> }` so renderers with other
  tail-glyph conventions aren't forced to patch the parser.
- `BlockKind` M1 scope omits `BlockQuote`. DeepSeek R1 / OpenAI o1
  thinking-block output uses `> …` extensively; M3 must include
  `BlockQuote` from day one or it ships broken for reasoning-model
  outputs.

Original intent: add an internal, append-only streaming parser to
`streaming-markdown-kit` that turns an incrementally-fed `&str` chunk
sequence into a stream of block-level `RenderEvent`s (enter / text /
exit). Context:
issues/002-streaming-render-state-machine-and-render-gap-audit.md Part B.

## Decisions

> **NOTE (2026-04-19 pivot)**: The Decisions below reflect the pre-pivot M1 draft
> and are preserved verbatim for historical context. They will be revised to
> address the items listed in Intent § "Decisions pending revision"
> (`OuterWrapperFence` → `Config::absorb_outer_markdown_fence`; cursor sentinel →
> `Config::cursor_sentinel: Option<char>`; `BlockKind` must include `BlockQuote`
> day one) before this spec is activated for implementation. **In case of
> conflict, Intent takes precedence over Decisions.** Correspondingly,
> "GFM tables, blockquotes, thematic breaks — deferred" in Out of Scope is
> wrong for `BlockQuote` specifically — that item is day-one scope per Intent.

- Crate-internal API: `pub(crate) struct StreamingParser` with `feed<F: FnMut(RenderEvent<'_>)>(&mut self, chunk: &str, emit: F)` and `finish<F: FnMut(RenderEvent<'_>)>(&mut self, emit: F)`. Crate-external visibility is deferred to M2.
- Event enum in M1: `EnterBlock(BlockKind)`, `ExitBlock`, `Text(&str)`, `HardBreak`, `Cursor`. Inline events are M2.
- `BlockKind` covers M1 scope only: `Paragraph`, `Heading { level: u8 }`, `FencedCode { info: String, backtick_count: u8 }`, `BulletListItem`, `OrderedListItem { start: u32 }`, `OuterWrapperFence` (synthetic marker, never reaches renderer).
- Char-boundary safety: the parser operates on `&str` chunks only; internally uses `chars()` / `char_indices()`. No `s.as_bytes()[i]` indexing anywhere in the state machine.
- Streaming buffer: parser owns a `String` tail-buffer for incomplete UTF-8 scalar sequences and incomplete line-start tokens (fence openers, ATX-heading `#` runs). Tail is drained on next `feed` call or on `finish()`.
- Cursor handling: the `▋` scalar (`DEFAULT_TAIL`) is recognised as an out-of-band cursor glyph and emitted as `RenderEvent::Cursor` — never as part of `Text` — regardless of block context (including inside open fenced code).
- Outer markdown fence detection: when the first non-whitespace tokens of the stream are a fence opener whose info-string equals `markdown` or `md` (any backtick count ≥ 3), the parser enters `OuterWrapperFence` mode — it does NOT emit `EnterBlock(FencedCode)` for that opener, and the matching closer (same backtick count or greater) is consumed silently. Inner blocks emit normally.
- Parity harness: a new test file `tests/parser_parity.rs` that, for every fixture currently exercised against the string-based transform chain, drives the new parser and reduces events back to a string via a minimal test-only `events_to_string` reducer, then asserts equality modulo the trailing `▋`.
- No new crate dependencies. Only existing deps (`streaming_markdown_sanitizer`, and std).
- No changes to existing public API (`streaming_display`, `streaming_display_with_latex_autowrap`, `wrap_bare_latex`, `inline_code_options`) in M1.

## Boundaries

### Allowed Changes

- `src/streaming_parser/**`
- `src/lib.rs` (module declaration only — `mod streaming_parser;`)
- `tests/parser_parity.rs`
- `tests/parser_unit.rs`
- `Cargo.toml` (only if a dev-dep is needed; production deps must not change)

### Forbidden

- Do not change the signature or behaviour of `streaming_display`, `streaming_display_default`, `streaming_display_with_latex_autowrap`, `wrap_bare_latex`, or `inline_code_options`.
- Do not add any new production dependency to `Cargo.toml` `[dependencies]`.
- Do not use byte-indexed string slicing (`s[i..]`, `s.as_bytes()[i]`) inside the state machine. Use `chars()` / `char_indices()` / `strip_prefix` / `strip_suffix`.
- Do not use `.unwrap()` or `.expect()` on non-infallible operations in the state machine. Parser errors surface via event stream (malformed input recovers by emitting `Text` for the unrecognised span — never panics).
- Do not expose `StreamingParser` or `RenderEvent` as `pub` from `lib.rs` in M1. They are `pub(crate)` / test-visible only.
- Do not emit inline-level events (em, strong, inline-code, link, image) — those are M2.

## Out of Scope

- Inline-level parsing (em, strong, inline code, links, images) — M2.
- Makepad-side `consume_event` consumer widget — M2.
- Retiring `unwrap_outer_markdown_fence`, `wrap_bare_latex`, or the `▋` filter in aichat — M3.
- GFM tables, blockquotes, thematic breaks — deferred (not in M1 `BlockKind`).
- Backpressure / event coalescing — M2 concern.
- Performance benchmarks vs the string transform chain — tracked separately; M1 only requires parity, not speed.

## Completion Criteria

Scenario: Plain paragraph feed emits enter / text / exit
  Test: test_paragraph_single_chunk_emits_block_triple
  Given a fresh StreamingParser
  When the caller feeds the chunk "Hello world\n\n"
  Then the event sequence is:
    | index | event                   |
    | 0     | EnterBlock(Paragraph)   |
    | 1     | Text("Hello world")     |
    | 2     | ExitBlock               |

Scenario: Fenced code block with info-string emits FencedCode with preserved info
  Test: test_fenced_code_preserves_info_string
  Given a fresh StreamingParser
  When the caller feeds "```rust\nfn main() {}\n```\n"
  Then the first event is EnterBlock(FencedCode) with info "rust" and backtick_count 3
  And a Text event delivers the line "fn main() {}"
  And the last event is ExitBlock

Scenario: ATX heading emits Heading with correct level
  Test: test_atx_heading_level_preserved
  Given a fresh StreamingParser
  When the caller feeds "### Design notes\n\n"
  Then the first event is EnterBlock(Heading) with level 3
  And a Text event delivers "Design notes"
  And the final event is ExitBlock

Scenario: Fence opener split across two feeds does not emit premature events
  Test: test_split_fence_opener_defers_emission
  Given a fresh StreamingParser
  When the caller feeds "``"
  Then no events are emitted yet
  When the caller feeds "`python\nx = 1\n```\n"
  Then the first event is EnterBlock(FencedCode) with info "python" and backtick_count 3
  And a Text event delivers "x = 1"
  And the last event is ExitBlock

Scenario: Unterminated fenced code at end of stream keeps block open
  Test: test_unterminated_fence_has_no_phantom_exit
  Given a fresh StreamingParser
  When the caller feeds "```rust\nfn ma"
  And the caller calls finish()
  Then the event sequence starts with EnterBlock(FencedCode) with info "rust"
  And a Text event delivers "fn ma"
  And no ExitBlock event follows before finish() returns

Scenario: UTF-8 scalar split across two feeds does not panic
  Test: test_utf8_scalar_split_across_feeds_is_buffered
  Given a fresh StreamingParser
  When the caller feeds a chunk ending with the first 2 bytes of the UTF-8 sequence for "中"
  Then the feed call returns without panic
  And no Text event containing a replacement or mojibake byte sequence is emitted
  When the caller feeds the remaining 1 byte of "中" followed by "\n\n"
  Then a Text event delivering exactly "中" is emitted
  And ExitBlock follows

Scenario: Outer markdown wrapper fence is absorbed and inner blocks emit normally
  Test: test_outer_markdown_wrapper_is_absorbed
  Given a fresh StreamingParser
  When the caller feeds "````markdown\n# Inner heading\n\n```rust\nfn x() {}\n```\n````\n"
  Then no EnterBlock(FencedCode) event is emitted for the outer 4-backtick fence
  And an EnterBlock(Heading) event with level 1 is emitted for "Inner heading"
  And a subsequent EnterBlock(FencedCode) event is emitted for the inner 3-backtick rust block
  And the inner block's ExitBlock precedes the end of the stream

Scenario: Cursor glyph "▋" is emitted via Cursor channel, not Text
  Test: test_cursor_glyph_routes_to_cursor_event
  Given a fresh StreamingParser
  When the caller feeds "hello ▋ world\n\n"
  Then the Text events together deliver "hello  world" with exactly one space between "hello" and "world"
  And exactly one Cursor event is emitted
  And no Text event's payload contains the scalar U+258B

Scenario: Cursor glyph inside an open fenced code block still routes to Cursor
  Test: test_cursor_glyph_inside_fenced_code_routes_to_cursor
  Given a fresh StreamingParser
  When the caller feeds "```\ncode▋here\n```\n"
  Then exactly one Cursor event is emitted
  And no Text event contains the scalar U+258B
  And Text events together deliver "codehere" within the fenced block

Scenario: Parser events reduce to the same string as the existing pipeline (parity)
  Test:
    Package: streaming-markdown-kit
    Filter: parser_parity::parity_matches_streaming_display_for_corpus
  Given the fixture corpus used by the existing streaming_display tests
  When each fixture is fed to StreamingParser and the emitted events are reduced with the test-only events_to_string reducer
  Then for every fixture the reduced string equals the output of streaming_display_with_latex_autowrap with the same SanitizeOptions, modulo the trailing cursor glyph
