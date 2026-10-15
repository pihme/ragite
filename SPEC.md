# ragite specification

Status: draft, pre-1.0. Nothing here is implemented yet. Requirements carry stable IDs (`REQ-n`); new ones get the next free number, removed ones are not reused.

## 1. Goal

ragite is an embeddable, CPU-friendly retrieval component. It splits text into chunks, computes embeddings and finds the matching passages with a hybrid search, each with a link back to its source. It ships as a SQLite extension that any application can load from any language: SQLite for RAG.

## 2. Guardrails

- **REQ-1** The core (chunking, embedding, search logic) is written in Rust and kept separate from the SQLite layer, so a plain Rust library remains possible later.
- **REQ-2** ragite is distributed only as a loadable SQLite extension, with its own thin FFI layer on `libsqlite3-sys`.
- **REQ-3** The interface is a virtual table that supports both writing and reading.
- **REQ-4** Transformer models run through [candle](https://github.com/huggingface/candle); static Model2Vec models run through `model2vec-rs`. No ONNX Runtime or other C++ inference library.
- **REQ-5** Inference runs well on a CPU; a GPU is never required.
- **REQ-6** ragite is licensed `MIT OR Apache-2.0` (`license = "MIT OR Apache-2.0"` in `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`). Every dependency and every bundled model must use a license compatible with that (permissive, e.g. MIT or Apache-2.0).

## 3. Scope and non-goals

- **REQ-7** Accepted input: plain text, Markdown, HTML and DoclingDocument JSON.
- **REQ-8** Not in scope: parsing source formats (PDF, Office, scans), answer generation (LLM calls) and evaluation. Users combine ragite with tools such as Docling (parsing), a local LLM runtime (generation) or Ragas (evaluation).
- Reranking is not part of the first version; it may be added later as an option.

## 4. SQLite interface

Example (planned):

```sql
SELECT load_extension('ragite');
CREATE VIRTUAL TABLE docs USING ragite(model='default');
INSERT INTO docs(text, source) VALUES ('# Installation ...', 'manual.md');
SELECT chunk, source FROM docs WHERE docs MATCH 'How do I install it?' LIMIT 5;
```

The virtual table follows the methods in [sqlite.org/vtab](https://sqlite.org/vtab.html).

### 4.1 Creating a table

- **REQ-9** `CREATE VIRTUAL TABLE <name> USING ragite(model=...)` creates the table and its shadow tables (section 5) in the same database file. `model='default'` selects the built-in model.

### 4.2 INSERT

- **REQ-10** An `INSERT` is handled in `xUpdate`: ragite splits the text into chunks, computes the embeddings and writes the document, its chunks, the vectors (sqlite-vec) and the full-text rows (FTS5).
- **REQ-11** All writes of one statement happen in the caller's transaction; a `ROLLBACK` undoes all of them.

### 4.3 UPDATE

- **REQ-12** An `UPDATE` that changes the text deletes the document's old chunks, vectors and FTS rows, re-chunks and re-embeds the new text, in the same transaction.
- **REQ-13** An `UPDATE` that changes only metadata (for example `source`) recomputes nothing.

### 4.4 DELETE

- **REQ-14** A `DELETE` removes the document with all its chunks, vectors and FTS rows, in the same transaction.

### 4.5 Search

- **REQ-15** `xBestIndex` recognises `MATCH` and `LIMIT`. `xFilter` embeds the query and combines vector search and BM25 full-text search (for example with Reciprocal Rank Fusion). `xNext`, `xEof`, `xColumn` and `xRowid` return the hits.
- **REQ-16** Each hit exposes at least the chunk text and its source (section 5).
- Further `WHERE` filters, `JOIN`s with application tables and `ORDER BY` are left to SQLite.

### 4.6 Helper function

- **REQ-17** A scalar function `ragite_embed(text)` returns the embedding of a text with the table's model, for use in the application's own tables.

### 4.7 Use from other languages

- **REQ-18** The extension can be loaded from Python (`sqlite3` with `enable_load_extension`), Go and Node.js. Distribution is planned as prebuilt binaries per platform with thin pip/npm/Go packages, as sqlite-vec does. (Publishing is not part of the first version.)

## 5. Source linkage: documents and chunks

Draft schema, all in the same database file:

```text
documents(doc_id, source, format, meta_json, hash)
chunks(chunk_id INTEGER PRIMARY KEY, doc_id, start, end, heading_path, text, meta_json)
chunk_vec   -- sqlite-vec vec0 table, rowid = chunk_id
chunk_fts   -- FTS5 table, rowid = chunk_id
```

- **REQ-19** Every chunk records its document, its character offsets (`start`, `end`) in the input text and its heading path.
- **REQ-20** For DoclingDocument input, page, section and element reference are stored in the chunk's `meta_json` (DoclingDocument carries reading order and provenance such as page and bounding box).
- **REQ-21** The vector table and the FTS5 table use `rowid = chunk_id`, so a KNN search (`WHERE embedding MATCH ?`) joined with `chunks` and `documents` yields text, source and position.

## 6. Model variants

- **REQ-22** (a) **Built-in (default):** model files are embedded in the extension with `include_bytes!`, behind a Cargo feature that is on by default. Model2Vec models load with `model2vec-rs` (`StaticModel::from_bytes`), transformer models with candle.
- **REQ-23** (b) **Own model at build time:** a model path is passed to the build through an environment variable (`build.rs` or `include_bytes!(env!(...))`); the default model is switched off with `--no-default-features`.
- **REQ-24** (c) **External model at run time:** loaded from a local path or a URL (downloaded and cached locally); for Model2Vec via `StaticModel::from_pretrained`.
- **REQ-25** The model ID is stored in the database, so ragite detects when a table is opened with a different model than it was built with.
- Late chunking (embed the whole document, then pool per chunk) only makes sense for transformer models, not for Model2Vec; it is a possible later option.

### 6.1 Model criteria

- **REQ-26** The default model runs well on a CPU and has a free license (MIT or Apache-2.0). English takes priority over German-only models; multilingual support is a bonus, not a requirement. Models with non-open-source terms (for example EmbeddingGemma under the Gemma terms) are excluded.

## 7. Licensing and NOTICE for models

- **REQ-27** Bundled model weights (variant a) are redistributed and keep their own license. The license texts of all bundled dependencies and models go into `THIRD_PARTY_NOTICES.md` (the NOTICE file), which ships with every artifact.

Known licenses (from the design research, to be verified against real metadata when dependencies are added): SQLite public domain; `libsqlite3-sys` MIT; sqlite-vec MIT OR Apache-2.0; candle MIT OR Apache-2.0; `model2vec-rs` MIT (LICENSE file; crates.io shows "non-standard"); `tokenizers` Apache-2.0; `text-splitter` MIT (if used). This is an assessment, not legal advice.

## 8. Open questions

### 8.1 Default model

Which model is built in for variant (a) is decided during development by a benchmark (retrieval quality, CPU speed, size). Candidates (parameters, dimension and license from the Hugging Face model cards):

| Model | Parameters | Dim. | License | Kind |
| --- | --- | --- | --- | --- |
| potion-base-8M | 7.6 M (~30 MB, F32) | 256 | MIT | Model2Vec, static, English |
| potion-base-32M | 32 M (~129 MB, F32) | 512 | MIT | Model2Vec, static, English |
| potion-retrieval-32M | 32 M (~129 MB, F32) | 512 | MIT | Model2Vec, retrieval-trained, English |
| all-MiniLM-L6-v2 | 22.7 M | 384 | Apache-2.0 | Transformer, English |
| bge-small-en-v1.5 | 33.4 M | 384 | MIT | Transformer, English |
| e5-small-v2 | 33.4 M | 384 | MIT | Transformer, English |
| nomic-embed-text-v1.5 | 137 M | 768 | Apache-2.0 | Transformer, English, longer context |
| potion-multilingual-128M (bonus) | 128 M | 256 | MIT | Model2Vec, 101 languages, from bge-m3 (MIT), trained on C4 (ODC-BY) |
| multilingual-e5-small (bonus) | 118 M | 384 | MIT | Transformer, multilingual |
| paraphrase-multilingual-MiniLM-L12-v2 (bonus) | 118 M | 384 | Apache-2.0 | Transformer, multilingual |
| gte-multilingual-base (bonus) | 305 M | 768 | Apache-2.0 | Transformer, multilingual, largest candidate |

Model2Vec claims up to 500x faster CPU inference than its source model with small quality loss; reliable absolute CPU numbers are missing, which the benchmark settles.

## 9. References

- Chunking: `text-splitter` (Rust, MIT; candidate to depend on), Docling `HierarchicalChunker`/`HybridChunker`.
- Hybrid search: sqlite-vec + FTS5 with Reciprocal Rank Fusion in SQL.
- Virtual tables: `sqlite-vec.c` (vec0 with `xBestIndex`/`xUpdate`), the FTS5 source (pattern for `MATCH`), `sqlite-loadable` (Rust FFI pattern; reference only, not a dependency).
- Considered and not used: `rusqlite` (its `vtab` feature is read-only), `ort` (pulls in a C++ library).
