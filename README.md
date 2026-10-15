# ragite

Embeddable, CPU-friendly retrieval (RAG) as a SQLite extension.

Website: https://pihme.github.io/ragite/

## What it is

ragite is an embeddable retrieval component for retrieval-augmented generation (RAG). It splits text into chunks, computes embeddings on the CPU and finds the best-matching passages with a hybrid search (vector similarity plus full-text BM25), always with a link back to the source document and position. It ships as a loadable SQLite extension, so any application in any language that can load SQLite extensions can use it: SQLite for RAG.

```sql
SELECT load_extension('ragite');
CREATE VIRTUAL TABLE docs USING ragite(model='<technical_id>');
INSERT INTO docs(text, source, format)
VALUES ('# Installation ...', 'manual.md', 'text/markdown');
SELECT text, source FROM docs WHERE source = 'manual.md';
SELECT chunk, source, score FROM docs_chunks WHERE docs_chunks MATCH 'How do I install it?' LIMIT 5;
```

(Planned interface; nothing is implemented yet.)

## Goal

- One file, in process, no server: documents, chunks, vectors and full-text index live in the same SQLite database.
- Write through the document table and search documents or chunks; `INSERT`, `UPDATE`, `DELETE` and `ROLLBACK` keep everything consistent.
- Runs well on a CPU, with a built-in default embedding model and the option to bring your own.
- Core in Rust, separate from the SQLite layer, so a Rust library stays possible later.

Out of scope: parsing source formats (PDF, Office, ...), answer generation and evaluation. Input is `text/plain`, `text/markdown`, `text/html`, or a DoclingDocument.

## Limitations

The chunker, the scoring algorithm, and the other presets are fixed. You cannot change how text is cut, how scores are combined, or similar choices from SQL. A build behaves the same way for every caller.

If you want different presets, fork the repository and change the requirements.

## Status

See [ROADMAP.md](ROADMAP.md) for what versions 1.0 and 2.0 will deliver.

Pre-1.0. The design lives in [SPEC.md](SPEC.md). Version 1 uses potion-base-8M, chunks of at most 512 Unicode scalars with an overlap of 64, and takes 50 chunks from each ranking.

## Benchmark

How the default Model is chosen (quality on MTEB sets, parity, CPU costs, Pareto front) and how to run it: [bench/README.md](bench/README.md).

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
