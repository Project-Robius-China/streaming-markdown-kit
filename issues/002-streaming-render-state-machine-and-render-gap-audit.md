# Issue 002: Streaming render state machine + render-gap audit

Status: design proposal + diagnostic backlog. No code change yet.
Recorded: 2026-04-18
Related: 001-multibyte-chars-byte-vs-char-vs-grapheme.md

## Why this document exists

Through the aichat integration cycle (streaming-markdown-kit + Makepad's
Markdown widget + rusty-mermaid for inline mermaid blocks) we accumulated:

1. A pile of **rendering / parsing bugs** spread across three repos —
   rusty-mermaid (parser + SVG primitives), Makepad (SVG draw layer +
   Markdown widget hook), and aichat (the integration shim).
2. A growing realisation that the **rendering pipeline has the wrong
   shape**: streaming-markdown-kit emits a *fully transformed string* on
   every token, Makepad's Markdown widget *re-parses the whole string* on
   every set_text, and we then apply ad-hoc text repair (`unwrap_outer_markdown_fence`,
   `▋` cursor stripping, fence-state guesses) at the integration layer
   because neither side has a token-level streaming API.

This document consolidates both — first a flat audit of what's fixed /
what isn't, then the proposed streaming state-machine design that would
remove a whole class of these bugs at the source.

---

## Part A — Diagnostic summary

### A.1 rusty-mermaid (parser + SVG primitives)

| # | Symptom | Root cause | Status | Fix location |
|---|---------|-----------|--------|--------------|
| 1 | Panic on CJK / `▋` / emoji input | byte-indexed `&input[1..]` in error-recovery | **fixed** | 5 parser sites — see issue 001 |
| 2 | Node labels render with literal `"…"` quotes (`A["Hello"]` → `>"Hello"<`) | Quote-strip step ran only for `[…]`, not for `[(…)]`/`[[…]]`/`((…))` | **fixed** | flowchart/parser.rs — generalised quote strip across all bracket variants |
| 3 | Multi-source edges (`A & B --> C`, `C --> D & E`) silently dropped one side | Edge parser only emitted single source × single target | **fixed** | flowchart/parser.rs — fan-out over `&`-separated sides |
| 4 | CJK / mixed text inside `<br/>` line-breaks rendered as mojibake | `normalize_line_breaks` split on byte offset, slicing CJK mid-codepoint | **fixed** | svg/primitive.rs — char-boundary safe split |
| 5 | Dark theme stroke/text contrast too low on default palette | Theme::dark used near-identical fill+stroke for several built-ins | **fixed** | core/style.rs — re-tuned for the dark backgrounds aichat actually uses |

PR: <https://github.com/base58ed/rusty-mermaid/pull/2>

### A.2 Makepad — SVG draw layer

| # | Symptom | Root cause | Status |
|---|---------|-----------|--------|
| 6 | `<text>` elements in rusty-mermaid SVGs invisible | DrawSvg ignored `<text>`; only handled paths | **fixed** — text command pipeline added (`render_text_cmds` in aichat::main, with view of opening up DrawText measurement upstream) |
| 7 | Edge arrowheads missing | DrawSvg honoured `marker-end="url(#…)"` only via the inline `<marker>` tag, but rusty-mermaid emits a single bare `<path>` triangle definition | **fixed** — synthesise triangle marker per edge endpoint when `marker-end` resolves |
| 8 | Mermaid code blocks shown as raw text inside the Markdown widget | Markdown widget had no extension hook for fenced-block hand-off | **fixed** — `MermaidSvgView` widget + Markdown extension hook (per-block `info_string == "mermaid"` → forward to MermaidSvgView) |

Pushed to `ZhangHanDong/makepad` dev branch.

### A.3 Makepad — Markdown widget rendering gaps (UNFIXED)

These are widget-feature gaps, not parsing bugs. They show up the moment the
LLM emits anything beyond headings + paragraphs + fenced code:

