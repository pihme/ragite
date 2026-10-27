<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/ragite-readme-lockup-dark.svg">
    <img alt="RAGite" src="docs/assets/ragite-readme-lockup-universal.svg" width="280">
  </picture>
</h1>

Embeddable, CPU-friendly retrieval (RAG) as a SQLite extension.

[![ci](https://img.shields.io/github/actions/workflow/status/pihme/ragite/ci.yml?branch=main&label=ci&style=flat-square)](https://github.com/pihme/ragite/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/pihme/ragite?display_name=release&label=release&style=flat-square)](https://github.com/pihme/ragite/releases/latest)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-2b6f8a?style=flat-square)](#license)

**[Website](https://pihme.github.io/ragite/)** · [User guide](docs/guide.md) · [Releases](https://github.com/pihme/ragite/releases)

## What it is

RAGite is an embeddable retrieval component for retrieval-augmented generation (RAG). It splits text into chunks, computes embeddings on the CPU and finds the best-matching passages with a hybrid search (vector similarity plus full-text BM25), always with a link back to the source document and position. It ships as a loadable SQLite extension, so any application in any language that can load SQLite extensions can use it: SQLite for RAG.

## Quickstart

Download `libragite.so` from the [latest release](https://github.com/pihme/ragite/releases/latest) (Linux x86-64), or build it with `cargo build -p ragite-sqlite --release` (see [CONTRIBUTING.md](CONTRIBUTING.md)). Start `sqlite3` in the directory that holds the file:

```sql
SELECT load_extension('./libragite');
SELECT name, technical_id FROM ragite_default_model();
CREATE VIRTUAL TABLE docs USING ragite;
INSERT INTO docs(text, source, format)
VALUES ('# Installation

Run the installer and restart the application.', 'manual.md', 'text/markdown');
SELECT text, source FROM docs WHERE source = 'manual.md';
SELECT chunk, source, score FROM docs_chunks WHERE docs_chunks MATCH 'How do I install it?' LIMIT 5;
```

This block is run as a test (`crates/ragite-sqlite/tests/readme.rs`), so it stays correct.

**[User guide](docs/guide.md):** storing documents (formats, chunking, updates, transactions), searching with the hybrid `MATCH`, linking results to your own tables, the embedding functions, and models. Its examples are run in CI as well.

- **Loading.** SQLite looks for the exact name you pass and then for that name plus the platform suffix, so pass a path without the suffix: `load_extension('./libragite')` in SQL, `.load ./libragite` in the `sqlite3` shell. `load_extension('ragite')` does not work: the file is called `libragite.so`, and a bare name is looked up on the library search path, not in the current directory. The file name per platform is `libragite.so` (Linux), `libragite.dylib` (macOS), and `ragite.dll` (Windows, so `load_extension('./ragite')` there). Release binaries are published for Linux only so far. Your SQLite must allow extension loading (in Python: `conn.enable_load_extension(True)`; the macOS system `sqlite3` cannot load extensions, use Homebrew SQLite).
- **Model.** `model` may be omitted in `CREATE VIRTUAL TABLE`; the table then uses the built-in default Model. In this build that is potion-retrieval-32M, technical id `potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7`, as `ragite_default_model()` reports. To pin it, write `USING ragite(model='potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7')`.

## Goal

- One file, in process, no server: documents, chunks, vectors and full-text index live in the same SQLite database.
- Write through the document table and search documents or chunks; `INSERT`, `UPDATE`, `DELETE` and `ROLLBACK` keep everything consistent.
- Runs well on a CPU, with a built-in default embedding model and the option to bring your own.
- Core in Rust, separate from the SQLite layer, so a Rust library stays possible later.

Out of scope: parsing source formats (PDF, Office, ...), answer generation and evaluation. Input is `text/plain`, `text/markdown`, `text/html`, or a DoclingDocument.

## Limitations

The chunker, the scoring algorithm, and the other presets are fixed. You cannot change how text is cut, how scores are combined, or similar choices from SQL. A build behaves the same way for every caller.

If you want different presets, fork the repository and change the requirements.

Where those presets live in the code, and how to add a document type, write or change a chunker, or change the fusion: [docs/tinkering.md](docs/tinkering.md).

## Status

Version 1.0 delivers the interface above. Releases follow Semantic Versioning: a breaking change to the SQL interface, or one that requires re-indexing, comes only with a new major version. The current release is on the [releases page](https://github.com/pihme/ragite/releases/latest). See [ROADMAP.md](ROADMAP.md) for what version 2.0 will deliver.

The design lives in [SPEC.md](SPEC.md). Version 1 uses potion-retrieval-32M (512 dimensions; the release `libragite.so` is about 135 MB (129 MiB) because the weights are embedded), chunks of at most 512 Unicode scalars with an overlap of 64, and takes the 50 best full-text chunks and up to 50 vector neighbors, then drops vector neighbors farther than cosine distance 0.8879346251487732.

## Benchmark

How the default Model is chosen (quality on MTEB sets, parity, CPU costs, Pareto front) and how to run it: [bench/README.md](bench/README.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: [SECURITY.md](SECURITY.md).

## How this project is built

RAGite is developed agent-first. AI coding agents write all code,
tests, and documentation, and review each other's changes, under human
direction: specs, design decisions, and acceptance based on observed
behaviour and test results. No human reads the code line by line. This
is a deliberate choice. Quality rests on automated tests, CI, and
independent agent review.

Evaluate the code against your own requirements before you depend on it.
Found a problem? Open an issue.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option (SPDX: `MIT OR Apache-2.0`). Third-party software and its licenses: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
