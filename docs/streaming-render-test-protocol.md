# Streaming render test protocol

Human-eye-test harness for validating M1 remend / gauging whether M2 is
needed / triaging Makepad widget regressions under sustained streaming
load. Each P-series prompt is designed to stress **one** specific
failure mode; run them individually, not concatenated. Recorded
2026-04-19; extend as needed.

## How to use

1. Run aichat. Keep a single window focused — no mid-test clicks,
   no "send again" mistakes.
2. Optional: set `AICHAT_STREAM_LOG=1` before `cargo run` to get a
   per-delta / per-set_text / send_message / TurnComplete log at
   `aichat_stream.log` next to the binary.
3. Paste one of the four P-prompts below and wait for the stream to
   finish completely. Observe the specific symptoms listed per-prompt.
4. Where the protocol asks for "compare against M1-off baseline":
   flip `USE_REMEND` (the `const bool` toggle in `aichat/src/main.rs`)
   to `false`, rebuild, retest with the same prompt.

## Failure-mode taxonomy (shared vocabulary)

Two distinct flicker categories underlie all observations:

- **Misinterpretation flicker** — same already-displayed characters
  toggle styling between frames because pulldown-cmark's tolerant
  recovery picks different interpretations on each token boundary.
  E.g. a word is bold for one frame, plain the next, bold again on the
  third. This is what M1 remend is specifically designed to kill.
- **Re-render flicker** — the buffer is re-parsed and the widget tree
  rebuilt on every token. Symptoms: stutter, frame drops, syntax
  highlight "pop-in" when a code block closes, mermaid diagrams
  jumping as they're re-layouted per token. M2 / M3 addresses this.

A third, non-flicker class also surfaced during M1 validation:

- **Content disappearance** — previously-rendered content vanishes
  from the view. Root cause is typically in the Makepad render layer
  (CodeView, TextFlow long-block layout, or PortalList auto-tail), not
  in the parser pipeline. See
  `issues/003-makepad-widget-identity-and-lifecycle-in-streaming.md`.

## P1 — Misinterpretation pressure test (validates M1 remend)

```
用一段 200 字左右的段落讲 Rust 的 ownership 模型。要求：
- 每句话里至少有 **粗体**、*斜体*、`行内代码` 三种之一
- 用到关键词 Move、Borrow、Lifetime、Drop、Send、Sync、Copy、Clone
- 其中至少 5 处是 ***粗斜体*** 或 `code with **bold inside**` 这种嵌套
写满 200 字，不要分段
```

**What to watch**: already-displayed bold / italic / strike / inline-code
fragments. Do their styles oscillate (bold → plain → bold) as the stream
continues? If M1 remend is working, **they must not**. A stable single
answer is the pass criterion.

**Pass** — no observed re-styling of already-displayed runs.
**Fail** — capture the exact sentence that flickered + approx. stream
position. Send both to remend triage; the rule table in `src/remend.rs`
is extended to handle it.

## P2 — Long code block re-render pressure test

```
用 Rust 写一个完整的、可运行的流式 markdown parser。要求：
- 至少 300 行代码
- 带完整的 use 声明、struct 定义、impl 块、单元测试
- 每个函数都有中文 doc 注释
- 整个输出放在一个 ```rust fenced block 里，一气呵成
```

**What to watch**:

- When the code block is mid-stream, do already-rendered code lines
  stutter, flash a different colour, or cause scroll to jitter?
- Does the code block's height grow smoothly, or does it resize in
  large jumps that cause viewport displacement?
- Most importantly — does previously-visible content **disappear**
  mid-stream? (This is a separate class from flicker — see
  "Content disappearance" above.)

**Pass** — smooth growth, no stutter of already-rendered lines,
everything that was shown stays shown.
**Fail modes** + diagnosis:

- Stutter / colour flash → re-render flicker. Candidate for M2.
- Content disappearance past ~50 lines → Makepad widget layer
  (CodeView / Markdown flow / PortalList auto-tail). **Not** a
  remend / sanitizer issue. See issues/003 bisect notes.

**Bisect instructions if failing**:

1. Set `USE_REMEND = false` in aichat/main.rs and rebuild. Does the
   symptom persist? If yes, remend is clean.
2. Set `use_code_block_widget: false` in aichat/main.rs (both Markdown
   instances). If the symptom disappears, CodeView is implicated.
3. If still failing after 1+2: the Markdown widget itself mis-lays-out
   long fenced blocks. Upstream Makepad concern.

## P3 — Mixed document pressure test (the compound stressor)

```
写一份技术文档，包含：
1. 一张架构图（mermaid flowchart TD，至少 10 个节点 + classDef）
2. 一段数学推导（2 个 display math 公式 + 3 个 inline math）
3. 一段 200 行的 Rust 代码示例
4. 一张有 5 行 4 列的 GFM 表格
5. 一张 mermaid stateDiagram-v2 状态机

