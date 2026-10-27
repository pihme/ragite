# Fuzzing the chunkers

`make test` does not run these targets. They are a manual check for the HTML, Markdown, plain-text, and Docling chunkers (REQ-95).

Each target feeds arbitrary bytes to a chunker, decoded as UTF-8 with invalid bytes replaced, and checks:

- the call finishes without a panic
- a plain-text chunk, a Markdown paragraph or code piece, an HTML paragraph or loose-text piece, and a Docling item piece each hold at most 512 Unicode scalars
- a Markdown heading, an HTML heading, and an HTML comment may be longer, because the length limit does not split them
- overlapping pieces share at least 64 scalars; the next piece starts on a word boundary only when the previous piece ends on one, and a next piece that does not start on a word boundary overlaps by exactly 64 scalars
- chunk text is valid UTF-8, and a chunk does not invent U+0000

## Run locally

Install a nightly toolchain and [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz):

```bash
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

From the repository root:

```bash
cargo +nightly fuzz run html -- -max_total_time=30
cargo +nightly fuzz run markdown -- -max_total_time=30
cargo +nightly fuzz run docling -- -max_total_time=30
```

`markdown` also checks the same bytes as plain text. Seeds live in `fuzz/corpus/<target>/`. A crash is written under `fuzz/artifacts/<target>/`.

## Reproduce a crash

```bash
cargo +nightly fuzz run html -- fuzz/artifacts/html/crash-FILENAME
```

A crash that survives minimization becomes a regression test under `crates/ragite-core` and a seed in the corpus.

## CI

The workflow [Fuzz](../.github/workflows/fuzz.yml) runs only when someone starts it by hand (`workflow_dispatch`). It is not part of push or pull-request CI.

In the GitHub Actions tab, choose the Fuzz workflow, then Run workflow. Set `target` to `html`, `markdown`, `docling`, or `all`, and `duration` to the seconds each target should run (default 600). The job fails when a target crashes. Crashes and the corpus are uploaded as artifacts.
