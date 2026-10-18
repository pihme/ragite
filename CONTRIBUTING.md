# Contributing

RAGite is open source, dual-licensed under the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.

## Issues

Issues, bug reports and ideas are very welcome. Pick the matching [issue form](https://github.com/pihme/ragite/issues/new/choose) (bug report, feature request, documentation, question). A good report names the RAGite version or commit, your OS and runtime versions, what you ran, what you expected and what happened.

## Pull requests

Pull requests are welcome; unless you state otherwise, any contribution you submit is dual-licensed as above (MIT OR Apache-2.0), without additional terms. For anything bigger than a small fix, please open an issue first so we can agree on the change. Keep a pull request to one change, fill in the [pull request template](.github/pull_request_template.md), give it a [Conventional Commit](https://www.conventionalcommits.org/) title (`fix: …`, `feat: …`, `docs: …`; it becomes the squash commit and decides the next release), and make sure `make test` passes.

## Chunker, scoring, and other presets

Changes to the chunker, the scoring algorithm, and the other fixed presets are very welcome. See [Limitations](README.md#limitations). A pull request that comes with data receives preferential treatment.

## Security and secrets

The tracker is public. Never paste tokens, keys, credentials or real service responses into an issue or pull request. Please report vulnerabilities privately, as described in [SECURITY.md](SECURITY.md).

## Build and test locally

Requirements:

- **Rust** 1.88 or newer (stable, via [rustup](https://rustup.rs/)); `rust-version` in `Cargo.toml` is the minimum.
- **Git LFS.** The model weights and tokenizers under `models/` are LFS files. Run `git lfs install` before cloning, or `git lfs pull` afterwards. A build with LFS pointer files instead of the real files stops with a message saying so.
- **A C compiler** (gcc or clang; MSVC on Windows). SQLite, sqlite-vec, and the tokenizer dependencies are compiled from C/C++ sources, so a C++ compiler is needed too (`build-essential` on Debian/Ubuntu, Xcode command line tools on macOS).
- **The `sqlite3` shell**, which the integration tests drive (`sqlite3` package on Debian/Ubuntu; Homebrew `sqlite` on macOS).
- A few GB of free RAM and disk: the debug build embeds the 129 MB Model.

```bash
make test
```

Without `make`, the same Rust tests run with:

```bash
cargo fetch --locked          # the license test runs `cargo metadata --offline`
cargo build -p ragite-sqlite  # the integration tests load target/debug/libragite.so
cargo test --workspace
```

If the extension is not built or a crate is not downloaded, the failing test says which of the two commands is missing. CI runs exactly these three commands on a fresh checkout (job `fresh-checkout`, TC-270).

`make test` runs on Linux. CI also builds the default extension on macOS and Windows and runs the parity test (TC-246) there. The macOS job uses Homebrew SQLite, because the system `sqlite3` cannot load an extension.

Python 3, Node.js, and Go must be on `PATH`. The Python test uses the standard-library `sqlite3` module. The Node.js test uses `node:sqlite`. The Go test uses cgo and links the host `libsqlite3` (`libsqlite3.so.0` on Linux). Those tests load the extension built in the tree. They do not use the network and do not install a RAGite package.

Test cases are numbered in [TESTSPEC.md](TESTSPEC.md). Cite those numbers from the tests.

## Fuzzing

Fuzzing is optional and is not part of `make test`. The targets, the seed corpus, and how to run them locally or from the manual Fuzz workflow are in [fuzz/README.md](fuzz/README.md).

Mutation tests are not part of `make test`. The manual GitHub Actions workflow `Mutation` runs `cargo mutants` 27.1.0 on `ragite-core`. It does not run on push or pull request. Outcomes are uploaded as a workflow artifact.
