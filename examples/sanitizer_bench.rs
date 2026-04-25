//! Rough throughput check. Not criterion-grade, but enough to know the function
//! is nowhere near a bottleneck at streaming cadences.
//!
//! Run: `cargo run --example bench --release`

use std::time::Instant;
use streaming_markdown_kit::sanitize_streaming_markdown;

fn make_doc(n_blocks: usize) -> String {
    let mut s = String::with_capacity(n_blocks * 200);
    for i in 0..n_blocks {
        match i % 4 {
            0 => s.push_str(&format!(
                "# Heading {i}\n\nSome prose with **bold** and *italic*.\n\n"
            )),
            1 => s.push_str("```rust\nfn main() { println!(\"{}\", 42); }\n```\n\n"),
            2 => s.push_str("$$\n\\sum_{i=0}^{n} x_i^2 = y\n$$\n\n"),
            _ => s.push_str("| a | b | c |\n| - | - | - |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |\n\n"),
        }
    }
    s
}

fn main() {
    for size_kb in [1, 8, 64] {
        let doc = make_doc(size_kb * 5);
        let bytes = doc.len();

        // Cold: single call
        let start = Instant::now();
        let _ = sanitize_streaming_markdown(&doc);
        let cold = start.elapsed();

        // Hot: 10_000 calls
        let iters = 10_000;
        let start = Instant::now();
        for _ in 0..iters {
            std::hint::black_box(sanitize_streaming_markdown(std::hint::black_box(&doc)));
        }
        let hot_total = start.elapsed();
        let hot_per = hot_total / iters as u32;
        let throughput_mb = (bytes as f64 * iters as f64) / hot_total.as_secs_f64() / 1_048_576.0;

        println!(
            "doc={:>5} bytes    cold={:>7.2}µs    hot={:>5.2}µs/call    {:>7.1} MB/s",
            bytes,
            cold.as_secs_f64() * 1e6,
            hot_per.as_secs_f64() * 1e6,
            throughput_mb
        );
    }

    // Incremental streaming simulation: 120 char/s LLM for 10 seconds = 1200 chars
    // streamed with sanitize at every 6-char chunk (typical). Measure.
    let full = make_doc(20);
    let chunk_sz = 6;
    let start = Instant::now();
    let mut pos = 0;
    while pos < full.len() {
        let end = (pos + chunk_sz).min(full.len());
        if !full.is_char_boundary(end) {
            pos += 1;
            continue;
        }
        let _ = sanitize_streaming_markdown(&full[..end]);
        pos = end;
    }
    let streaming_total = start.elapsed();
    println!(
        "\nstreaming simulation: {} bytes in {}-byte chunks = {} calls, total {:>6.2}ms",
        full.len(),
        chunk_sz,
        full.len() / chunk_sz,
        streaming_total.as_secs_f64() * 1e3
    );
}