| # | Element | Current behaviour | What's missing |
|---|---------|-------------------|----------------|
| 9 | `[text](url)` links | rendered as inline text only — no underline, no click | inline-link styling + click→action wiring in widgets/src/markdown.rs |
| 10 | `![alt](url)` images | fully ignored (or shown as bracket text) | image fetcher + inline image widget; needs an async fetch path before draw |
| 11 | GFM tables (`\| col \| col \|`) | **rendered as nothing — widget eats table events silently**. Verified 2026-04-19: `widgets/src/markdown.rs:706-729` has 8 `Tag::Table*` / `TagEnd::Table*` match arms, all with literal `// TODO: Implement table support` bodies. `pulldown-cmark` fires Table events (feature `ENABLE_TABLES` is on) but the widget consumes them and produces no flow output. Cell contents may leak through as inline Text from some paths. | table block parser + grid widget instance per row/col |
| 12 | Mermaid `stateDiagram-v2` | rusty-mermaid parses it, but `state_count` of nodes/edges renders below threshold visibility — the SVG comes back ~empty (see `tests/probe_svg.rs::probe_state_diagram_renders`) | state-machine renderer in rusty-mermaid is incomplete (separate from this repo) |
| 12b | **LaTeX inside mermaid node/edge labels** | Observed 2026-04-19 during M1 integration testing. LLMs (Kimi / DeepSeek) occasionally emit mermaid edges like `Shipped --> Lost: 物流异常\nt_{transit} >\mathbb{E}[T] + 3\sigma` with LaTeX inside the label. rusty-mermaid renders the label as-is (SVG `<text>` shows raw `\mathbb{E}`, `\text{}`, `_{}`, etc.). Mermaid.js itself doesn't interpret LaTeX in labels natively either (experimental KaTeX plugin exists upstream but not in rusty-mermaid). **Not an M1 regression.** | Either (a) accept as limitation and document, (b) pre-process mermaid source in aichat to strip/simplify LaTeX in labels before calling `render_mermaid_to_svg`, or (c) upstream rusty-mermaid to post-process `<text>` content through a KaTeX renderer. Option (a) is the cheapest; (c) is the right answer and belongs in rusty-mermaid's issue tracker. |

(9)-(11) are upstream Makepad widget work; (12), (12b) are upstream rusty-mermaid / LLM-output quirks.

### A.4 aichat-layer (integration shims)

| # | Concern | Where | Note |
|---|---------|-------|------|
| 13 | LLM wraps reply in outer ```` ```markdown … ``` ```` | `unwrap_outer_markdown_fence(text)` — main.rs:410 | Handles any backtick count ≥ 3, optional opener/closer asymmetry, returns body even if closer not yet streamed |
| 14 | `▋` streaming cursor leaks into mermaid source | `MermaidSvgView::set_mermaid_src` filters `▋` before passing to rusty-mermaid | Real fix is fence-state-aware emission in `streaming_display_with_latex_autowrap` — covered in Part B |
| 15 | Per-frame log spam (`[mermaid-svg] drawing N text cmds`) | removed | NextFrame loop drives the flow-dot animation → 60fps redraw → diagnostic logs are fatal |

---

## Part B — Streaming state-machine parser

### B.1 Current pipeline (and why it's wrong-shaped)

```
LLM token  ──►  buffer.push_str(token)
                       │
                       ▼
            ┌─────────────────────────────────┐
            │ streaming_display_with_latex_   │   re-runs against
            │ autowrap(&full_buffer)          │   the entire buffer,
            └─────────────────────────────────┘   every token
                       │
                       ▼ String (transformed)
            ┌─────────────────────────────────┐
            │ unwrap_outer_markdown_fence()   │
            └─────────────────────────────────┘
                       │
                       ▼ &str
            ┌─────────────────────────────────┐
            │ markdown_widget.set_text(cx, …) │   re-parses whole
            │   ↳ pulldown-cmark on full str  │   string into a fresh
            │   ↳ rebuild widget tree         │   widget tree
            └─────────────────────────────────┘
```

Three problems compound:

- **O(N²) on token count.** N tokens × O(N) full-string transform × O(N)
  full-string parse + tree rebuild.
- **Unstable intermediate states.** Every token boundary is a parse point.
  Mid-emission an opening ` ``` ` exists without its closing pair, an
  opening `[` without its `]`. Pulldown-cmark's tolerant recovery still
  produces visibly different trees frame-to-frame ("chrome flicker": text
  briefly bold, then not, then bold again).
- **Integration-layer hacks accumulate.** `unwrap_outer_markdown_fence`,
  `▋` filtering, `wrap_bare_latex` are all attempts to make a *string*
  shaped right enough for a *stateless* re-parse. With a real streaming
  parser none of these are needed at the integration layer — they belong
  to (or vanish into) the parser's state.

### B.2 Community precedent — Tarnawski's `streaming-markdown.js`

Reference implementation: <https://github.com/thetarnav/streaming-markdown>

The model is:

```
parser  =  { state_stack, buffer, last_open_node }
parser.feed(chunk)  →  emits Render events:
                         ENTER(BlockType, attrs)    -- "open <p>", "open <ul>"
                         TEXT(string)               -- "append 'foo' to current"
                         EXIT                        -- "close current"
                         RAW_INLINE(InlineType, …)  -- "open *em*", "close *em*"
```

Three properties make it composable with retained-mode UIs (which is what
Makepad's Markdown widget is):

1. **Append-only event stream.** Once `ENTER(Paragraph)` fires, that
   paragraph node exists in the renderer; subsequent `TEXT` events append
   characters to it. The renderer never needs to throw work away.
2. **Pending state lives in the parser, not in the rendered tree.** While
   the parser is "inside an unclosed ``` ``` ``` ", the renderer already
   owns an open `CodeBlock` node accumulating raw text. When the closing
   fence arrives, the parser fires `EXIT`. No widget rebuild.