请按 1→5 的顺序生成，每段之间用 --- 分隔
```

**What to watch**:

- Does the mermaid diagram re-layout visibly as the streaming buffer
  grows (nodes jumping around), or does it only render once the fence
  closes? Re-layout-per-token is the clearest re-render-flicker
  symptom — a strong M2 signal.
- Does the code block show a visible "syntax highlight pop" when the
  closing \`\`\` arrives?
- Is the GFM table rendered at all? (issue 002 §A.3 #11 — Makepad
  widget has 8 TODO stubs here, tables render as nothing today.)
- Is overall scrolling smooth?
- Do mermaid labels containing LaTeX (`\mathbb{E}`, `\text{状态}`,
  `t_{transit}`) render as Unicode (𝔼 / 状态 / tₜᵣₐₙₛᵢₜ) or raw
  backslash-source? (rusty-mermaid LaTeX fix, see core/src/latex.rs.)

**Pass (for M1 scope)** — nothing regresses relative to P1. The table
gap (#3) and tables being empty are known and tracked independently.
**Fail** — record which of the five sections misbehaves and how.

## P4 — Thinking / reasoning-block smoke test (skip for Kimi)

Only if the backend is DeepSeek R1 / OpenAI o1 — Kimi doesn't emit
thinking blocks, skip.

```
给我一个复杂决策问题的分析（要在 <think>...</think> 或 "> 引用" 块里展开思考过程），
然后用普通段落给出结论。问题本身是：如何给一个 100 人公司设计代码评审流程
```

**What to watch**: does the blockquote `>` line render correctly as a
quoted block while streaming, or does it pop between "prose with leading
`>` char" and "blockquote"? The M3 event-stream-parser spec notes
`BlockQuote` must be day-one scope; this test is the forcing function.

## Observation hygiene

- **Dual-window comparison**: if you have a pre-M1 aichat binary
  (approx. commit `07c94f12` region), run both windows side-by-side
  with the same prompt. Subjective A/B is more robust than memory.
- **Screen recording**: 30 seconds of video beats 30 seconds of
  eyeballing. Flicker at the per-token level is under 16ms and easy to
  miss; frame-by-frame review catches it.
- **Watch for "eaten" characters**: during streaming, characters that
  were visible one frame and gone the next — a sanitizer / remend
  interaction failure signature, not a flicker. Distinct symptom.

## Decision tree after testing

```
P1 flickers? ──── yes ─→ M1 remend bug. Send repro, extend rules.
    │
    no
    ↓
P2 / P3 stutter or pop visible? ─── yes ─→ re-render flicker.
    │                                       M2 becomes justified.
    │                                       Prioritise mermaid/latex
    │                                       deferred-render at
    │                                       fence-close boundary.
    no
    ↓
P2 long code or P3 mermaid disappears? ─── yes ─→ Makepad widget layer.
    │                                               Bisect: remend off,
    │                                               CodeView off.
    │                                               Tracked in
    │                                               issues/003.
    no
    ↓
M1 alone is sufficient for current product target.
M2 can be deferred; focus on tables + mermaid LaTeX + widget polish.
```

## Current snapshot (2026-04-19)

- P1: **pass** — remend kills misinterpretation flicker as designed.
- P2: **bisect complete — CodeView confirmed as culprit.** With
  `use_code_block_widget: false` (fenced code falls back to Markdown's
  inline fixed-font path), content no longer disappears even at 12 KB+
  buffer / 100+ lines. Parser pipeline (remend / sanitizer / kit) is
  clean. Upstream Makepad work needed to fix CodeView under rapid
  streaming set_text. Workaround for now: keep
  `use_code_block_widget: false` (loses syntax highlighting).
- P3: partially tested — mermaid LaTeX labels now render Unicode
  (fix in rusty-mermaid/crates/core/src/latex.rs). Tables remain empty
  pending Makepad widget work. Mermaid diagrams per-token re-layout
  behaviour not yet characterised.
- P4: not run (current Kimi flow doesn't emit thinking blocks).

Update this snapshot block when test results change.
