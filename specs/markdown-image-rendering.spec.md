spec: task
name: "Markdown image rendering (local + network + cache + errors)"
tags: [markdown, image, rendering, upstream-makepad]
estimate: 3d
---

## Intent

Replace the current inline-placeholder image rendering in Makepad's
Markdown widget (which shows a literal `🖼 <alt text>` string wherever
`Start(Tag::Image)` fires) with real image display: decode the bytes at
the URL, instantiate a sized image widget inline in the TextFlow, cache
the decoded texture so streaming re-parses don't re-fetch, and degrade
gracefully with a typed placeholder when the image can't load. Context:
`issues/002-streaming-render-state-machine-and-render-gap-audit.md §A.3 #10`
is the standing gap; user reported during aichat testing that LLMs
frequently emit `![alt](/path/to/file.png)` and it renders as a useless
`🖼` placeholder.

## Decisions

- Implementation lives in Makepad widgets, not this crate: edits go to
  `widgets/src/markdown.rs` only; no streaming-markdown-kit changes.
- Decode via the `image` crate (already transitively present in makepad-widgets
  under `features = ["maps"]`). Supported formats: PNG, JPEG, WebP, GIF (still
  frame only), BMP. No HEIC, no AVIF, no animated GIF.
- Inline image widget: instantiate Makepad's built-in `Image` widget as a
  `tf.item_with` template named `inline_image` with a `DrawImage` shader.
- URL parsing via a small helper `fn parse_image_src(url: &str) -> ImageSrc`
  that classifies into `ImageSrc::Local(PathBuf)` / `ImageSrc::File(PathBuf)`
  (for `file://`) / `ImageSrc::Http(String)` / `ImageSrc::Data(Vec<u8>, Mime)` /
  `ImageSrc::Invalid`.
- Local path resolution: absolute paths used as-is; relative paths resolved
  against `std::env::current_dir()` at load time (aichat is a desktop app so
  cwd is well-defined).
- Network fetch (HTTP/HTTPS): **implemented in M-img-2** via Makepad's native
  `cx.http_request` + `Event::NetworkResponses` pipeline — zero new crate
  dependencies, platform-dispatched (macOS/iOS/Linux/Windows/Android/WASM).
  See `m-img-2-http-images.spec.md` for decisions (dedup via
  `pending_http: HashMap<LiveId, u64>`, 32 MiB body cap, per-URL warn dedup,
  same 16 MiB decoded-texture cache as the local path). Timeout + redirects
  inherit the OS stack's defaults; no custom tokio/ureq runtime.