3. **Speculative rollback is bounded.** The only time the renderer must
   retract is when a line-prefix token (e.g. `> ` for blockquote) flips
   meaning — and the spec bounds the rollback window to one block. Easy
   to implement: keep a "tentative open" marker the renderer can promote
   or discard.

### B.3 Adapter sketch for Makepad

streaming-markdown-kit becomes the **token-stream owner**; the Makepad
Markdown widget becomes a **render listener**.

```rust
// New crate-level type
pub struct StreamingParser {
    state_stack: Vec<BlockState>,
    open_inline: Vec<InlineState>,
    pending_line_prefix: Option<LinePrefix>,
    // sanitizer + latex autowrap baked in here, NOT applied as a string
    // post-pass after the fact
}

pub enum RenderEvent<'a> {
    EnterBlock(BlockKind, BlockAttrs),
    ExitBlock,
    EnterInline(InlineKind),
    ExitInline,
    Text(&'a str),
    HardBreak,
    SpecialBlock(SpecialBlockKind, &'a str), // mermaid, latex, etc.
}

impl StreamingParser {
    pub fn feed<F: FnMut(RenderEvent<'_>)>(&mut self, chunk: &str, mut emit: F);
}
```

Makepad-side, the Markdown widget grows a `consume_event(cx, ev)` entry
point parallel to today's `set_text`. Existing `set_text` callers continue
to work — they internally drive a one-shot `StreamingParser::feed` with
the whole string.

### B.4 What the integration-layer hacks become

| Today's hack | Future location |
|--------------|-----------------|
| `unwrap_outer_markdown_fence(text)` | StreamingParser sees the outer ` ``` markdown` opener, recognises "self-wrap" mode, emits no block for the wrapper itself. Inner blocks emit normally. |
| `▋` cursor filter inside `set_mermaid_src` | StreamingParser owns "is this character inside an open fenced block?" — when emitting the cursor it routes via a separate `RenderEvent::Cursor` channel that the renderer can place at the live insertion point regardless of block context. The cursor never enters the source-text channel. |
| `wrap_bare_latex(text)` | Latex detection becomes a state-machine branch: when scanner sees `$$` or single-`$`-with-context, emit `EnterBlock(Latex, …)` directly; no string mutation. |

### B.5 Migration plan (proposed, not yet committed)

Three milestones, each independently shippable:

1. **M1 — Parser skeleton.** Internal-only `StreamingParser` with
   block-level state machine (paragraph / heading / fenced-code / list);
   no inline parsing yet, inline content goes through as `Text`. Drive
   it from existing `streaming_display_with_latex_autowrap` as a parallel
   test target — for every input/expected-output pair the existing
   renderer produces, the new parser must emit an event sequence that
   reduces to the same output. Build confidence without changing the
   public API.
2. **M2 — Inline events + Makepad consumer.** Add inline-level events
   (em / strong / code / link). On the Makepad side add
   `Markdown::consume_event` and route the aichat streaming path through
   it. Old `set_text` keeps working for non-streaming consumers.
3. **M3 — Retire string post-passes.** Move sanitizer and latex
   autowrap into the parser proper. Drop `unwrap_outer_markdown_fence`,
   the `▋` filter, and `wrap_bare_latex` from aichat. Their behaviour is
   now folded into the parser's state.

### B.6 Open design questions

- **Multi-byte-safe lookahead.** The parser needs to peek at the next
  one or two scalars without committing. Use the same `chars().clone()`
  pattern as issue 001 — never byte-index.
- **Mermaid / latex hand-off.** When the parser hits an unrecognised
  fenced info-string, does it emit `SpecialBlock(Unknown, …)` or fall
  back to a generic `CodeBlock(info, …)`? Probably the latter; the
  Markdown widget's existing extension-hook table decides routing.
- **Backpressure.** If the renderer stalls (offscreen / hidden tab),
  events buffer in-memory unbounded. Need a soft cap + collapse rule
  (multiple `Text` events to the same open block coalesce).

---

## References

- Tarnawski, *streaming-markdown* (TypeScript reference impl)
  <https://github.com/thetarnav/streaming-markdown>
- CommonMark spec — fenced code blocks
  <https://spec.commonmark.org/0.31.2/#fenced-code-blocks>
- Pulldown-cmark — what we currently re-parse with each token
  <https://github.com/raphlinus/pulldown-cmark>
- rusty-mermaid PR #2 (Part A items 2–5)
  <https://github.com/base58ed/rusty-mermaid/pull/2>
- Makepad fork dev branch (Part A items 6–8 + 13–15)
  <https://github.com/ZhangHanDong/makepad/tree/dev>
