spec: task
name: "M1 — remend-rs: tail closing-synthesis preprocessor"
tags: [streaming, preprocessor, m1, public-api]
estimate: 2d
---

## Intent

Add a pure Rust function `remend(src: &str) -> Cow<'_, str>` that detects
markdown constructs left syntactically open at the tail of a streaming
buffer (unclosed `**`, backticks, fences, links, math, HTML tags, …) and
appends the speculative closers needed to make the result a well-formed
markdown string. The function is a direct Rust port of the `remend` JS
package's trick — a stateless, idempotent string transform applied as a
downstream step after the existing sanitizer but before the downstream
markdown parser. It removes the need for the downstream parser's tolerant
recovery to choose between interpretations on every token boundary, which
is the root cause of chat-UI flicker with Kimi / DeepSeek / Qwen / Llama
output. Context:
issues/002-streaming-render-state-machine-and-render-gap-audit.md Part B
and the design-discussion pivot recorded there.

## Decisions

- Public API: `pub fn remend(src: &str) -> std::borrow::Cow<'_, str>` in a new module `src/remend.rs`, re-exported from `src/lib.rs`.
- Composition: add one new top-level helper `streaming_display_with_latex_autowrap_remend(raw: &str, opts: SanitizeOptions) -> String` that runs `wrap_bare_latex → sanitize_streaming_markdown_with → remend → append DEFAULT_TAIL`.
- Existing public API unchanged: `streaming_display`, `streaming_display_default`, `streaming_display_with_latex_autowrap`, `wrap_bare_latex`, `inline_code_options` keep their current signatures and behaviour. Callers opt in to remend by switching to the new `_remend` helper.
- Scope of **mutation**: `remend` only mutates the characters after the last real block boundary in `src`. Real block boundaries are: a blank line (`\n\n`) **that is not inside an open fenced block**, a closed fenced code block's end, or start-of-input. This is what lets remend return `Cow::Borrowed` when the whole input is already well-formed.
- Scope of **scanning**: O(input.len()) in the worst case. Identifying the last real block boundary requires walking from the start while maintaining a fence-stack (a `\n\n` inside an open fenced code block is not a boundary). The `Cow::Borrowed` guarantee above applies to mutation only, not to scanning cost.
- Rule priority (longer runs before shorter, structural before inline): outer fenced code → display math (`$$`) → link `[text](url` → image `![alt](url` → triple emphasis `***` → bold `**` / `__` → italic `*` / `_` → inline code backtick run → strikethrough `~~` → inline math `$`. **HTML tags are not in this list — see Out of Scope.**
- Emphasis opener / closer detection follows CommonMark §6.4 left-flanking / right-flanking rules, grounded on Unicode general-category classification (`is_ascii_whitespace` / `is_alphabetic` alone are not enough; CJK ideographs are Unicode letters and therefore word characters under §6.4). Reference: <https://spec.commonmark.org/0.31.2/#emphasis-and-strong-emphasis>.
- Intraword tightening beyond CommonMark: CommonMark §6.4 permits `*`-delimited intraword emphasis (`un*break*able`) and forbids `_`-delimited intraword emphasis (`snake_case` is preserved). remend tightens the `*` / `**` case to match the `_` / `__` case — a `*`-run or `**`-run whose would-be opener position is flanked by Unicode word characters on both sides does not trigger closer synthesis, even though §6.4 alone would allow it. The `_` / `__` case already forbids synthesis here by §6.4 and is unaffected. Rationale: the asymmetry in the standard is deliberate for authored prose (authors meaningfully emphasise inside words with `*`) but counterproductive for LLM-streamed output (mid-stream intraword `**` at a chunk boundary is almost always the midpoint of `**un**necessary`-style expansion or a typo). A future maintainer evaluating whether to restore §6.4 behaviour should trust corpus evidence over intuition — the third position (relaxing the tightening) is deferred to M2 only if corpus shows intraword `*`-emphasis arises cleanly in streamed output.
- Each rule is a free function `fn close_<rule>(tail: &str) -> Option<String>`. Rules are pure, idempotent, independent. The dispatcher picks the first matching rule at the tail (no rule can apply inside another matched rule's payload).
- Fenced code content bypass: scanning never descends into a closed fenced block's content. An opened-but-unclosed fenced block at the tail is handled by the fenced-code rule itself (synthesise matching closer of the same backtick count on a new line).
- Language-aware bypass: inline rules (emphasis, inline code, math, strike) are bypassed inside fenced blocks whose info-string (lowercased, trimmed) is in the compile-time set `BYPASS_LANGUAGES = {"mermaid", "diagram", "math", "tex", "latex", "typst", "asciidoc"}`. Only the fenced-code closer rule applies inside these blocks. The set is a `const` in `src/remend.rs`, not a runtime knob in M1, to keep the M1 API surface minimal.
- Char-boundary safety: all scanning uses `chars()`, `char_indices()`, `strip_prefix`, `strip_suffix`. No byte indexing.
- No `.unwrap()` / `.expect()` in production paths.
- No new production dependency. No regex. Hand-rolled scanners only.
- Implicit costs of the hard constraints above (recorded so later maintainers know what is red-line vs. negotiable): (a) `no byte indexing` costs ~3–5× on CJK-heavy text vs a byte-level `memchr`-style scan; acceptable because we scan once per `remend` call and typical LLM-chat buffers stay ≤ 10 KB; (b) `no regex` costs hand-written scanner boilerplate per rule; trade-off accepted to keep binary size small (this crate is embedded into Makepad apps) and to avoid pulling a regex engine for ~20 simple rules.
- Tests live in `tests/remend.rs` (integration) and inline `#[cfg(test)]` modules in `src/remend.rs` (unit tests for individual rules).
- Fixture corpus: `tests/corpus/<model>/*.md`, each file a complete assistant response. Streaming chunks are **synthesised by the test harness via prefix sampling** — every `{16, 32, 64, 128, 256, 512, 1024}`-byte offset, every `\n\n`, every mid-fence position. Rationale: remend is a pure function of its prefix; live-stream recording gives no extra information for remend correctness.
- Corpus category coverage — M1 open-gate minimum: at least one sample per category in `{plain prose, code-heavy, math-heavy, mixed CJK/Latin}`. The M1 seed corpus satisfies this with two Kimi responses — `tests/corpus/kimi/markdown-demo-short.md` (~4 KB, `\frac` / matrix / greek letters) + `tests/corpus/kimi/markdown-demo-long.md` (~12 KB, integrals, Bayesian probability, matrix products, GFM alerts, 3 code languages, 7 mermaid types). Math-heavy is listed explicitly because `$…$` / `$$…$$` / bare-LaTeX handling interacts with the `math` / `tex` / `latex` entries in `BYPASS_LANGUAGES`; without math samples, that code path would be untested. DeepSeek / Qwen / GPT samples are deferred to M2 — their unique failure modes (DeepSeek thinking-block retraction, Qwen CJK-on-chunk-boundary) are either Out of Scope for M1 or covered by existing rules.
- Corpus assertions are property-style, not golden strings: (a) no panic, (b) idempotency, (c) valid UTF-8 output, (d) no HTML closer synthesis, (e) no intraword emphasis synthesis, (f) `BYPASS_LANGUAGES` blocks are preserved modulo fenced-code closer, (g) prefix preservation — for every sample, `output[..b]` byte-equals `input[..b]` where `b` is the last real block boundary offset per the "Scope of mutation" Decision. Property (g) is strictly stronger than (b) for catching prefix-mutation bugs. See `tests/corpus/README.md` for harness conventions.
- Prefix sampling covers correctness on all tail positions. It does **not** approximate real LLM chunk-boundary distributions — those follow BPE tokenizer boundaries, not fixed byte offsets. Real-stream-distribution sampling is an M2+ empirical question requiring recorded streaming sessions with timestamps; M1 is deliberately silent on it.
- Prefix-sampling harness API (dev-only, lives in `tests/remend.rs` or a new `tests/corpus/mod.rs`):
  ```rust
  /// Return every sampled prefix for a corpus file.
  fn sample_prefixes(src: &str) -> Vec<&str>;
  /// Run the seven corpus-wide properties (a)–(g) against a set of prefixes.
  /// Panics on first violation with enough context to locate the offending prefix.
  fn assert_all_properties(prefixes: &[&str]);
  /// Load every `.md` under `tests/corpus/<any>/`. For `.jsonl` fixtures,
  /// loads the `expected_final_after_remend` field.
  fn load_corpus() -> Vec<(&'static str, String)>;  // (name, content)
  ```
  These signatures are fixed by this spec — implementer does not re-design the harness at TDD time.
- Constraint evolution principle: Decisions in this spec are implicitly either **red-line** (design-level change needed to relax) or **data-deferred** (could be relaxed if future corpus shows current rule is counterproductive). Currently data-deferred: (i) intraword `*` / `**` tightening beyond CommonMark §6.4 — relax if streamed output ever produces clean intraword emphasis; (ii) `BYPASS_LANGUAGES` as a compile-time `const` — lift to runtime config if downstream callers need to register custom languages. All other Decisions are red-line; reopening requires design review, not just corpus evidence.

## Boundaries

### Allowed Changes

- `src/remend.rs`
- `src/lib.rs`
- `tests/remend.rs`
- `tests/corpus.rs`
- `tests/corpus/**`
- `Cargo.toml`

### Forbidden

- Do not change the signature or behaviour of `streaming_display`, `streaming_display_default`, `streaming_display_with_latex_autowrap`, `wrap_bare_latex`, `inline_code_options`, or anything re-exported from `streaming_markdown_sanitizer`.
- Do not add any new production dependency to `Cargo.toml` `[dependencies]`.
- Do not use a regex crate. Rules must be hand-written scalar-level scanners.
- Do not use direct byte-indexed string slicing that can panic on a multibyte boundary. The recommended safe APIs (char-iteration, `.get()`, `split_at_checked`) are listed in the Decisions "Char-boundary safety" item.
- Do not mutate characters before the last real block boundary — remend is tail-mutation-only (see Decisions for scanning vs mutation scope distinction).
- Do not inspect or modify the contents of closed fenced code blocks anywhere in the input.
- Do not synthesise any HTML tag closer. Auto-closing a half-emitted attribute value (`<img onerror="evil()` → anything) is an injection vector; upstream `streaming_markdown_sanitizer` is responsible for stripping incomplete HTML, remend must leave it alone.

## Out of Scope

- Commit / tail double-buffer architecture — `specs/m2-commit-tail-double-buffer.spec.md` (not yet written).
- Event-stream parser (Tarnawski style) — `specs/roadmap/m3-event-stream-parser.spec.md`. M3 is likely required in the Makepad integration, not "may never ship" — see that spec's Intent for why. M1 deliberately skips it to unblock a 4-week product ship; M1's remend is a scaffold that M3 will either subsume (delete the remend module) or complement (keep as a lightweight mode).
- HTML tag closing-synthesis. Explicitly out of scope for security reasons — handled by upstream sanitizer.
- Thinking-bubble UI for DeepSeek R1 / o1 retraction behaviour — separate UI concern, not a remend rule.
- GFM table partial-row synthesis — existing `sanitize_streaming_markdown` already trims incomplete tables via `trim_incomplete_table`; remend does not attempt to synthesise missing cells.
- Retiring the `unwrap_outer_markdown_fence` / `▋` filter shims in the aichat example — lives in aichat, separate from remend scope.
- Performance benchmarks against Streamdown's JS remend — informational only; not a completion criterion.

## Completion Criteria

Scenario: Unclosed bold at tail gets closing asterisks
  Test: test_remend_closes_trailing_bold
  Given a fresh remend call
  When the caller passes "Hello **world"
  Then the returned string equals "Hello **world**"
  And the return variant is Cow::Owned

Scenario: Unclosed inline code at tail gets closing backtick
  Test: test_remend_closes_trailing_inline_code
  Given a fresh remend call
  When the caller passes "see `foo"
  Then the returned string equals "see `foo`"
  And the return variant is Cow::Owned

Scenario: Unclosed fenced code at tail gets closing fence on a new line
  Test: test_remend_closes_trailing_fenced_code
  Given a fresh remend call
  When the caller passes "```rust\nfn main() {"
  Then the returned string equals "```rust\nfn main() {\n```"
  And the return variant is Cow::Owned

Scenario: Unclosed link at tail gets closing parenthesis
  Test: test_remend_closes_trailing_link
  Given a fresh remend call
  When the caller passes "see [the docs](https://example.com/path"
  Then the returned string equals "see [the docs](https://example.com/path)"

Scenario: Already-closed bold is left unchanged
  Test: test_remend_idempotent_on_closed_bold
  Given a fresh remend call
  When the caller passes "fully **closed** text"
  Then the returned string equals "fully **closed** text"
  And the return variant is Cow::Borrowed

Scenario: Intraword underscore does not trigger italic synthesis
  Test: test_remend_ignores_intraword_underscore
  Given a fresh remend call
  When the caller passes "call snake_case_identifier here"
  Then the returned string equals "call snake_case_identifier here"
  And the return variant is Cow::Borrowed

Scenario: Unclosed emphasis inside a closed fenced code block is not touched
  Test: test_remend_does_not_descend_into_closed_fence
  Given a fresh remend call
  When the caller passes "prefix\n```\nthis **is not bold\n```\nok"
  Then the returned string equals "prefix\n```\nthis **is not bold\n```\nok"
  And the return variant is Cow::Borrowed

Scenario: Mermaid block content is protected from inline rules
  Test: test_remend_protects_mermaid_block_content
  Given a fresh remend call
  When the caller passes "```mermaid\nflowchart LR\n    A[**not bold**] --> B"
  Then the returned string ends with the fenced-code closer "\n```"
  And the substring "A[**not bold**]" appears unchanged in the result
  And no additional `**` is appended inside the mermaid block payload

Scenario: Triple emphasis takes priority over double and single
  Test: test_remend_triple_emphasis_priority
  Given a fresh remend call
  When the caller passes "***all three"
  Then the returned string equals "***all three***"
  And no result contains "****" or "*****"

Scenario: Content before the last block boundary is not mutated
  Test: test_remend_touches_only_tail_after_last_boundary
  Given a fresh remend call
  When the caller passes "closed paragraph **ok**\n\nunclosed **bold"
  Then the returned string equals "closed paragraph **ok**\n\nunclosed **bold**"
  And the substring before "\n\nunclosed" is byte-identical to the input

Scenario: Idempotency holds across every corpus prefix
  Test:
    Package: streaming-markdown-kit
    Filter: corpus::idempotent_on_all_prefixes
  Given the seed corpus `tests/corpus/kimi/markdown-demo-short.md` and `markdown-demo-long.md`
  And a prefix-sampling harness that slices at every {16,32,64,128,256,512,1024}-byte offset and every `\n\n`
  When the harness calls remend on each prefix, then calls remend on that output
  Then for every sampled prefix the second result equals the first
  And for every sampled prefix the second call's return variant is Cow::Borrowed

Scenario: Algebraic prefix-preservation — remend never rewrites bytes before the last real block boundary
  Test:
    Package: streaming-markdown-kit
    Filter: corpus::prefix_preservation_on_all_samples
  Given the seed corpus under `tests/corpus/kimi/`, `tests/corpus/opus/`, `tests/corpus/gpt/`
  And the prefix-sampling harness described above
  When the harness calls remend on each sampled prefix `input` and obtains output `output`
  And the harness computes `b`, the byte offset of the last real block boundary in `input`, defined per the "Scope of mutation" Decision (a `\n\n` not inside an open fenced block, a closed fenced-code end, or 0 for start-of-input)
  Then `output[..b]` equals `input[..b]` byte-for-byte for every sample
  And no rule's insertion point has an offset less than `b`

Scenario: Block-boundary identification is fence-stack aware — \n\n inside an open fenced block is not a boundary
  Test: test_remend_prefix_preservation_fence_interior_nn_not_boundary
  Given a fresh remend call
  When the caller passes "prose\n\n```rust\nfn a() {}\n\nfn b() {"
  Then the returned string ends with the fenced-code closer "\n```"
  And the byte range `[0, len("prose\n\n"))` of the input is preserved byte-for-byte in the output
  And the `\n\n` between `fn a() {}` and `fn b() {` is NOT used as the last block boundary
  And the rule insertion point is at the end of the input, not at the fence-interior `\n\n`

Scenario: Well-formed input returns Cow::Borrowed for the whole input
  Test: test_remend_borrowed_on_wellformed
  Given a fresh remend call
  When the caller passes a string that contains no unclosed markdown construct at the tail
  Then the returned Cow is Borrowed and points at the input slice
  And no allocation happens in the call

Scenario: Malformed pathological input does not panic and surfaces no error
  Test: test_remend_no_panic_on_pathological_input
  Given a fresh remend call
  When the caller passes a pathologically malformed string combining unclosed triple emphasis, unclosed fence, unclosed link, unclosed math, CJK bytes, and the cursor glyph "▋" all at the tail
  Then the call returns a String or borrowed slice without panicking
  And no .unwrap() or .expect() on the call path panics
  And the returned value is a valid UTF-8 string

Scenario: Invalid construct is rejected — underscore flanked by digits is not an italic opener
  Test: test_remend_rejects_intradigit_underscore_as_italic
  Given a fresh remend call
  When the caller passes "version 1_0_beta"
  Then the returned string equals "version 1_0_beta"
  And no underscore is synthesised at the tail

Scenario: Intraword double-asterisk at the tail is not synthesised (remend-tightened, stricter than CommonMark §6.4)
  Test: test_remend_tightened_intraword_bold
  Given a fresh remend call
  When the caller passes "foo**bar"
  Then the returned string equals "foo**bar"
  And no `**` is appended at the tail
  And the return variant is Cow::Borrowed

Scenario: Space-left emphasis at tail closes even when the payload is CJK
  Test: test_remend_space_flanked_bold_at_tail_with_cjk_body
  Given a fresh remend call
  When the caller passes "文本 **粗体"
  Then the returned string equals "文本 **粗体**"
  And the return variant is Cow::Owned
  And the CJK characters in the payload are passed through unchanged

Scenario: CJK-flanked intraword double-asterisk is tightened out by remend
  Test: test_remend_tightened_cjk_intraword_bold
  Given a fresh remend call
  When the caller passes "中文**加粗"
  Then the returned string equals "中文**加粗"
  And no `**` is appended at the tail
  And the return variant is Cow::Borrowed
  And the decision trace records this as remend-tightening, not CommonMark §6.4 rejection (plain §6.4 would allow this opener because CJK ideographs are Unicode word characters and `*` permits intraword emphasis)

Scenario: Partial HTML tag at tail is never closed — regression guard for injection-vector avoidance
  Test: test_remend_never_synthesises_html_closer
  Given a fresh remend call
  When the caller passes "partial <img onerror=\"evil()"
  Then the returned string equals "partial <img onerror=\"evil()"
  And the returned string contains no `>` character at any position beyond the input length
  And no `</img>` substring appears in the returned string
