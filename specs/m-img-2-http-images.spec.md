spec: task
name: "M-img-2 HTTP image fetch via Makepad native network stack"
tags: [markdown, image, http, cross-platform]
depends: [markdown-image-rendering]
estimate: 1d
---

## Intent

Extend M-img-1 so that `![alt](https://host/path.png)` — and `http://` —
actually fetches, decodes, and renders inside the Markdown widget,
replacing the current "deferred placeholder" behaviour. Use Makepad's
native `cx.http_request` + `Event::NetworkResponses` path (already used
by `draw/src/image_cache.rs`) so the feature is platform-dispatched
across macOS, iOS, Linux, Windows, Android, and WASM with zero new
crate dependencies.

## Decisions

- HTTP client: `cx.http_request(request_id, HttpRequest::new(url, HttpMethod::GET))` from `makepad_platform` / `makepad_network`. No reqwest, no ureq, no Tokio needed — the event loop already delivers responses via `Event::NetworkResponses`.
- Event hookup: Markdown widget's existing `handle_event` gains an `Event::NetworkResponses(e)` arm alongside `Event::Actions(..)`. Mirrors the pattern in `widgets/src/image.rs:263`.
- Request id: `LiveId` hashed from the cache key (URL hash from M-img-1). Collision risk is acceptable for this widget scope.
- Concurrency / dedup: a widget-local `pending_http: HashMap<LiveId, u64>` maps request-id → cache-key. Before sending, check if the cache key is already in `pending_http` → skip duplicate fetch. No separate `HashSet` needed.
- Cache: reuse M-img-1's `image_cache: HashMap<u64, ImageCacheEntry>` + LRU. On successful decode the entry is inserted the same way as M-img-1's local path — same byte cap (16 MiB), same eviction rule.
- Failure policy: HTTP error, non-2xx status, empty body, decode error → `log::warn!` with identifiers (`"http error"`, `"http status <code>"`, `"empty body"`, `"decode error"`), placeholder stays (🖼 + alt). Failed URLs NOT cached, so the next `set_text` retries.
- Body size cap: reject any response with `body.len() > IMAGE_HTTP_BODY_MAX = 32 * 1024 * 1024` before decode. `log::warn!("markdown image: body too large ({N} bytes)")`, placeholder stays.
- Scheme gating: `ImageSrc::Http(String)` variant already reserved by M-img-1 (`parse_image_src` currently routes all `http(s)://` URLs to the deferred-placeholder branch). This spec flips that branch from warn+placeholder to fetch-and-fill.
- Placeholder during load: between Start(Tag::Image) and response arrival, render 🖼 + alt (same as the error placeholder) so the layout doesn't jump. On successful decode, `cx.redraw_all()` re-renders with the `inline_image` widget.
- Timeout: rely on Makepad platform default (`HttpRequest::new` uses the OS stack's defaults — e.g. NSURLRequest on macOS/iOS, ~60s). No custom timeout in v1.
- Redirects: rely on Makepad platform default (follows 3xx per OS HTTP stack). No explicit control in v1.
- WASM: same code path works — `cx.http_request` on WASM dispatches through `fetch()`. CORS is the consumer's problem (document it).

## Boundaries

### Allowed Changes

- `robius/makepad/widgets/src/markdown.rs` — extend `image_cache` + add HTTP branches
- `robius/makepad/examples/aichat/src/main.rs` — only if existing event handling needs a delegation tweak
- `streaming-markdown-kit/specs/markdown-image-rendering.spec.md` — update the 4 HTTP scenarios' status from "deferred to M-img-2" to "in scope for M-img-2"

### Forbidden

- Do not add `reqwest`, `ureq`, `hyper`, `tokio`, or any other HTTP/async runtime crate to `makepad/widgets/Cargo.toml` or `aichat/Cargo.toml`.
- Do not introduce a thread or `std::thread::spawn` for HTTP work — Makepad's network stack already handles the OS-level async.
- Do not cache failed responses.
- Do not persist the HTTP cache to disk in v1 — memory-only, widget-local, same as M-img-1.
- Do not emit more than one `log::warn!` per failed URL per widget lifetime. Dedup via a `warned_urls: HashSet<u64>` field if needed.

## Out of Scope

- ETag / Cache-Control respect
- Disk-persistent cache
- Configurable timeout / retry / backoff
- Concurrent request limit (reliance on OS stack's fairness)
- Background prefetch during streaming
- SVG via HTTP (M-img-1 already doesn't decode SVG; spec extension if wanted)
- HEIC / AVIF decode (same as M-img-1 — unsupported by Makepad's native decoder)
- Auth headers / cookies

## Completion Criteria

Scenario: HTTP 200 PNG decodes and renders
  Test: test_http_200_png_decodes
  Level: integration
  Test Double: loopback tiny-http server
  Given the Markdown widget receives an Image event with url "http://127.0.0.1:<port>/tiny.png"
  And the local test server returns status 200 with a valid 10x10 PNG body
  When the widget's handle_event processes the NetworkResponses event
  Then the cache key for that URL has a populated ImageCacheEntry
  And a redraw produces an inline_image widget (not the 🖼 placeholder)

Scenario: HTTP 404 falls back to placeholder
  Test: test_http_404_falls_back
  Level: integration
  Test Double: loopback tiny-http server
  Given the Markdown widget receives an Image event with url "http://127.0.0.1:<port>/missing.png"
  And the local test server returns status 404
  When the widget's handle_event processes the NetworkResponses event
  Then log::warn! fires with message containing "http status 404"
  And the cache key is NOT inserted into image_cache
  And subsequent set_text with the same URL re-issues the HTTP request

Scenario: HTTP connect error falls back
  Test: test_http_connect_error_falls_back
  Level: integration
  Test Double: closed localhost port (no server)
  Given the Markdown widget receives an Image event with url "http://127.0.0.1:1/unreachable.png"
  When the OS stack emits NetworkResponse::HttpError for that request_id
  Then log::warn! fires with message containing "http error"
  And the cache key is NOT inserted into image_cache

Scenario: Same URL across two events dedups to one request
  Test: test_http_same_url_dedups
  Level: unit
  Targets: pending_http dedup logic (no real network)
  Given the Markdown widget handles two Start(Tag::Image) events with identical url "http://example.test/a.png" in a single render pass
  When the widget attempts to issue HTTP requests for both
  Then only one entry is added to pending_http
  And only one cx.http_request call is made

Scenario: Oversize body rejected before decode
  Test: test_http_oversize_body_rejected
  Level: unit
  Targets: body-size-cap branch (synthetic HttpResponse struct, no real network)
  Given a response whose body.len() > IMAGE_HTTP_BODY_MAX (32 MiB)
  When the NetworkResponses arm inspects the body
  Then log::warn! fires with message containing "body too large"
  And the decode path is NOT invoked
  And the cache key is NOT inserted

Scenario: Cache hit on second set_text skips HTTP
  Test: test_http_cache_hit_skips_fetch
  Level: unit
  Targets: cache lookup short-circuit (pre-populated image_cache, no network)
  Given a successful HTTP fetch has populated image_cache for url "http://host/x.png"
  When the widget runs set_text again with the same url
  Then no new request_id is added to pending_http
  And no cx.http_request call is made
  And the existing cached texture is reused

Scenario: Non-http scheme still routes to M-img-1 Invalid branch
  Test: test_http_non_http_scheme_unchanged
  Level: unit
  Targets: parse_image_src scheme detection
  Given a Start(Tag::Image) event with url "ftp://host/a.png"
  When parse_image_src is called
  Then the result is ImageSrc::Invalid
  And M-img-1's existing "unknown scheme" warning fires
  And no HTTP request is issued

Scenario: Warn dedup — same failing URL warns once per widget lifetime
  Test: test_http_warn_dedup
  Level: unit
  Targets: warned_urls HashSet insertion
  Given two failed HTTP fetches for the same url "http://host/fail.png" in the same widget instance
  When both failures are processed
  Then log::warn! fires exactly once for that url
  And warned_urls contains one entry for that URL's cache key

Scenario: HTTPS URL works identically to HTTP
  Test: test_https_200_png_decodes
  Level: integration
  Test Double: loopback TLS server (self-signed; platform-specific, may be manual-verification-only)
  Given the Markdown widget receives an Image event with url "https://127.0.0.1:<port>/tiny.png"
  And the local test server serves TLS with a valid 10x10 PNG body and status 200
  When the NetworkResponses arm processes the response
  Then the cache is populated identically to the HTTP case
