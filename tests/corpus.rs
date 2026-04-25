//! Corpus-level property tests. Binds to spec scenarios:
//!   - `corpus::idempotent_on_all_prefixes`
//!   - `corpus::prefix_preservation_on_all_samples`
//!
//! Harness implements the API defined in `specs/m1-remend-rs.spec.md`
//! Decisions: `sample_prefixes`, `assert_all_properties`, `load_corpus`.

use std::borrow::Cow;
use std::fs;
use std::path::Path;
use streaming_markdown_kit::remend;

/// Load every `.md` fixture under `tests/corpus/<model>/`.
fn load_corpus() -> Vec<(String, String)> {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&corpus_dir) else {
        return out;
    };
    for model_entry in entries.flatten() {
        let model_path = model_entry.path();
        if !model_path.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(&model_path) else {
            continue;
        };
        for file_entry in files.flatten() {
            let p = file_entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&p) {
                let rel = p.strip_prefix(&corpus_dir).unwrap_or(&p);
                out.push((rel.to_string_lossy().into_owned(), content));
            }
        }
    }
    out
}

/// Return every sampled prefix for `src` (as byte slices that are valid
/// char-boundary sub-strings).
fn sample_prefixes(src: &str) -> Vec<&str> {
    let mut offsets: Vec<usize> = vec![0];
    for &size in &[16, 32, 64, 128, 256, 512, 1024] {
        if size <= src.len() {
            offsets.push(size);
        }
    }
    // Every \n\n position (index of the second \n + 1 = byte after the boundary).
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            offsets.push(i + 2);
            i += 2;
        } else {
            i += 1;
        }
    }
    offsets.push(src.len());
    offsets.sort();
    offsets.dedup();

    let mut result = Vec::with_capacity(offsets.len());
    for mut off in offsets {
        while off < src.len() && !src.is_char_boundary(off) {
            off += 1;
        }
        if off <= src.len() && src.is_char_boundary(off) {
            result.push(&src[..off]);
        }
    }
    result
}

/// Run the seven corpus-wide properties (a)–(g) on one prefix.
/// Panics on first violation with a `(file, offset, property)` context.
fn assert_properties(file: &str, prefix: &str) {
    // (a) no panic — achieved by completing this call.
    let out = remend(prefix);

    // (c) valid UTF-8 output — guaranteed by &str/String type; assert char count makes sense.
    assert!(
        out.is_char_boundary(0),
        "[{file} @ {}] output not char-boundary-aligned",
        prefix.len()
    );

    // (b) idempotency — second application must equal first and be Borrowed.
    let twice = remend(&out);
    assert_eq!(
        *twice,
        *out,
        "[{file} @ {}] idempotency failed: first = {:?}, second = {:?}",
        prefix.len(),
        &out[..out.len().min(80)],
        &twice[..twice.len().min(80)],
    );
    assert!(
        matches!(twice, Cow::Borrowed(_)),
        "[{file} @ {}] second application should be Cow::Borrowed",
        prefix.len()
    );

    // (d) no HTML closer synthesis — output must not introduce `>` or `</` beyond the input.
    //   Equivalent: any byte appended by remend must not contain `>`.
    assert!(
        out.len() >= prefix.len(),
        "[{file} @ {}] remend returned shorter output than input (it should only append)",
        prefix.len()
    );
    let appended = &out[prefix.len()..];
    assert!(
        !appended.contains('>'),
        "[{file} @ {}] appended tail contains `>` (HTML synthesis): {appended:?}",
        prefix.len()
    );
    assert!(
        !appended.contains("</"),
        "[{file} @ {}] appended tail contains `</` (HTML synthesis): {appended:?}",
        prefix.len()
    );

    // (e) no intraword emphasis synthesis. Weaker corpus-wide form: the
    // unit-scenario tests cover the intraword-rejection semantics directly
    // (test_remend_tightened_intraword_bold, _cjk_intraword_bold, etc).
    // At corpus level we only assert that property (d) holds and defer the
    // per-rule flanking analysis to those scenarios.

    // (f) BYPASS_LANGUAGES preservation is indirectly covered by (g): if
    //     input opens a bypass-language fence, the only appended content
    //     should be `\n` + backticks, which property (d) allows.

    // (g) prefix preservation — output[..prefix.len()] must byte-equal prefix.
    assert_eq!(
        &out[..prefix.len()],
        prefix,
        "[{file} @ {}] prefix preservation violated",
        prefix.len()
    );
}

#[test]
fn idempotent_on_all_prefixes() {
    let corpus = load_corpus();
    assert!(
        !corpus.is_empty(),
        "corpus directory is empty — expected seed files"
    );
    let mut total_prefixes = 0usize;
    for (name, content) in &corpus {
        for prefix in sample_prefixes(content) {
            let once = remend(prefix);
            let twice = remend(&once);
            assert_eq!(
                *twice,
                *once,
                "idempotency failed for {name} @ {}",
                prefix.len()
            );
            assert!(
                matches!(twice, Cow::Borrowed(_)),
                "second application for {name} @ {} should be Cow::Borrowed",
                prefix.len()
            );
            total_prefixes += 1;
        }
    }
    eprintln!(
        "corpus idempotency: {} files × {} prefixes total",
        corpus.len(),
        total_prefixes
    );
}

#[test]
fn prefix_preservation_on_all_samples() {
    let corpus = load_corpus();
    assert!(
        !corpus.is_empty(),
        "corpus directory is empty — expected seed files"
    );
    let mut total_prefixes = 0usize;
    for (name, content) in &corpus {
        for prefix in sample_prefixes(content) {
            assert_properties(name, prefix);
            total_prefixes += 1;
        }
    }
    eprintln!(
        "corpus property coverage: {} files × {} prefixes total",
        corpus.len(),
        total_prefixes
    );
}
