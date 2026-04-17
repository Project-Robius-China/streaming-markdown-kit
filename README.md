# streaming-markdown-kit

Drop-in streaming-markdown helpers for **Makepad 2.0** apps (LLM chat UIs, live
docs, anywhere you're streaming markdown from a token source).

Gets you three things without patching Makepad:

1. **Flicker-free rendering of half-written markdown** — half-open code fences,
   unpaired `$$`, and incomplete tables are trimmed from each frame's body so
   your layout doesn't jump when the closing token finally arrives. Pass-through
   for complete input.
2. **Blinking cursor** at the end of the stream — implemented as a literal `▋`
   character that rides Makepad's built-in `animated_chars` GPU fade window. No
   custom widget, no animator.
3. **CJK + emoji support in code blocks** — bundles LXGW Wen Kai so Chinese/
   Japanese/Korean characters render inside code (Makepad's default `font_code`
   is Latin-only).

Zero Makepad source changes required. Works against stock `makepad-widgets`.

## Install

```toml
[dependencies]
streaming-markdown-kit = { path = "../streaming-markdown-kit" } # or git = "..."
```

## Rust side

```rust
use streaming_markdown_kit::{streaming_display, inline_code_options};

// Each time a new chunk arrives from your LLM, rebuild the body:
let body = streaming_display(&accumulated_text, inline_code_options());
markdown.set_text(cx, &body);
markdown.start_streaming_animation();

// When streaming ends, pass the raw text (no cursor, no sanitise):
markdown.set_text(cx, &final_text);
markdown.stop_streaming_animation();
```

The two knobs:

- `inline_code_options()` — use when your `Markdown` DSL has
  `use_code_block_widget: false` (inline `<code>` drawn with `draw_text`, code
  fades in char-by-char like regular prose). Recommended default.
- `SanitizeOptions::default()` — use when `use_code_block_widget: true`
  (a real `CodeView` sub-widget renders the code). Trims half-fences to keep
  syntax highlighting from thrashing on partial input.

## Fonts: one-time copy step

Makepad 2.0's `crate_resource(..)` can only cross-reference crates that
themselves run `script_mod!` / `script_eval!` at startup — kit doesn't, so
you can't spell `crate_resource("streaming_markdown_kit:resources/…")`.
Until upstream lifts that limitation, copy the two fonts into your own
crate's `resources/` directory:

```bash
cp $(cargo pkgid streaming-markdown-kit | sed -E 's|^.+file://||; s|#.+$||')/resources/*.ttf ./resources/
# or just:
cp path/to/streaming-markdown-kit/resources/*.ttf ./resources/
```

(Or automate in `build.rs` — the files are ~18 MB combined, dominated by
LXGW Wen Kai.)

## DSL side — paste into your `script_mod!`

Override the theme's `font_code` so every widget that reads it (Markdown's
inline `<code>`, CodeView, terminals, etc.) gets CJK fallback for free. Paste
the block below into your `script_mod!` near the top, before any widget
definitions:

```splash
// TextStyle / FontFamily / FontMember come from `mod.text.*`; the
// `crate_resource(..)` helper from `mod.res.*` — add both `use`s at the top
// of your `script_mod!` if you haven't already.
use mod.text.*
use mod.res.*

// If you use the `light` theme instead of `dark`, change `mod.themes.dark`.
mod.themes.dark = mod.themes.dark {
    font_code: TextStyle {
        font_size: theme.font_size_code
        font_family: FontFamily {
            latin   := FontMember { res: crate_resource("self:resources/LiberationMono-Regular.ttf") asc: 0.0 desc: 0.0 }
            chinese := FontMember { res: crate_resource("self:resources/LXGWWenKaiRegular.ttf")      asc: 0.0 desc: 0.0 }
        }
        line_spacing: 1.35
    }
}
```

After this, any `<Markdown>` you already have gains CJK-capable code blocks.
No need to rename or swap the widget.

## Crate size

≈ 18 MB, almost entirely LXGW Wen Kai Regular. Cargo caches it once per
machine. If that's unacceptable for your distribution, fork the crate and strip
`resources/LXGWWenKaiRegular.ttf`; the kit degrades to Latin-only code font.

## What it does NOT do

- **LLM backend wiring.** Makepad's `makepad-ai` crate has its own set of
  issues (SSE frame boundary handling, strict JSON schema) — those are API
  client problems, not markdown rendering problems, and they're out of scope
  for this crate.
- **Custom cursor widget.** The `▋` character is deliberately not a widget —
  going through the text path is what makes it free-to-animate. If you need a
  blinking glyph that's visually distinct from the text style, use a custom
  Unicode character and override `draw_text` for that codepoint, or draw a
  sibling cursor view yourself.

## License

- Source code: MIT OR Apache-2.0.
- `resources/LXGWWenKaiRegular.ttf`: SIL Open Font License 1.1 (redistributed
  per OFL §2).
- `resources/LiberationMono-Regular.ttf`: SIL Open Font License 1.1.
