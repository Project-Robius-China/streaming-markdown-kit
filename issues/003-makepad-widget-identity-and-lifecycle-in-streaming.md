# Issue 003: Makepad widget identity & lifecycle during streaming

Status: open design question. Blocks M3 (event-stream parser) and partially
M2 (commit/tail double-buffer). Does not block M1 (remend-rs).
Recorded: 2026-04-19
Related: issues/002-streaming-render-state-machine-and-render-gap-audit.md

## Why this exists

The M1 → M2 → M3 milestone chain for streaming-markdown-kit focuses on
"what does the parser output?". That misses a Makepad-specific problem:
**how does the Makepad widget tree consume streaming output while
preserving state?** On the web this is cheap — the DOM layer handles it
via diff + layer caching. On Makepad there is no free lunch. This
document captures the gap and the design questions that must be
answered before M2 or M3 can specify widget-side work.

## The four gaps

### 1. Widget identity allocation

In Makepad, widgets are identified by live IDs. Widget state (scroll
offset, selection range, hover / focus, cached layout, cached shader
uniforms, cached font runs) is keyed on that ID. The choice of "how do
we allocate IDs to streaming-emitted blocks?" determines what survives
a re-render:

| Strategy | Problem |
|----------|---------|
| `block_N` by position in document | Inserting a new block ahead shifts every subsequent ID → all downstream widgets lose state (scroll, selection, highlight cache) |
| `hash(block_content)` | Content changes every streaming token while the block is open → ID churns every frame → state lost every frame (worse than position) |
| `hash(block_content_at_close_time)` | Requires block close to materialise the stable ID → until close, widget cannot retain state → flicker on every chunk until block finishes |
| `monotonic_counter_per_open_event` | Stable ID from first `EnterBlock` onward. New blocks get new IDs. Insert-before-existing-content is impossible by definition of append-only stream. **Candidate.** |

The `monotonic_counter_per_open_event` strategy is compatible with the
append-only contract of M3's `RenderEvent` stream. It is the only
strategy from the table that keeps widget state stable across streaming
updates without waiting for block close.

**Caveat: the strategy works cleanly only under M3, not M2.** Under M2's
commit/tail model the tail region is re-parsed every chunk, which means
the same physical block may be seen as an `EnterBlock` multiple times —
once per re-parse — until it crosses into the committed region. If the
block's interpretation changes between re-parses (e.g., a partial fence
info-string becomes valid on the next chunk), a counter-based ID would
flip-flop. Under M3 the parser state is monotonic and this cannot
happen.

M2's implication: tail widgets should be cheap (no highlight cache, no
mermaid render, no scroll/selection memory) so that their ID churning
across tail re-parses is invisible to the user. Only at commit boundary
is the block's ID frozen and its expensive widgets (syntax highlight,
mermaid, latex) instantiated. M3 generalises this by making every block
"append-only committed" from its first `EnterBlock`, removing the
cheap-tail constraint.

Decision needed: confirm `monotonic_counter_per_open_event` for M3.
For M2, the rule is "tail widgets are cheap; committed widgets own
stable IDs" — make this explicit in M2 spec when written.

### 2. Selection & scroll preservation

Makepad does not have a DOM-equivalent document selection model. Text
selection and scroll offset are per-widget state, stored on the widget
(or on a `TextEditor` if one is used). Consequences for streaming:

- If the Markdown widget's current approach is `Label`-per-block (read-only
  text display), no cross-block selection is possible at all. Copy-paste
  of multi-paragraph output is impossible. Users will notice.
- If we switch to `TextEditor`-per-block, selection is per-block only.
  Dragging across two paragraphs copies one. Also user-noticeable, and
  a worse UX than plain `TextEdit` over the whole buffer.
- A full-document `TextEditor` with inline-styled runs is the
  web-equivalent UX but is a large widget rewrite in Makepad.

Decision needed: **what is the minimum acceptable selection UX?** This
drives whether M3 must include a `TextEditor`-based layer or can stay
`Label`-per-block.

### 3. Syntax highlight cache keying

Rust / TypeScript / Python fenced code blocks are syntax-highlighted by
some tokeniser (syntect-rs, tree-sitter, or a roll-our-own). Each one
is expensive — 500-line Rust block tokenisation is multi-millisecond.
With the current full-buffer-reparse model, a 500-line Rust block in
the middle of a long reply gets re-tokenised on every streaming token.

Needed: cache key. `(language, content_hash)` is the obvious choice, but
while the block is open, content changes every token → cache miss every
frame → problem not solved. The fix is to tie cache invalidation to
`EnterBlock` → `ExitBlock` boundaries (tokenise once when the block
closes), which requires M3's event stream.

For partially-open blocks during streaming, options are:
- Show plain monospace, no highlight, until close → "render pop" on close.
- Re-tokenise per chunk anyway, accept the cost → defeats the purpose of
  caching.
