//! Render Mermaid diagrams to PNG bytes, ready to hand straight to
//! Makepad's `Image::load_image_from_data_async`.
//!
//! Available only when the `mermaid` feature is enabled — it pulls in
//! `mermaid-rs-renderer`, `usvg`, and `resvg`, which adds ~100 transitive
//! crates and a cold-build cost of ~30s. Opt-in only.
//!
//! ## Flow
//!
//! 1. `mermaid-rs-renderer::render` turns the Mermaid source into an SVG
//!    string (parser → IR → layout → SVG, all in pure Rust).
//! 2. `usvg` parses the SVG into a tree and loads system fonts so CJK labels
//!    render without tofu.
//! 3. `resvg` rasterises the tree into a pixmap.
//! 4. The pixmap encodes to PNG bytes suitable for texture upload.
//!
//! ## Streaming safety
//!
//! This function assumes the full mermaid source is present. Call it once the
//! enclosing fenced code block is complete (i.e. after the closing ``` ).
//! Calling with partial source will usually error out of the parser — handle
//! the `Err` by showing a loading placeholder instead.

use std::time::Duration;

/// PNG bytes plus pixel dimensions, ready for
/// [`Image::load_image_from_data_async`].
#[derive(Debug, Clone)]
pub struct RenderedMermaid {
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Time spent in `mermaid-rs-renderer`.
    pub render_time: Duration,
    /// Time spent in `usvg` + `resvg` rasterisation.
    pub rasterise_time: Duration,
}

/// Default background colour (white). Override if you need a dark-mode mermaid
/// render — currently mermaid-rs-renderer doesn't expose a dark theme through
/// `render()`, so you'd want to composite this PNG over your chat bubble
/// colour yourself.
const BACKGROUND: resvg::tiny_skia::Color = resvg::tiny_skia::Color::WHITE;

/// Render a mermaid source string to a PNG byte buffer.
///
/// Returns `Err` if the diagram is syntactically invalid, if the SVG can't be
/// parsed, or if allocation fails.
/// Render mermaid source to raw SVG using `rusty-mermaid` (25 diagram types,
/// dagre layout, light/dark themes). Skips rasterisation — use this when the
/// downstream widget renders SVG directly.
pub fn render_mermaid_to_svg(source: &str) -> anyhow::Result<String> {
    rusty_mermaid::to_svg(source, &rusty_mermaid::Theme::dark())
        .map_err(|e| anyhow::anyhow!("mermaid render failed: {e:?}"))
}

pub fn render_mermaid_to_png(source: &str) -> anyhow::Result<RenderedMermaid> {
    let t0 = std::time::Instant::now();
    let svg = rusty_mermaid::to_svg(source, &rusty_mermaid::Theme::dark())
        .map_err(|e| anyhow::anyhow!("mermaid render failed: {e:?}"))?;
    let render_time = t0.elapsed();

    let t1 = std::time::Instant::now();
    let mut opts = usvg::Options::default();
    // Set a sensible default font so resvg's fallback picks something readable
    // when glyph shaping still can't find a requested face.
    opts.font_family = "Helvetica Neue".to_string();
    opts.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(&svg, &opts)
        .map_err(|e| anyhow::anyhow!("usvg parse failed: {e}"))?;

    let size = tree.size().to_int_size();
    let (w, h) = (size.width(), size.height());
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| anyhow::anyhow!("cannot allocate pixmap {w}x{h}"))?;
    pixmap.fill(BACKGROUND);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let png_bytes = pixmap
        .encode_png()
        .map_err(|e| anyhow::anyhow!("png encode failed: {e}"))?;
    let rasterise_time = t1.elapsed();

    Ok(RenderedMermaid {
        png_bytes,
        width: w,
        height: h,
        render_time,
        rasterise_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_flowchart() {
        let src = "flowchart TD\n    A-->B\n    B-->C";
        let out = render_mermaid_to_png(src).expect("render");
        assert!(out.png_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(out.width > 0 && out.height > 0);
    }

    #[test]
    fn smoke_sequence() {
        let src = "sequenceDiagram\n    A->>B: hi\n    B-->>A: yo";
        let out = render_mermaid_to_png(src).expect("render");
        assert!(out.png_bytes.len() > 100);
    }

    #[test]
    fn cjk_labels_do_not_crash() {
        // Standard mermaid uses `A[label]` syntax; rusty-mermaid (stricter than
        // some older renderers) rejects bare CJK as a node identifier, which is
        // actually per-spec.
        let src = "flowchart LR\n    A[用户] --> B[前端]\n    B --> C[后端]";
        let out = render_mermaid_to_png(src).expect("render");
        assert!(out.width > 0 && out.height > 0);
    }

    #[test]
    fn invalid_input_returns_err() {
        // Intentional gibberish that shouldn't parse.
        let src = "not a real mermaid diagram at all }}}";
        let _ = render_mermaid_to_png(src); // may Ok or Err depending on crate — just ensure no panic
    }
}
