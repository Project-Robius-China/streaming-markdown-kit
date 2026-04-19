# Table v2 polish — trigger-test protocol

Companion to `streaming-render-test-protocol.md`. The current (v1) GFM
table renderer in Makepad's `widgets/src/markdown.rs` ships a usable
grid — visible borders, header tint, CJK + Latin, release-mode smooth.
Four v2 polish items are **NOT** implemented and deliberately deferred
until an actual content pattern exposes them as painful. This doc
gives one prompt per polish item so the deferral can be tested
empirically instead of argued theoretically.

How to use: run each P-T prompt against the aichat (or any downstream
consumer) in `cargo run --release`, observe the rendered output
against the "看" criterion, tick "修" or "留" in the checklist at the
bottom.

## P-T1 — Long cell content (> 400 lpx column cap)

```
给我一个表格，对比 5 种主流 TLS 握手协议。要求：
- 列：协议名、完整的 RFC 编号与链接、握手往返次数、加密套件族、关键安全特性完整描述
- "关键安全特性完整描述"列每行写 80 字以上的完整句子，描述该协议对抗中间人攻击、前向保密、0-RTT 数据等的具体机制
- 输出标准 GFM markdown 表格
```

**Trigger**: forces a cell whose measured width exceeds the 400 lpx
column cap.

**看**: the long-description column — does its text overflow past the
right column boundary line? Does it push the whole table wider than
the message bubble?

**Fix if triggered**: change `draw_abs`'s `wrap: false` to `wrap: true`
in the cell-draw loop, and switch row height from `font_size × LINE_HEIGHT_MULT`
to the `laidout_text.size_in_lpxs.height` of the widest cell in that row.

## P-T2 — Column alignment (`:---` / `:---:` / `---:`)

```
给我一个 GFM 表格，展示 5 家云厂商的产品定价，要求：
- 4 列：厂商名（左对齐）、产品（居中）、价格 USD（右对齐）、发布年份（右对齐）
- 分隔行**必须使用 `:---` `:---:` `---:` `---:` 的精确对齐语法**
- 价格至少一位四位数，这样能看出右对齐是否生效
```

**Trigger**: source-markdown has explicit alignment specifiers that
`draw_table` currently discards (`_alignments` bound to `_` in the
Start(Tag::Table) handler).

**看**: are the price digits **末位对齐** (right-aligned)? Is the
"产品" column visually centered? Or does everything left-align?

**Fix if triggered**: capture `alignments: Vec<Alignment>` when
entering Start(Tag::Table); pass to `draw_table`; in the cell-draw
loop, adjust `text_x` inside the cell rect based on
`align[c]` (Alignment::Left / Center / Right / None).

## P-T3 — Inline formatting inside cells (bold / code / link)

```
给我一个表格对比 3 种 Rust 异步 runtime：
- 列：名称、GitHub 链接、**核心 feature**（需要粗体关键词）、安装命令（`代码格式`）
- "名称"列用 **Tokio** / **async-std** / **smol** 格式
- "GitHub 链接"列用 [tokio-rs/tokio](https://github.com/tokio-rs/tokio) 真 markdown 链接
- "核心 feature"列用 **work-stealing** 这类粗体短语 + 普通描述混排
- "安装命令"列用 `cargo add tokio` 这种行内代码
```

**Trigger**: pulldown-cmark emits Start(Strong) / Start(Code) /
Start(Link) events INSIDE a TableCell. The current buffer is
`Vec<Vec<String>>` which flattens these to plain text.

**看**: do cell contents show **bold weight** / monospace `code` font
/ clickable underlined links? Or does everything render as plain
inline text with the formatting markers stripped?

**Fix if triggered**: change cell buffer from `String` to a richer
type — `Vec<(InlineStyle, String)>` or reuse TextFlow's own laidout
text primitive. Draw cell content via iterated `DrawText` calls with
style switches instead of a single `draw_abs`.

## P-T4 — Rounded corners (pure aesthetic)

No special prompt. Look at any existing table; the outer border is
currently 4 flat rectangles (`DrawColor::draw_abs`). True rounded
corners need an SDF rectangle in `draw_table_bg`.

**看**: do the four corners of the outer border look too sharp
against rounded chat bubbles (which are typically radius ~8 px)? Is
it a visual jarring?

**Fix if triggered**: port the `FlowBlockType::Code` SDF-box pattern
from `widgets/src/markdown.rs`'s `draw_block.pixel` shader to
`draw_table_bg`. Replace its 4 flat rects with a single
`Sdf2d::box(... radius)` call.

## P-T5 — Very wide table (10 columns — overflow behavior)

```
生成一张 10 列的 markdown 表格：服务器编号、公网 IP、内网 IP、OS 版本、
CPU 核数、内存 GB、磁盘 GB、所在机房、负责团队、状态。至少 5 行数据。
```

**Trigger**: cumulative column width exceeds the parent turtle's
inner width.

**看**: what does the current `Walk::fixed(total_w, total_h)`
reservation do when `total_w > parent_inner_width`? Does the table
overflow to the right (truncated by PortalList scroll)? Wrap to a
second row of cells? Squeeze?

**Fix if triggered**: this is the most invasive — adding horizontal
scroll inside a PortalList item is non-trivial in Makepad. Cheaper:
soft-cap `total_w = min(sum(col_w), parent_inner_width)` and
proportionally shrink all columns.

## Test tracker

Run the prompts, observe, fill in:

| Test | Observed | 修 or 留 |
|---|---|---|
| P-T1 long cell | | |
| P-T2 alignment | | |
| P-T3 inline fmt in cell | | |
| P-T4 rounded corners | | |
| P-T5 10-col wide | | |

Only items marked "修" deserve a separate fix patch. "留" items get
recorded here so they aren't re-proposed without evidence.

## Current snapshot (2026-04-20)

Not yet tested. v2 items all deferred by default until this protocol
actually surfaces them.