- Incremental tokeniser (tree-sitter supports this; syntect does not) →
  tokenise per chunk cheaply, keeping the ID stable via gap (1) above.

Decision needed: which of the three. My current guess is "plain
monospace until close" for simplicity, but that's a visible UX
regression relative to current Makepad Markdown widget behaviour if
the widget already highlights during streaming (which it may — need to
check).

### 4. Expensive terminal renderers (Mermaid, LaTeX)

Same issue as (3) but worse — Mermaid's layout algorithm is dozens of
milliseconds on a non-trivial diagram; LaTeX typesetting is similar.
These MUST not run per chunk. They must wait for `ExitBlock`.

The current aichat implementation achieves this by **never triggering**
the Mermaid renderer until the whole buffer is present — i.e. it's
already "on close" because the whole buffer is "closed" when the
streaming loop ends. With commit/tail (M2), committed blocks close
before the stream ends, so Mermaid can render earlier → earlier visible
feedback. With event-stream (M3) this is even cleaner.

No decision needed here — the design is obvious (wait for `ExitBlock`).
But the widget-side plumbing (subscribe to `ExitBlock` events, dispatch
to the right sub-renderer) does not exist yet.

## What this means for the milestone chain

- **M1 (remend-rs)**: unaffected. Pure string transform, no widget
  concerns.
- **M2 (commit/tail)**: affects gap 1 (IDs of committed blocks must
  survive tail re-parse) and gap 4 (committed mermaid blocks can
  render earlier). M2 spec should reference this document and
  explicitly decide the committed-block ID strategy.
- **M3 (event-stream)**: affects all four gaps. M3 spec should not be
  finalised until the four decisions above are made — otherwise M3 will
  ship a parser whose consumer (the Makepad widget) cannot use it
  correctly.

## What to do next

Before M3 planning:

1. **Measure**: with the current full-buffer-reparse implementation,
   profile per-chunk CPU for a representative streaming response that
   includes a 500-line fenced Rust block and a 20-node mermaid diagram.
   If the per-chunk cost is below, say, 8ms (120fps budget), M3 may
   not actually be needed. If above 16ms (60fps budget), M3 is required.
2. **Check current widget behaviour**: does the current Makepad Markdown
   widget re-allocate all sub-widgets on every `set_text`, or does it
   have a diff mechanism? This is a 30-minute spike and answers
   whether gap 1 is a real problem today or only a theoretical one.
3. **Decide selection UX minimum**: 15-minute product decision. Write it
   down here once made.

Items 1 and 2 are implementation diagnostic tasks, not spec work. They
should happen during or after M1 to inform M2 and M3 scope.

## Update 2026-04-19 — item 2 answered (M1 aichat integration spike)

Read `widgets/src/markdown.rs` after wiring `remend` into aichat. Findings:

- **`Markdown::set_text` diff-gates at line 370.** `if self.body.as_ref() != v { self.body.set(v); self.redraw(cx); }` — identical consecutive `set_text` calls are O(1) no-ops. ✓
- **`process_markdown_doc` runs pulldown-cmark from scratch on every draw** (line 413 builds a fresh `Parser::new_ext` over the full buffer). No cached AST, no incremental parse.
- **TextFlow is immediate-mode.** Each draw walks the event stream and calls `tf.bold.push`, `tf.new_line_collapsed_with_spacing`, etc. directly — there are no retained per-block widget handles, no live IDs per paragraph / code block / heading.

### Consequences for M2 and M3

1. **Gap 1 (widget identity) does not exist today.** Issue 003's
   "widget-identity allocation" discussion assumed TextFlow had per-block
   retained state. It doesn't. Every redraw rebuilds the flow from
   scratch. There is no state to lose → no identity allocation
   strategy needed for M2.

2. **M2 (commit/tail double-buffer) is simpler than assumed.** It
   becomes a pure string-level bookkeeping task in aichat (or in
   `streaming-markdown-kit`) — maintain `(committed, tail)`, only run
   remend + sanitize on `tail`, pass `committed + tail_processed` to
   `set_text`. No Makepad widget changes needed for M2. The
   "committed-block ID strategy" open question in this issue
   (Caveat section above) is moot until M3.

3. **M3 (event-stream parser) is MORE work than originally scoped.**
   To get the benefits M3 is supposed to deliver (incremental
   highlighting cache, fenced-block-close-triggered mermaid render,
   append-only widget lifecycle preserving scroll / selection / cached
   layout), TextFlow itself must become a retained-mode container with
   block-level children. That is a Makepad architectural change, not
   just a new parser crate. Scope recalibration belongs in any future
   M3 spec.

4. **Re-render flicker is real but bounded.** Each token triggers a
   full `process_markdown_doc` pass over the current buffer. For a
   10 KB buffer this is a few milliseconds — noticeable but not the
   widget-identity-churn problem I had mismodelled. The dominant
   visual instability at M1 time was the upstream *misinterpretation*
   flicker (tolerant-recovery flipping between interpretations), which
   `remend` addresses. Whether additional instability justifies M2 /
   M3 is an empirical observation to make with remend now in place.

