# Fixture corpus

Ground-truth markdown samples used to exercise `remend` (M1) and later
milestones. Layout:

```
corpus/
├── kimi/       # Kimi K2 responses — real (from aichat_history.json)
├── opus/       # Claude Opus 4.7 — synthetic-coverage (.jsonl with explicit chunks, .md for prefix sampling)
├── gpt/        # GPT 5.4 — real completion
├── deepseek/   # (M2 prerequisite — not yet collected)
├── qwen/       # (M2 prerequisite — not yet collected)
└── …
```

### Real vs synthetic-coverage

Two kinds of samples coexist:

- **real**: complete responses actually produced by a chat client and
  captured to disk. Valid for both content coverage and (in M2) for
  informing chunk-boundary heuristics once timestamps are added.
- **synthetic-coverage**: responses crafted to stress every remend
  rule boundary at hand-picked chunk splits. Valid only for rule
  coverage — chunk boundaries are not representative of the model's
  actual streaming output distribution. The `.jsonl` format preserves
  the crafted chunks as an array; the companion `.md` is the
  `expected_final_after_remend` extracted for prefix-sampling
  assertions.

The `source` field in a `.jsonl` discriminates (`"source": "real"` vs
`"source": "crafted"`). `.md`-only samples are implicitly real.

Each `.md` file is a **complete final assistant response** as stored by
the producing chat client. Streaming chunks are synthesised by the test
harness via prefix sampling — see below.

## Why final responses, not recorded streams?

`remend` is a pure function of its input string. Whether a particular
prefix arrived via one chunk or ten is irrelevant — only the prefix
content matters. Prefix sampling a complete response at many
granularities produces a larger and more exhaustive test set than
record-and-replay of one live stream would.

Commit/tail boundary heuristics (M2) do depend on real timing, so for M2
we will need additional live-recorded samples (chunks + timestamps).
Those samples belong in `corpus/streams/<model>/…` and do not yet exist.

## Prefix sampling

The test harness slices each `.md` file at:

- Every fixed byte offset: `{16, 32, 64, 128, 256, 512, 1024}`
- Every `\n\n` position (paragraph boundaries)
- Every `\n` immediately following a fenced-code opener (mid-fence)
- The exact byte before each closing backtick run (mid-fence-close)
- Every char-boundary-adjacent position inside a CJK run on the first 3
  occurrences per file (to stress multi-byte scalars at the tail)

Each produced prefix is a `&str` input for `remend`. Per-prefix
assertions are:

1. **No panic.**
2. **Idempotency:** `remend(remend(prefix)) == remend(prefix)`.
3. **Well-formed UTF-8 output.**
4. **No HTML closer synthesis:** the output must contain no `>`
   character introduced at a position beyond the input length, and no
   substring `</…>` that was not already in the input.
5. **No intraword `*`/`**` synthesis:** if the output contains more
   asterisks than the input, the first added asterisk must not be
   preceded by a word character at its insertion position.
6. **Fenced-code bypass:** if a rule's insertion point is inside a block
   whose info-string is in `BYPASS_LANGUAGES`, the output byte-equals
   the input plus at most a trailing fenced-code closer.

These are **property-style assertions**, not golden strings. Golden
strings would over-specify remend's output for edge cases and brick the
tests against any future rule refinement.

## Adding new samples

Drop a `.md` file under the appropriate `corpus/<model>/` directory. No
metadata file needed for M1. File name should describe the content, not
the test it's used in. Prefer files between 1 KB and 30 KB — smaller
than 1 KB gives too few prefix-sampling points; larger than 30 KB slows
the test suite without adding coverage.

## Seed samples (M1)

Two Kimi responses extracted from the aichat example's
`aichat_history.json` on 2026-04-19. Both responses were prompted with
"give me markdown with all the elements" variants and include every
construct remend needs to handle:

- `kimi/markdown-demo-short.md` (~4 KB) — starts with a 4-backtick
  `\`\`\`\`markdown … \`\`\`\`` outer wrapper, contains bold / italic /
  strike / inline code / fenced python / fenced mermaid (flowchart +
  architecture) / table / blockquote / footnote / HTML `<details>` /
  mixed CJK-Latin.
- `kimi/markdown-demo-long.md` (~12 KB) — 7 mermaid diagram types
  (flowchart, architecture, sequence with alt/par, classDiagram,
  gantt, stateDiagram-v2, erDiagram), 3 fenced code languages (Python,
  Rust, SQL), LaTeX blocks (integrals, matrices, Bayesian probability),
  GFM alerts `> [!NOTE]` / `> [!TIP]` / `> [!IMPORTANT]` /
  `> [!WARNING]`, task lists, footnotes.

This is sufficient construct coverage to ship M1. DeepSeek / Qwen / GPT
/ Claude diversity is a M2 prerequisite.
