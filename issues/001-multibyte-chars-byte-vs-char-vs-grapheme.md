# Issue 001: Multi-byte chars — byte vs char vs grapheme

Status: partially resolved (parsers fixed with `char`), render-layer follow-up pending.
Recorded: 2026-04-18

## Context

While integrating streaming-markdown-kit + Makepad + rusty-mermaid for the
aichat example, two classes of bugs surfaced that trace back to mixing up the
three "unit of a character" abstractions in Rust/Unicode:

1. **Parser panics** (rusty-mermaid diagrams): on CJK / `▋` / emoji inputs.
   Example: `thread 'main' panicked at rusty-mermaid/crates/diagrams/src/
   sequence/parser.rs:62 ... byte index 1 is not a char boundary`.
2. **Render-width miscalculation** (aichat `render_text_cmds`): text anchor
   `text-anchor="middle"` drifts off-centre when labels contain CJK or emoji.

Root cause is the same confusion in both cases: **which "unit" do we want
when iterating a string?**

## The three layers

| Layer | Example | API | Semantics |
|-------|---------|-----|-----------|
| **byte** | `'#'` = 1 byte | `&s[i..]`, `s.as_bytes()` | Storage unit. UTF-8 sequences are 1-4 bytes. Arbitrary byte-index slicing panics mid-codepoint. |
| **char** (Unicode scalar) | `中` = 1 char = 3 bytes | `s.chars()`, `s.char_indices()` | One Unicode code point. `for c in s.chars() { ... }` is always UTF-8-boundary safe. |
| **grapheme cluster** | `👨‍👩‍👧` = 5 chars with ZWJ = 1 grapheme | `unicode_segmentation::graphemes(s, true)` | One user-perceived character. Covers ZWJ sequences, combining accents, flag emoji, etc. Requires an external crate. |

## Where each layer is correct

### Parsers — `char` is enough

rusty-mermaid's parsers had this error-recovery pattern:

```rust
if !try_parse_statement(input, ...)? && !input.is_empty() {
    *input = &input[1..];  // WRONG: panics on multi-byte starts
}
```

The **intent** is "skip one unrecognised unit and try again." The smallest
meaningful unit for a parser is a Unicode scalar — not a grapheme. Going to
grapheme would:

- Pull in `unicode-segmentation` for no semantic win.
- Still produce identical skip behaviour for parser-level error recovery:
  a 5-char emoji family gets skipped in 5 iterations (each iteration a no-op
  that advances by one char), vs 1 iteration with grapheme. End result is
  the same — the whole cluster is bypassed.

Fix applied (5 sites):

```rust
if !try_parse_statement(input, ...)? && !input.is_empty() {
    let mut chars = input.chars();
    chars.next();
    *input = chars.as_str();  // char-safe, O(1) amortised
}
```

Sites fixed in rusty-mermaid:
- `crates/diagrams/src/sequence/parser.rs:62`
- `crates/diagrams/src/class/parser.rs:49`
- `crates/diagrams/src/state/parser.rs:69`
- `crates/diagrams/src/state/parser.rs:335`
- `crates/diagrams/src/er/parser.rs:39`
- `crates/diagrams/src/requirement/parser.rs:39`

### Render / text layout — `char` is insufficient, `grapheme + font metrics` is correct

aichat `MermaidSvgView::render_text_cmds` estimates text width for the
`text-anchor="middle"` offset:

```rust
let est_width: f64 = line
    .chars()
    .map(|c| {
        let advance = if (c as u32) >= 0x2E80 { 1.0 } else { 0.55 };
        advance * world_font_size
    })
    .sum();
```

This is a **char-level** estimate. It's wrong for:

- Emoji families (`👨‍👩‍👧` = 5 chars, visual width ≈ 1em → estimate gives
  ≈ 5em, 5× too wide).
- Combining-accent letters (`é` via `e` + `\u{0301}` = 2 chars, visual
  width ≈ 1em → estimate gives ≈ 1.1em, slightly too wide).
- Variation selectors, zero-width chars, etc.

For typical mermaid labels (plain CJK + Latin, no emoji families) the
char-level estimate is *good enough*. But the correct model is:

1. Iterate grapheme clusters, not chars.
2. For each grapheme, ask the font shaper for actual advance width.
3. Sum.

Makepad's DrawText already does this internally at draw time. The gap is
that DrawText's measurement API isn't exposed so `render_text_cmds` can't
query it. Two options:

- Use `unicode-segmentation::graphemes(line, true).count()` with hand-picked
  advance ratios per grapheme — ~5 LoC + one crate dep. Fixes emoji families
  but still approximate.
- Extend Makepad's DrawText to expose a `measure_line(cx, style, text) ->
  f64` API. Correct, shared across other widgets that need it, but requires
  upstream PR.

## Unresolved / follow-up

1. **Grapheme-aware text width estimate** in `aichat/src/main.rs :: render_text_cmds`. Pick option 1 for quick wins, option 2 for the long term. Tracked by the `<br/>`-wrap + CJK-anchor feedback we've been tuning through this session.
2. **Inventory other parser sites** in rusty-mermaid that might still use
   byte-indexed slices. Audit list:

   ```
   *input = &input[N..]
   &s[i..j]
   s.as_bytes()[i]
   ```

   The 5 sites above were **error-recovery** advances (dangerous). The bulk
   of other occurrences advance past a known-ASCII literal (`'['`, `':'`,
   etc.) which is safe — the first byte is guaranteed ASCII.
3. **Streaming cursor `▋` leak into mermaid source**: patched in
   `MermaidSvgView::set_mermaid_src` by filtering `▋` before handing to
   rusty-mermaid. The deeper fix would be for `streaming_display_with_latex_autowrap`
   to not append `▋` inside an open fenced block — but that requires it to
   know the fence state, which it currently doesn't.

## References

- Rust `str` slicing requires char boundaries:
  <https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary>
- `unicode-segmentation` crate for grapheme iteration:
  <https://crates.io/crates/unicode-segmentation>
- Unicode Annex #29 — Text Segmentation:
  <https://www.unicode.org/reports/tr29/>
- Makepad 2.0 font skill — grapheme-based shaping is the rendering layer's
  concern; parsing/error-recovery is the code layer's concern.