### Items still open

- **Item 1 (per-chunk CPU profile)**: not yet done. Needs a live run
  of aichat with a representative streaming response; no Makepad
  profiling harness is in place yet. Leave for later.
- **Item 3 (selection UX decision)**: still a product call. Not
  blocked by M1.

## Update 2026-04-19 (late) — "content disappears mid-stream" diagnosis reversal

The P2 test prompt ("write a 300+ line streaming-markdown-parser in a
single fenced rust block") reproducibly shows content disappearing mid-
stream once the response length exceeds ~50 lines. Initial diagnosis
chain — **all wrong**, left here so future work doesn't repeat them:

1. **Suspected remend synthesising bad closer** → ruled out by flipping
   `USE_REMEND = false` in aichat and reproducing the symptom anyway.
2. **Suspected `streaming-markdown-sanitizer` trimming a `\n\n` inside
   the open fence because the tail line looks like a GFM table** →
   partially valid (fixed independently; sanitizer is now fence-aware
   for table detection — see sanitizer commit, regression tests added)
   but didn't close the gap. Fix kept for correctness regardless.
3. **Suspected Kimi streaming connection restart mid-prompt** (based on
   a `len=4941 → len=67` drop in an early log) → the drop turned out
   to be the legitimate prompt transition (`TurnComplete` on the
   previous greeting followed by a fresh `send_message` for the P2
   prompt). Fine-grained instrumentation on TextDelta / send_message /
   TurnComplete / set_text confirmed there is no anomalous reset.

**Actual finding**: under sustained instrumentation, `data.streaming_text`
grows monotonically to 12 KB+. Every `set_text` call is invoked with
the full current buffer. **The data layer is correct.** The content
the user sees disappearing was rendered earlier; what goes wrong is
later redraws fail to re-render some of it.

This places the bug in **Makepad's render layer**, not in anything
streaming-markdown-kit or aichat-layer owns. Most likely candidates
(ordered by suspicion):

1. **`CodeView` inside the Markdown widget** (activated via
   `use_code_block_widget: true`, routes fenced-code to
   `makepad-code-editor::CodeView`). CodeView has its own text buffer
   + retained editor state + layout. A rapid series of `set_text`
   calls with ever-growing buffers could trigger stale scroll offsets,
   editor-buffer desync with requested content, or incomplete redraw
   after relayout.
2. **Makepad Markdown widget's flow layout on very tall fenced blocks**
   — the widget lays out via TextFlow; a fenced block that grows past
   some internal cache size might clip incorrectly or skip re-measuring.
3. **PortalList `auto_tail: true` interaction** — the Assistant
   bubble grows as content streams. If `auto_tail` miscalculates a
   growing item's anchor, it may scroll past the bubble and the user
   perceives content as "gone".

Current bisect step (2026-04-19): set `use_code_block_widget: false`
temporarily in both Markdown instances in aichat/main.rs. If the symptom
disappears, suspect #1 is confirmed. Result not yet in hand.

**Bisect result (2026-04-19, same day)**: **CodeView confirmed**.
With `use_code_block_widget: false`, P2 passes cleanly even with a
12 KB+ buffer / 180+ line Rust fenced block streamed token-by-token.
Content does not disappear at any point. The parser pipeline
(remend + sanitizer + streaming-markdown-kit) is completely clean;
the regression is entirely inside `makepad-code-editor::CodeView`'s
handling of rapid `set_text` calls with ever-growing content.

**Workaround**: keep `use_code_block_widget: false` in aichat. Trade-
off: fenced code blocks render as plain fixed-font inline text, losing
syntax highlighting. Acceptable for product shipping; upstream CodeView
fix tracked separately (M-widget-rendering item).

### Consequence for M2 / M3 roadmap

Independent of whether suspect #1, #2, or #3 is the culprit, this is
**Makepad widget-layer work, not streaming-parser work**. It does not
change the M1→M2→M3 axis for remend-rs / commit-tail / event-stream,
but it adds a new deliverable:

- **M-widget-rendering** (new, upstream Makepad concern): triage and
  fix whichever of CodeView / Markdown-flow-layout / PortalList
  misbehaves on long streamed fenced blocks. Does not block M2. Can
  ship independently.

A terse record of the bisect outcome will be added here once the `use_code_block_widget: false` test lands.

## Non-goals

- This is not a task contract. It is a design-constraint document. Do
  not attempt to implement any code based on it directly — derive a
  spec first.
- Widget-side work is out of scope for the `streaming-markdown-kit`
  crate. This document will eventually move (or get a twin) under the
  Makepad fork's `docs/` tree once M3 starts. For now it lives here
  because the parser design constrains it.