- Texture cache: widget-local `HashMap<u64, ImageCacheEntry>` keyed by
  `hash(url_str)`. Entry holds `Rc<TextureRef>` (or Image's equivalent) + decoded
  intrinsic `(w, h)`. LRU-capped at `IMAGE_CACHE_MAX_BYTES = 16 * 1024 * 1024`
  per Markdown instance.
- Display sizing: respect intrinsic dimensions up to `IMAGE_MAX_DISPLAY_W =
  480` logical pixels; scale down proportionally keeping aspect ratio. Larger
  than-max images decode full-resolution then render scaled (no thumbnail-on-
  decode optimization in v1).
- Placeholder for any non-loaded / errored state: exactly the current
  `🖼 <alt or url>` inline text. The placeholder path is shared across all
  error types; the specific error is surfaced only via a hover tooltip
  ("file not found", "decode error", "timeout"). v1 does not implement the
  tooltip — the placeholder simply shows `🖼` and the alt/url. Error type
  must be logged to `log::warn!` so developers can diagnose.
- Streaming stability: each `set_text` re-parse emits `Start(Tag::Image)` for
  every image in the buffer. The widget must NOT re-fetch on each re-parse —
  the cache lookup by url-hash short-circuits. Only a never-before-seen URL
  triggers a load.
- Data URLs (`data:image/png;base64,...`) decoded synchronously; failure
  (malformed base64 / unsupported mime) falls back to placeholder.
- No markdown extended sizing syntax (`![alt](url =WxH)`) in v1. Sizes come
  only from intrinsic dimensions.
- No clickability in v1 — image does NOT emit a widget action on click. A
  future revision may add click-to-expand / click-to-open-full-size.

## Boundaries

### Allowed Changes

- `widgets/src/markdown.rs`
- `widgets/Cargo.toml` (only to enable/confirm existing dependency features;
  no new dependencies)

### Forbidden

- Do not add a new production dependency to `widgets/Cargo.toml`. The
  `image` crate must already be reachable via `makepad-widgets[maps]`;
  if not, fall back to skipping local-image decode and emit the
  current `🖼` placeholder plus a compile-time feature-gate note.
- Do not add async HTTP machinery that requires a new crate (e.g.
  reqwest, hyper). HTTP support is conditional on `ureq` already being
  reachable OR is explicitly disabled in v1 (see Decision).
- Do not block the UI thread on network I/O under any circumstance.
  All HTTP fetches are async; local-fs reads under 1 MB may be
  synchronous (measured < 5ms on warm NVMe) but larger local files
  must be moved to a background thread.
- Do not modify behavior of fenced code blocks, mermaid blocks, tables,
  or any other Markdown widget code path. Only the `Start(Tag::Image)` /
  `End(TagEnd::Image)` arms and associated state.
- Do not emit platform-specific code without `cfg` gates. Image decode
  and path resolution must compile on macOS / Linux / Windows / iOS /
  Android / WASM targets.
- Do not remove the existing `🖼 <alt>` placeholder fallback. It remains
  the single error-state UI.

## Out of Scope

- Animated GIF / APNG playback (static first-frame only).
- AVIF / HEIC support.
- Clickable images (click → open in external viewer or zoom modal).
- Extended markdown sizing syntax `![alt](url =WxH)`.
- Image cropping / rounded corners on the image frame.
- Right-click context menu / save-as / copy-image-url.
- Smart thumbnail decode (render a reduced-size texture for very large images).
- HTTP response caching to disk (memory-only cache in v1).
- Content-Security-Policy / URL allowlist (app may add this externally).
- Mobile-specific adjustments (responsive width, touch-to-expand): covered
  by a separate future `M-mobile-*` spec.
- DeepSeek thinking-block images (same as issue 003 gap).

## SVG support

Status: covered by M-img-3 (see widgets/src/markdown.rs, 2026-04 onward).
SVG rendering uses Makepad's native `makepad_svg::parse::parse_svg` +
`DrawSvg::render_to_rect` stack — zero external deps, no rasterization
crate. Works for local paths, `file://` URLs, data URLs
(`data:image/svg+xml;base64,...`), and HTTP(S) URLs through the same
`decode_and_cache_bytes` dispatch that handles PNG/JPEG/WebP. SVG bytes
up to 4 MiB are accepted; larger inputs are rejected with a `log::warn!`
and render the `🖼` placeholder. Malformed XML / non-UTF-8 / empty-root
outcomes fall back to the same placeholder path. Cache accounting
charges raw source bytes (not parsed-AST memory). Known limitations
(v1): SVG animations (`<animate>`, `<animateTransform>`) render as the
t=0 frozen frame — the `InlineSvg` widget does not drive `next_frame`;
no CSS stylesheets (`<style>` content ignored); no external image
references (`<image xlink:href="...">`); no `<text>`-on-path; no
scripting; no filters beyond a single feGaussianBlur/feOffset/feFlood
drop-shadow assembled at parse time. See `libs/svg/` source and
`examples/vector/` for the coverage baseline.

## Completion Criteria

Scenario: Local absolute PNG path renders the decoded image
  Test: test_image_local_abs_png_decodes
  Given a PNG file at `/tmp/agent-spec-test-fixtures/cat.png` with intrinsic size 200x150
  When the markdown `![kitten](/tmp/agent-spec-test-fixtures/cat.png)` is streamed in
  Then an `inline_image` template is instantiated
  And the image's draw rect has width 200 and height 150 lpx
  And the cache contains exactly one entry keyed on the url hash

Scenario: Local absolute JPEG path renders at max-width cap with aspect ratio preserved
  Test: test_image_local_jpeg_scaled_to_cap
  Given a JPEG file at `/tmp/agent-spec-test-fixtures/wide.jpg` with intrinsic size 1920x1080
  When `![photo](/tmp/agent-spec-test-fixtures/wide.jpg)` is rendered
  Then the image draws at width 480 lpx (the `IMAGE_MAX_DISPLAY_W` cap)
  And the image draws at height 270 lpx (aspect-ratio-preserved: 480 * 1080 / 1920)

Scenario: Relative local path resolves against cwd
  Test: test_image_relative_path_against_cwd
  Given the process `cwd` is set to `/tmp/agent-spec-test-fixtures`
  And a PNG file `diagram.png` exists in that directory
  When markdown `![d](./diagram.png)` is rendered
  Then the `inline_image` is instantiated with decoded bytes of diagram.png

Scenario: file:// URL renders identically to raw local path
  Test: test_image_file_scheme_equivalent_to_local_path
  Given a PNG file at `/tmp/agent-spec-test-fixtures/icon.png`
  When `![i](file:///tmp/agent-spec-test-fixtures/icon.png)` is rendered
  Then the cached entry and rendered dimensions match the raw-path case
  And only one decode occurred (not two) when both paths appear in one document

Scenario: data URL base64 PNG decodes and renders inline
  Test: test_image_data_url_base64_png
  Given the markdown `![dot](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==)`
  When the widget renders the buffer
  Then the `inline_image` template is instantiated at 1x1 lpx
  And no file-system or network IO is performed during the decode

Scenario: Missing local file falls back to `🖼 alt-text` placeholder
  Test: test_image_missing_file_falls_back_to_placeholder
  Given no file exists at `/tmp/does/not/exist.png`
  When markdown `![broken](/tmp/does/not/exist.png)` is rendered
  Then the flow contains the literal text `🖼 broken`
  And no `inline_image` template is instantiated
  And a `log::warn!` record is emitted with message containing "file not found"

Scenario: Unsupported format falls back to placeholder with error log
  Test: test_image_unsupported_format_falls_back
  Given a file at `/tmp/agent-spec-test-fixtures/doc.heic` exists but is HEIC-encoded
  When `![photo](/tmp/agent-spec-test-fixtures/doc.heic)` is rendered
  Then the flow contains `🖼 photo`
  And no `inline_image` is instantiated
  And a `log::warn!` with "decode error" or "unsupported format" is emitted

Scenario: Corrupt bytes in a supported format fall back to placeholder
  Test: test_image_corrupt_bytes_falls_back
  Given a file at `/tmp/agent-spec-test-fixtures/bad.png` exists with the first 4 bytes replaced by zeros (breaking the PNG signature)
  When markdown `![x](/tmp/agent-spec-test-fixtures/bad.png)` is rendered
  Then the flow contains `🖼 x`
  And a `log::warn!` record with "decode error" is emitted

Scenario: Repeated render with identical URL uses cache, no re-decode
  Test: test_image_cache_hit_on_repeated_set_text
  Given a Markdown widget that has already rendered `![a](/tmp/agent-spec-test-fixtures/cat.png)` once and populated its cache
  When `set_text` is called again with the same buffer
  Then the decode function is invoked zero additional times
  And the rendered draw rect is unchanged

Scenario: Mid-stream incomplete image syntax does not fetch
  Test: test_image_incomplete_syntax_defers_load
  Given the streaming buffer ends with the half-typed `![alt](/tmp/` (no closing paren)
  When the Markdown widget re-parses this buffer
  Then pulldown-cmark emits no `Start(Tag::Image)` event
  And the decode function is not invoked
  And the flow shows the literal half-typed source text (handled by pulldown's tolerant recovery)

Scenario: LRU eviction keeps cache under byte cap
  Test: test_image_cache_lru_evicts_over_cap
  Given `IMAGE_CACHE_MAX_BYTES = 1048576` (override to 1 MB for this test)
  And the widget has cached three 512x512 PNG RGBA entries totalling ~3 MB of raw bytes
  When a fourth image at a new URL is decoded and added
  Then exactly one cache entry is evicted (the least-recently-used)
  And the cache byte-size is below `IMAGE_CACHE_MAX_BYTES`

Scenario: Two images with different URLs and the same bytes decode independently
  Test: test_image_same_bytes_different_url_no_alias
  Given two PNG files at distinct paths but with byte-identical content
  When markdown renders both `![a](/tmp/.../one.png)` and `![b](/tmp/.../two.png)`
  Then the cache contains two entries (keyed by URL hash, not content hash)
  And neither image borrows the other's texture handle

Scenario: HTTP URL placeholder behavior when feature is disabled
  Status: obsolete — in scope for M-img-2, see `m-img-2-http-images.spec.md`.
  The feature-flag / "no ureq" framing is superseded by Makepad's native
  `cx.http_request` stack which has no compile-time gate.
  Test: test_image_http_url_without_feature_shows_placeholder
  Given the widget build has HTTP image support disabled (no `ureq` dependency)
  When markdown `![remote](https://example.com/foo.png)` is rendered
  Then the flow contains `🖼 remote`
  And a single `log::info!` record with "http image support not compiled" is emitted once per widget lifetime

Scenario: HTTP URL renders asynchronously when feature enabled (critical)
  Status: in scope for M-img-2, see `m-img-2-http-images.spec.md` §
  "HTTP 200 PNG decodes and renders". Implementation uses Makepad's native
  `cx.http_request` (no tokio runtime needed).
  Test: test_image_http_url_renders_async_when_enabled
  Tags: critical
  Given the widget has HTTP image support enabled and a running tokio runtime
  And a local test server serves `http://127.0.0.1:PORT/foo.png` returning a valid 100x100 PNG
  When markdown `![remote](http://127.0.0.1:PORT/foo.png)` is rendered
  Then the first draw shows `🖼 remote` while the fetch is in flight
  And within 500 ms after fetch completion a subsequent redraw shows an `inline_image` of size 100x100 lpx
  And the cache then contains one entry keyed on the full URL

Scenario: HTTP 404 falls back to placeholder
  Status: in scope for M-img-2, see `m-img-2-http-images.spec.md` §
  "HTTP 404 falls back to placeholder". Warning message in M-img-2 is
  "http status 404" (no new `HTTP 404` literal — same information).
  Test: test_image_http_404_falls_back
  Given the widget has HTTP enabled and the test server returns 404 for the URL
  When markdown `![x](http://127.0.0.1:PORT/missing.png)` is rendered
  Then after the fetch resolves, the flow shows `🖼 x`
  And a `log::warn!` record contains "HTTP 404"
  And the failed URL is NOT cached (so a future retry would re-fetch)

Scenario: HTTP timeout falls back to placeholder within 5s
  Status: partially in scope for M-img-2. Makepad's native stack inherits
  OS-default timeouts (~60 s on macOS NSURLSession); the explicit 5 s cap
  is out-of-scope per `m-img-2-http-images.spec.md` "Out of Scope" /
  "Configurable timeout". Connect-error / HttpError paths ARE implemented
  and render the placeholder + emit `warning!("http error ...")`.
  Test: test_image_http_timeout_falls_back
  Given the widget has HTTP enabled and the test server stalls indefinitely on the URL
  When markdown `![slow](http://127.0.0.1:PORT/stall.png)` is rendered
  Then within 5500 ms after the request starts, the flow shows `🖼 slow`
  And a `log::warn!` record contains "timeout"
  And the failed URL is NOT cached

Scenario: Non-existent scheme produces placeholder without fetch attempt
  Test: test_image_unknown_scheme_placeholder
  Given a markdown url with an unrecognised scheme such as `gopher://old.server/foo.png`
  When the widget renders it
  Then the flow contains `🖼 alt-or-url`
  And no HTTP, file-system, or data-URL decode is invoked

Scenario: Draw performance — 10-image document renders under budget on cold start
  Test: test_image_cold_decode_10_local_under_budget
  Mode: optimize
  Given a markdown document with 10 distinct local PNG references (100x100 each)
  When `set_text` is called on a fresh widget and the first draw completes
  Then the total wall-clock time from set_text entry to draw return is under 200 ms on the reference macOS arm64 release-build target

Scenario: Placeholder renders exactly once per image per set_text call
  Test: test_image_placeholder_rendered_once_per_set_text
  Given a markdown document with two image references that both fall back to placeholder (files missing)
  When set_text is called
  Then exactly two `🖼 ...` sequences appear in the flow (not four, not zero)
  And the widget does not enter an event-re-emission loop

Scenario: Image followed by text flows correctly on the next line or continuation
  Test: test_image_followed_by_paragraph_text_layout
  Given markdown `![logo](/tmp/agent-spec-test-fixtures/logo.png) some trailing text`
  When the widget renders it
  Then the inline_image widget is placed
  And the "some trailing text" string is laid out on a TextFlow row position that starts at or after the image's right edge, either on the same row (y-position within image height) or on a row below the image (y-position at least image_height + line_spacing)
