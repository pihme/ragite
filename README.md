# ragite

Embeddable, CPU-friendly retrieval (RAG) as a SQLite extension.

Website: https://pihme.github.io/ragite/

## What it is

ragite is an embeddable retrieval component for retrieval-augmented generation (RAG). It splits text into chunks, computes embeddings on the CPU and finds the best-matching passages with a hybrid search (vector similarity plus full-text BM25), always with a link back to the source document and position. It ships as a loadable SQLite extension, so any application in any language that can load SQLite extensions can use it: SQLite for RAG.

```sql
SELECT load_extension('ragite');
CREATE VIRTUAL TABLE docs USING ragite(model='default');
INSERT INTO docs(text, source) VALUES ('# Installation ...', 'manual.md');
SELECT chunk, source FROM docs WHERE docs MATCH 'How do I install it?' LIMIT 5;
```

(Planned interface; nothing is implemented yet.)

## Goal

- One file, in process, no server: documents, chunks, vectors and full-text index live in the same SQLite database.
- Write and search through one virtual table; `INSERT`, `UPDATE`, `DELETE` and `ROLLBACK` keep everything consistent.
- Runs well on a CPU, with a built-in default embedding model and the option to bring your own.
- Core in Rust, separate from the SQLite layer, so a Rust library stays possible later.

Out of scope: parsing source formats (PDF, Office, ...), answer generation and evaluation. Input is plain text, Markdown, HTML or DoclingDocument JSON.

## Status

See [ROADMAP.md](ROADMAP.md) for what versions 1.0 and 2 will deliver.

Pre-1.0, specification only. There is no code, no release and no package yet. The design lives in [SPEC.md](SPEC.md); the default embedding model is still open and will be chosen by benchmark.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: [SECURITY.md](SECURITY.md).

## How this project is built

ragite is developed agent-first. AI coding agents write all code,
tests, and documentation, and review each other's changes, under human
direction: specs, design decisions, and acceptance based on observed
behaviour and test results. No human reads the code line by line. This
is a deliberate choice. Quality rests on automated tests, CI, and
independent agent review.

Evaluate the code against your own requirements before you depend on it.
Found a problem? Open an issue.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option (SPDX: `MIT OR Apache-2.0`). Third-party software and its licenses: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
