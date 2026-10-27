# RAGite specification

Status: version 1. The interface in this file is implemented in the current release. Requirements carry stable IDs (`REQ-n`); new ones get the next free number, removed ones are not reused. A constraint is a rule for the shape of the code. It has no `REQ-n` and no test cases. Section 10 records each change, newest first.

## 1. Goal

RAGite is an embeddable, CPU-friendly retrieval component. It splits text into chunks, computes embeddings and finds the matching passages with a hybrid search, each with a link back to its source. It ships as a SQLite extension that any application can load from any language: SQLite for RAG.

## 2. Guardrails

- **REQ-1** The core (chunking, embedding, search logic) is written in Rust and kept separate from the SQLite layer, so a plain Rust library remains possible later.
- **REQ-2** RAGite is distributed only as a loadable SQLite extension, with its own thin FFI layer on `libsqlite3-sys`.
- **REQ-3** The interface is a virtual table that supports both writing and reading.
- **REQ-4** Transformer models run through [candle](https://github.com/huggingface/candle); static Model2Vec models run through `model2vec-rs`. No ONNX Runtime or other C++ inference library.
- **REQ-5** Inference runs well on a CPU; a GPU is never required.
- **REQ-6** RAGite is licensed `MIT OR Apache-2.0` (`license = "MIT OR Apache-2.0"` in `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`). Every dependency and every bundled model must use a license compatible with that (permissive, e.g. MIT or Apache-2.0).

## 3. Scope and non-goals

- **REQ-7** Accepted input is a media type on the Document: `text/plain`, `text/markdown`, `text/html`, or a DoclingDocument. Do not reopen.
- **REQ-8** Not in scope: parsing source formats (PDF, Office, scans), answer generation (LLM calls) and evaluation. Users combine RAGite with tools such as Docling (parsing), a local LLM runtime (generation) or Ragas (evaluation).
- Reranking is not part of the first version; it may be added later as an option.

## 4. SQLite interface

Example (planned):

```sql
SELECT load_extension('ragite');
CREATE VIRTUAL TABLE docs USING ragite(model='<technical_id>');
INSERT INTO docs(text, source, format)
VALUES ('# Installation ...', 'manual.md', 'text/markdown');
SELECT doc_id, source, score FROM docs
WHERE docs MATCH 'How do I install it?' AND source = 'manual.md' LIMIT 5;
SELECT chunk, source, score FROM docs_chunks
WHERE docs_chunks MATCH 'How do I install it?' LIMIT 5;
```

The virtual table follows the methods in [sqlite.org/vtab](https://sqlite.org/vtab.html).

A write addresses one Document: the `text` value is the whole input, and RAGite stores that text on the Document and cuts it into Chunks. The Document table returns the stored text. `source` is an opaque label the application supplies, such as a path, a URL, or a title; RAGite stores it and returns it with each Chunk. The same Source may belong to more than one Document. The indexed bytes are the `text` value. RAGite assigns each Document a `doc_id`. `UPDATE` and `DELETE` name the Document by `doc_id`. A `WHERE` clause on another column selects every matching Document. Creating `<name>` also creates a read-only companion `<name>_chunks`. `MATCH` on `<name>` returns each matching Document once, ranked by its best Chunk. `MATCH` on `<name>_chunks` returns Chunks. Do not reopen.

### 4.1 Creating a table

- **REQ-9** `CREATE VIRTUAL TABLE <name> USING ragite` creates the Document table, the read-only companion `<name>_chunks`, and the shadow tables (section 5) in the same database file. One database may hold several such tables. Each table's shadow tables are prefixed by `<name>`. The companion has no shadow tables of its own. `model` may be omitted (REQ-62). When it is present, it is the Model's technical identifier (REQ-46). Do not reopen.
- **REQ-62** When `CREATE VIRTUAL TABLE` omits `model`, RAGite binds the Table to the technical identifier of the build's default Model. The stored value is that technical identifier. The word `default` is not a model name. An empty `model` value fails the statement. Do not reopen.
- **REQ-39** `CREATE VIRTUAL TABLE` fails and leaves nothing behind when the Model cannot be loaded, when `<name>_chunks` already exists, or when one of the shadow names already exists. A failed create does not leave a Document table, a companion, or shadow tables. Do not reopen.
- **REQ-88** `companion` and `parent` are accepted only on the companion create that RAGite runs while creating `<name>`. Any other `CREATE VIRTUAL TABLE` that passes either argument fails with `argument is not accepted` and leaves no table from that statement. Opening the database again still connects the companion RAGite created. Do not reopen.

### 4.2 INSERT

- **REQ-10** An `INSERT` is handled in `xUpdate`: RAGite splits the text into chunks, computes the embeddings and writes the document, its chunks, the vectors (sqlite-vec) and the full-text rows (FTS5).
- **REQ-35** An `INSERT` or `UPDATE` whose `text` has no characters fails the statement and leaves the database unchanged. Whitespace is still text. RAGite does not store a Document with empty text. Do not reopen.
- **REQ-37** An `INSERT` must supply `text`. `source`, `format`, and `hash` may be omitted. Omitting `source`, or supplying `''`, stores an empty Source. Omitting `text` fails the statement. NULL in an optional column follows REQ-73. `doc_id` and `score` are not accepted on `INSERT` or `UPDATE`. RAGite assigns `doc_id`. On `UPDATE`, the caller may set `text`, `source`, `format`, and `hash`. Do not reopen.
- **REQ-72** RAGite compares `format` to the four accepted types without regard to letter case and stores the canonical lowercase type. A parameter, such as `; charset=utf-8`, fails the statement. Do not reopen.
- **REQ-73** NULL in an optional column is the same as omitting it. NULL `source` stores an empty Source. NULL `format` stores `text/plain`. NULL `hash` makes RAGite compute the SHA-256. NULL `text` fails the statement. On `UPDATE`, assigning NULL to `source`, `format`, or `hash` sets that same default, and a NULL `format` re-chunks. NULL `model` on `CREATE VIRTUAL TABLE` selects the build's default Model (REQ-62). Do not reopen.
- **REQ-36** An `INSERT` or `UPDATE` that produces no Chunks still stores the Document. It writes no Chunk rows, vectors, or full-text rows. A `MATCH` search does not find it. A query on ordinary columns does return it, for example `WHERE source = '...'`. This is expected, including for text that is only `script` and `style`, and for a DoclingDocument whose items have no characters. A Markdown or HTML Document whose only content is headings has one Chunk per heading (REQ-50). Do not reopen.
- **REQ-31** The Document stores the inserted `text` and the Document table returns it. `start` and `end` count Unicode scalar values, the same units as a Rust `char`. They do not count UTF-8 bytes or grapheme clusters. For plain text and Markdown, they count in the stored text and the Chunk text is the exact slice between those offsets. For a DoclingDocument they follow REQ-20. Do not reopen.
- **REQ-93** On `INSERT` and `UPDATE`, a numeric value in `text`, `source`, `format`, or `hash` is stored as SQLite's text representation of that value. A real number keeps its fractional part: `3.7` is stored as `3.7`. Do not reopen.
- **REQ-34** The Document table returns `doc_id`, `text`, `source`, `format`, `hash`, and `score`. `score` is the Score column. Its `rowid` is `doc_id`. The Document has no `meta_json`. `hash` is an opaque label, like `source`. The caller may set it on `INSERT` and change it with `UPDATE`. RAGite stores a caller-supplied value as given and does not compare it to the text or use it in any decision. When an `INSERT` omits `hash`, RAGite stores the SHA-256 of the stored text as UTF-8, written as 64 hex characters. An `UPDATE` that does not set `hash` leaves it unchanged, including when the text changes. Changing `hash` recomputes nothing. Do not reopen.
- **REQ-32** `format` is a media type with no parameters, stored on the Document. `<name>_chunks` does not expose it. When it is omitted, RAGite stores `text/plain` and does not infer the type from the text. The only accepted values are `text/plain`, `text/markdown`, `text/html`, and `application/vnd.docling.document+json`. Matching ignores letter case and stores the canonical lowercase type (REQ-72). Any other value, including a parameter, fails the statement. Do not reopen.
- **REQ-11** All writes of one statement happen in the caller's transaction; a `ROLLBACK` undoes all of them.

### 4.3 UPDATE

- **REQ-12** An `UPDATE` that changes the text or the format deletes the document's old chunks, vectors and FTS rows, re-chunks and re-embeds, in the same transaction. A format change uses the stored text with the new media type. An invalid format fails the statement and leaves the Document unchanged. Do not reopen.
- **REQ-13** An `UPDATE` that changes only metadata that does not affect how the text is cut (for example `source` or `hash`) recomputes nothing.

### 4.4 DELETE

- **REQ-14** A `DELETE` removes the document with all its chunks, vectors and FTS rows, in the same transaction.
- **REQ-29** RAGite assigns `doc_id` when a Document is inserted (REQ-57). The same Source may belong to more than one Document. `UPDATE` and `DELETE` name the Document by `doc_id`. A `WHERE` clause on another column, such as `source`, selects every matching Document. Do not reopen.
- **REQ-57** `doc_id` and Chunk id are positive integers. Documents and Chunks have separate sequences (REQ-64). After a row is deleted, that id is not assigned again in its sequence. An update that rewrites a Document's Chunks gives those Chunks new ids. Each sequence is a SQLite `INTEGER PRIMARY KEY AUTOINCREMENT` (REQ-84). Do not reopen.
- **REQ-84** A deleted id stays unused, so each sequence is SQLite `AUTOINCREMENT`. RAGite does not keep a second counter and does not change `sqlite_sequence` itself. A failed statement and a rolled-back transaction follow SQLite: `sqlite_sequence` moves back with that work, so an id allocated only inside that work can be assigned again. Do not reopen.
- **REQ-64** `doc_id` values are unique among Documents in the Table. Chunk ids are unique among Chunks in the Table. The same number may be both a `doc_id` and a Chunk id. Each sequence assigns the next value it has not used. Do not reopen.

### 4.5 Search

The Score is the number Reciprocal Rank Fusion assigns to a Chunk. A higher Score is a better match. A Document's Score is the Score of its best Chunk. The Score column is the virtual-table column named `score`. It is present on both surfaces. On a `MATCH` query it carries that row's Score, and the rows come back best first. On any other query it is null. The caller does not pass a cutoff. Do not reopen.

- **REQ-15** `xBestIndex` recognises `MATCH` on both surfaces. SQLite applies `LIMIT` to the fused rows after that scan. The rankings stay the fixed depth, and each row `LIMIT` returns keeps the Score it has in the same query without `LIMIT` (REQ-38). `xFilter` embeds the query and merges the vector ranking and the BM25 ranking with Reciprocal Rank Fusion, k = 60, both sides weighted equally. The vector ranking uses cosine similarity (REQ-66). The top row of each list has rank 1 (REQ-80). The caller does not pass weights. RAGite takes the same fixed number of Chunks from each ranking before the fusion. The caller does not pass that depth. The number is chosen in the same benchmark as the default model and the chunk length (section 8.1). A Chunk that appears in only one list still takes part in the fusion. On the Document table, the result is one row per Document, ranked by its best Chunk. On `<name>_chunks`, the result is one row per Chunk. The Score column carries that Score for a `MATCH` and is null otherwise. Do not reopen.
- **REQ-38** A `MATCH` without `LIMIT` returns every fused row. On `<name>_chunks` that is every fused Chunk. On the Document table that is every matching Document. The candidate depth is the only cap RAGite applies. The caller uses `LIMIT` to take fewer rows. Do not reopen.
- **REQ-43** A `MATCH` whose query has no characters returns no rows and does not fail. RAGite does not run the Model for that query. A query that is only whitespace is a real query and is embedded as given. Do not reopen.
- **REQ-41** A `MATCH` returns rows ordered by Score, highest first. When two Scores are equal, the Document table orders by `doc_id` ascending. The companion orders by `doc_id` ascending, then by Chunk id ascending. Do not reopen.
- **REQ-42** With no `MATCH`, the Document table returns rows in `doc_id` ascending order. The companion returns rows in `doc_id` ascending order, then Chunk id ascending. Chunk ids follow emission order (REQ-52). `score` is null. A `WHERE` clause on an ordinary column keeps that order. Do not reopen.
- **REQ-16** Each Chunk row exposes `chunk`, `doc_id`, `source`, `start`, `end`, `heading_path`, `meta_json`, and `score`. Its `rowid` is the Chunk id (REQ-65). `heading_path` is a JSON array of strings, outermost heading first. A plain-text Chunk stores `[]`. `meta_json` is null except on a DoclingDocument Chunk, where it is the object in REQ-20. `score` is the Score column. Do not reopen.
- **REQ-30** Creating `<name>` also creates the read-only virtual table `<name>_chunks`. `<name>` has one row per Document and its `rowid` is `doc_id`. The companion's `rowid` is the Chunk id (REQ-65). Writes go to `<name>`. A normal predicate such as `source = '...'` selects every matching Document. `MATCH` on `<name>` returns each matching Document once, ranked by its best Chunk. `MATCH` on `<name>_chunks` returns Chunks. Writes on `<name>_chunks` fail. Do not reopen.
- **REQ-89** `ALTER TABLE <name> RENAME` and `ALTER TABLE <name>_chunks RENAME` fail. The error is `rename is not accepted`. Both surfaces keep the names they were created with. `SELECT` and `INSERT` on `<name>` still work, and `DROP TABLE <name>` still removes that table. Do not reopen.
- **REQ-65** On the Document table, `rowid` is `doc_id`. On `<name>_chunks`, `rowid` is the Chunk id. Neither surface has a `chunk_id` column. Do not reopen.
- **REQ-96** Equality on `rowid` is a unique lookup. On the Document table the row is that `doc_id`. On `<name>_chunks` the row is that Chunk id. `xBestIndex` accepts the constraint, estimates one row, and marks the scan unique. A missing id returns no row and does not fail. The comparison matches `rowid` equality on a SQLite rowid table: `2.5` and the text `'2.5'` return no row, and `2.0` and the text `'2'` return the row whose id is 2. With no `rowid` equality, rows stay in the order of REQ-42. When `MATCH` is usable as well, the plan stays the search from REQ-15.
- **REQ-78** The Document table declares `doc_id INTEGER`, `text TEXT`, `source TEXT`, `format TEXT`, `hash TEXT`, and `score REAL`. The companion declares `chunk TEXT`, `doc_id INTEGER`, `source TEXT`, `start INTEGER`, `end INTEGER`, `heading_path TEXT`, `meta_json TEXT`, and `score REAL`. Do not reopen.
- **REQ-82** A `MATCH` constraint on the Document table or on `<name>_chunks` runs the hybrid search whichever column it is written on. `docs MATCH 'q'`, `text MATCH 'q'`, and `chunk MATCH 'q'` are the same search. Do not reopen.
- **REQ-66** The vector ranking uses cosine similarity. Reciprocal Rank Fusion uses those ranks, with k = 60 and equal weights (REQ-15). Do not reopen.
- **REQ-80** The top row of a list has rank 1, so its term is `1/61`. A Chunk that appears in both lists sums its two terms. A Document's Score is its best Chunk's Score. Do not reopen.
- **REQ-75** A `MATCH` string is plain text. RAGite searches the FTS5 index for every token the Table's `unicode61` tokenizer produces from that string. A token is a run of letters and numbers. Every other scalar is a separator and is not a token. Case and diacritics follow that tokenizer. A Chunk is a full-text hit only when it contains every one of those tokens. The tokens may occur in any order and need not be adjacent. Extra tokens in the Chunk do not matter. RAGite does not pass the `MATCH` string through as an FTS5 query. It searches for an AND of those tokens, each quoted, so that a token is never read as FTS5 syntax. `AND`, `OR`, `NOT`, `*`, and quotation marks are not operators. A star does not make a prefix query. Quotation marks do not group a phrase. When the tokenizer produces no tokens, the full-text side matches nothing and the statement does not fail. The vector side embeds that same plain text, with the query prefix. Version 1 accepts no FTS5 query syntax from the caller. Do not reopen.
- **REQ-61** The text embedded for a Chunk is the passage prefix followed by the Chunk text. The text embedded for a `MATCH` query is the query prefix followed by the query text. FTS indexes the Chunk text and searches for every `unicode61` token of the query text, without a prefix (REQ-75). The heading path and `meta_json` are not part of either embedding or of the FTS index. Do not reopen.
- **REQ-99** When `MATCH` is usable, RAGite builds each ranking in REQ-15 only from Chunks that satisfy the other constraints SQLite marks usable, for `=` and `IN`, on `rowid` and on every column except `score`. On the Document table those columns are `doc_id`, `text`, `source`, `format`, and `hash`, and `rowid` is `doc_id`. On `<name>_chunks` they are `chunk`, `doc_id`, `source`, `start`, `end`, `heading_path`, and `meta_json`, and `rowid` is the Chunk id. A constraint on the Document table keeps every Chunk of a Document that satisfies it. A constraint on `<name>_chunks` keeps the Chunks that satisfy it. Every such constraint applies. Both rankings keep their fixed depth inside that set. A Chunk that is first inside the set has rank 1 even when closer matches exist outside it, and its Score is that rank (REQ-80). A `rowid` equality keeps the search plan (REQ-96). Comparison of `rowid` follows REQ-96. Any other column uses SQLite's comparison for its declared type (REQ-78). When no Chunk satisfies the constraints, the query returns no rows and does not fail. An empty `MATCH` still returns no rows and does not run the Model (REQ-43). With no `MATCH`, a predicate selects from the full scan (REQ-42, REQ-96). Do not reopen.
- **REQ-100** A comparison on `score` is applied to the fused rows. The Score is the result of the fusion, so it does not exist while the rankings are built and cannot select Chunks before the depth is counted. A Chunk outside that depth stays absent. Do not reopen.
- **REQ-101** `LIMIT` and `OFFSET` choose a window of the fused rows (REQ-38). They do not shorten the rankings. A rank is a position in the depth-sized list, so a shorter list would assign different Scores. The row `LIMIT 1` returns has the Score it has when the query has no `LIMIT`. `ORDER BY` reorders the fused rows and leaves the set unchanged. With no `ORDER BY`, the order is REQ-41. Do not reopen.
- **REQ-102** A `=` or `IN` that equates a column of this virtual table to a column of another table is a constraint on this table when SQLite marks it usable. SQLite marks it usable when the other table is the outer loop. RAGite then builds each ranking in REQ-99 from the Chunks that constraint names, once per outer row. That plan costs less than a `MATCH` with no such constraint, so an inner join puts the other table first and the result does not depend on an incidental choice of loop order. A Chunk outside the global depth is present when the key names its set, and its Score is the rank inside that set (REQ-80). `CROSS JOIN` and `LEFT JOIN` fix the loop order. With the other table on the left the key is usable. With this table on the left of a `LEFT JOIN` the key is not usable and the search is the unrestricted `MATCH`. A predicate that does not constrain a column of this table is not offered to `xBestIndex`. `pages.project = 'wiki'` with no RAGite column in the join does not select Chunks before the depth is counted, and a Document outside that depth stays absent. Do not reopen.
- **REQ-103** `LIKE`, `GLOB`, `REGEXP`, `IS`, `IS NOT`, and the inequalities are applied to the fused rows. They do not name a set of Chunk ids, so the rankings cannot be restricted to them before the depth is counted. `BETWEEN` arrives as two inequalities and follows the same rule. An `OR` of equalities is consumed when SQLite offers it as one `IN` or as a separate search for each arm. When SQLite marks no arm usable, it applies the `OR` to the fused rows. A Chunk outside the depth stays absent. Do not reopen.

### 4.6 Helper function

- **REQ-17** `ragite_embed(table_name, text, role)` returns the embedding of `text` using the Model of that Document table, for use in the application's own tables. `table_name` is the Document table, not the companion. A name that is not a RAGite Document table fails the statement. `ragite_embed_model(model_identifier, text, role)` returns the embedding of `text` using that Model directly. `model_identifier` is the Model's technical identifier, the same string `CREATE VIRTUAL TABLE` accepts for `model`. The word `default` is not accepted. A path or a URL fails the statement. An unknown identifier fails the statement. `role` is `query` or `passage`. The two functions are separate because a table name and a model id are both text. Each returns the float32 BLOB from REQ-69. Do not reopen.
- **REQ-45** Each Model has a query prefix and a passage prefix. Either string may be empty. The prefixes are hardcoded with the Model and are not stored in the database. For the built-in default model they are in the code. For a model linked at build time, the provider of that model also provides them (REQ-23). `CREATE VIRTUAL TABLE` does not take prefix arguments. RAGite adds the passage prefix when it embeds a Chunk, and the query prefix when it embeds a `MATCH` query. FTS indexes the Chunk text and searches the query text without a prefix (REQ-61). The prefix is not part of the stored text or of the text passed to an embed function. `ragite_embed` and `ragite_embed_model` require the `role` argument and add that prefix. Omitting `role`, or any value other than `query` or `passage`, fails the statement. Version 1 has no further prefix role. Do not reopen.
- **REQ-46** A version 1 build contains one Model. The default Model is chosen at build time. `list_models()` and `get_default_model()` return a model description: a human-readable name, and a technical identifier `name@revision`. The revision is the full commit SHA of the bundled weights for a Hugging Face model, or the revision the provider supplied for a model linked at build time. The caller does not pass the revision. `list_models()` returns that one Model. `get_default_model()` returns it as the build's default. Insert and embed take the description and use its technical identifier. In SQL, `ragite_list_models()` and `ragite_default_model()` return the columns `name` and `technical_id`. `CREATE VIRTUAL TABLE` takes `technical_id` when `model` is present, and may omit `model` (REQ-62). `ragite_embed_model` takes `technical_id`. The word `default` is not a model name. The value stored with the Table is the technical identifier. A later build with the same name and a different revision does not embed or search that Table. `SELECT` of stored columns still works. Do not reopen.
- **REQ-47** `ragite_list_models()` and `ragite_default_model()` work after the extension is loaded, before any RAGite table exists. They describe the Model built into that extension. They do not read a database table. `ragite_embed_model` also works before any RAGite table exists (REQ-77). Do not reopen.
- **REQ-67** `ragite_table_model(table_name)` returns the technical identifier stored for that Table. It works when that Model cannot be loaded. A name that is not a RAGite Document table fails the statement. `ragite_table_model` and `ragite_embed` match that name as SQLite matches an identifier: ASCII letters differ only by case, a schema qualifier follows the same rule, and the stored spelling is the name of the shadow tables. Do not reopen.
- **REQ-44** `ragite_embed` and `ragite_embed_model` fail when `text` has no characters. They do not return a vector. A text that is only whitespace is embedded as given. Do not reopen.
- **REQ-54** When RAGite fails a statement, the SQLite error message names the check that failed. When the offending value is short, the message includes it: the format, the model identifier, the role, the table name, or the `schema_name`. The message leaves out document text, chunk text, vectors, and prefixes. REQ-53 is this rule for a DoclingDocument that fails its check. `DROP TABLE <name>_chunks` follows REQ-90. Do not reopen.
- **REQ-69** `ragite_embed` and `ragite_embed_model` return a BLOB of little-endian IEEE-754 float32 values, one per dimension, with no header. The BLOB length is 4 times the Model's dimension. That is the byte layout sqlite-vec stores. Do not reopen.
- **REQ-77** `ragite_embed_model` works after the extension loads, before any RAGite table exists. It uses the built-in Model named by the technical identifier and does not read a database table. Do not reopen.
- **REQ-81** `ragite_embed`, `ragite_embed_model`, `ragite_list_models`, `ragite_default_model`, and `ragite_table_model` are deterministic. A generated column or an index may call them. Do not reopen.

### 4.7 Use from other languages

- **REQ-18** The extension can be loaded from Python (`sqlite3` with `enable_load_extension`), Go and Node.js. Distribution is planned as prebuilt binaries per platform with thin pip/npm/Go packages, as sqlite-vec does. (Publishing is not part of the first version.)
- **REQ-87** The load entry point is `sqlite3_ragite_init`. `SELECT load_extension('ragite')` loads that symbol. Do not reopen.

### 4.8 Bundled sqlite-vec

- **REQ-104** Loading the extension registers sqlite-vec 0.1.9 by calling `sqlite3_vec_init` before the `ragite` module. After `load_extension`, that connection has the scalar functions `vec_version`, `vec_debug`, `vec_distance_l2`, `vec_distance_l1`, `vec_distance_hamming`, `vec_distance_cosine`, `vec_length`, `vec_type`, `vec_to_json`, `vec_add`, `vec_sub`, `vec_slice`, `vec_normalize`, `vec_f32`, `vec_bit`, `vec_int8`, `vec_quantize_int8`, and `vec_quantize_binary`, and the modules `vec0` and `vec_each`. `vec_version()` returns `v0.1.9`. `vec_npy_file`, `vec_npy_each`, `vec_static_blob_from_raw`, `vec_static_blobs`, and `vec_static_blob_entries` are absent. The shadow table `<name>_vec` remains the internal `vec0` table in section 5.1. Do not reopen.

## 5. Source linkage: documents and chunks

Rules that apply to every format are in section 5.1. Each media type has its own subsection. When one requirement covers more than one format, each subsection states the part that applies there, under the same ID.

**Constraint. Chunkers stay independent.** The plain-text, Markdown, HTML, and Docling chunkers are separate. Each one may contain its own scan of newlines and blank lines, even when another chunker has a similar scan. They do not share that scan. The split of a unit that is longer than the length limit may stay one shared function, because REQ-70 is the same rule for every format and the copied scalars stay inside that unit. Do not reopen.

### 5.1 Common to every format

Draft schema of one Table, stored in the same database file. These names are the column shape. On disk each shadow table is prefixed by the Document table's name (`docs_documents`, `docs_chunk_rows`, `docs_vec`, `docs_fts` for a table named `docs`), so several Tables can share a file. The public companion keeps the name `<name>_chunks`, so the chunk shadow table is `<name>_chunk_rows`. The companion and the shadow tables are created in the virtual table's own database, including `temp` and an attached database (REQ-76). Do not reopen.

```text
<name>_documents(doc_id, source, text, format, hash)
<name>_chunk_rows(chunk_id INTEGER PRIMARY KEY, doc_id, start, end, heading_path, text, meta_json)
<name>_vec   -- sqlite-vec vec0, embedding float[dimension] distance_metric=cosine, rowid = chunk_id
<name>_fts   -- FTS5 table, rowid = chunk_id
```

- **REQ-76** The companion and the shadow tables are created in the same database as the virtual table. `main`, `temp`, and an attached database all work. Their names are unqualified inside that database. Do not reopen.
- **REQ-19** Every chunk records its document, its `start` and `end`, and its heading path. `start` and `end` follow REQ-31 and the format's subsection. The heading path is a JSON array of strings, outermost heading first (REQ-56). For every format other than a DoclingDocument, `meta_json` is null (REQ-20). Do not reopen.
- **REQ-31** The Document stores the inserted `text` and the Document table returns it. `start` and `end` count Unicode scalar values, the same units as a Rust `char`. They do not count UTF-8 bytes or grapheme clusters. The format subsections say what text those offsets slice. Do not reopen.
- **REQ-33** RAGite chooses where to cut. The caller does not pass a chunk size. A Chunk stays inside one section. A unit longer than the length limit is split by that limit, and the overlap stays inside that unit (REQ-70). The length limit and the overlap are fixed for version 1 and chosen in the same benchmark as the default model (section 8.1). Each format's cuts are in sections 5.2–5.5. Do not reopen.
- **REQ-70** When RAGite splits a paragraph, code block, or Docling item, it copies a run of Unicode scalar values from the end of one piece onto the start of the next. Both Chunk texts include that run, and their `start` and `end` ranges overlap. The first piece has no copied prefix. The overlap stays inside that paragraph, block, or item. The benchmark chooses how many scalars are copied. Do not reopen.
- **REQ-52** Chunk ids increase in the order RAGite emits Chunks. A query with no `MATCH` lists a Document's Chunks in that order (REQ-42). Sections 5.2–5.5 give that order for each format. Do not reopen.
- **REQ-98** RAGite picks the chunker from the Document's stored `format`. `text/markdown`, `text/html`, and `application/vnd.docling.document+json` are cut by sections 5.3–5.5. Any other stored format is cut as `text/plain` (section 5.2), and that is not an error. In version 1 REQ-32 accepts only those four types, so the only other format is `text/plain` itself. A later version that accepts a new type without its own cut rules cuts it as plain text.
- **REQ-21** The vector table and the FTS5 table use `rowid = chunk_id`, so a KNN search (`WHERE embedding MATCH ?`) joined with that Table's chunks and documents yields text, source and position.
- **REQ-28** The full-text table uses the FTS5 `unicode61` tokenizer with no stemming, for every language in the Table. Do not reopen.
- **REQ-95** The plain-text, Markdown, HTML, and Docling chunkers accept any Unicode text and finish. A plain-text Chunk, a Markdown paragraph or code piece, an HTML paragraph or loose-text piece, and a Docling item piece each hold at most 512 Unicode scalars. A Markdown heading (REQ-50), an HTML heading (REQ-50), and an HTML comment (REQ-79) stay one Chunk when the text is longer. When a unit is split, the next piece starts 64 scalars before the previous piece ends, and that previous piece is 512 scalars (REQ-70). Plain-text and Markdown Chunk text is the exact source slice. A Chunk does not invent U+0000: HTML has that scalar only inside the Chunk's source span, and a Docling Chunk has it only when the JSON contains that scalar or a `\u0000` escape. A failed Docling check names that check and does not repeat a long document (REQ-54). The fuzz targets under `fuzz/` check these properties. They are not part of normal CI.

### 5.2 `text/plain`

- **REQ-31** For plain text, `start` and `end` count in the stored text and the Chunk text is the exact slice between those offsets.
- **REQ-19** A plain-text Chunk stores heading path `[]`.
- **REQ-60** A newline is `\n` or `\r\n`. A blank line is a line ending, optional spaces or tabs, then another line ending. Stored text and exact slices keep the original characters, including `\r`. Do not reopen.
- **REQ-55** A paragraph break is a blank line (REQ-60). A single newline stays inside the paragraph. The blank line itself is not a Chunk. `text/plain` emits one Chunk per paragraph. A paragraph longer than the length limit is split by that limit, and the overlap stays inside that paragraph (REQ-70).
- **REQ-52** Chunks are emitted in paragraph order.

### 5.3 `text/markdown`

- **REQ-31** For Markdown, `start` and `end` count in the stored text and the Chunk text is the exact slice between those offsets.
- **REQ-71** A Markdown heading is a line that may start with up to three spaces, then one to six `#` marks, a space, and a title. Closing `#` marks may follow. Seven or more `#` marks, or no space after the marks, leave the line as a normal paragraph line. The heading Chunk is the exact line (REQ-50). Do not reopen.
- **REQ-50** Each heading is one Chunk, including when the Document has no other text. Its Chunk text is the exact heading line. The length limit does not split one heading into several Chunks. The heading path of a heading Chunk is the headings above it. Text that follows a heading is further Chunks, and their heading path includes that heading. A heading whose text has no characters does not become a Chunk.
- **REQ-56** Each string in `heading_path` is the heading's title. The leading `#` marks, the space after them, a line break, and any closing `#` marks stay out of the path. The heading Chunk's own text remains the exact line (REQ-50).
- **REQ-55** The text between headings is cut at blank lines (REQ-60). A fenced code block is one paragraph (REQ-58). An indented code block is one paragraph (REQ-63). A paragraph longer than the length limit is split by that limit, and the overlap stays inside that paragraph (REQ-70).
- **REQ-58** A fence that opens with `` ``` `` or `~~~` and runs through its matching close (REQ-83) is one paragraph. Lines inside it are not headings, and blank lines inside it are not paragraph breaks. The Chunk text is the exact slice, fences included. A block longer than the length limit is split inside that block, and the overlap stays inside it. Do not reopen.
- **REQ-83** An opening fence is three or more backticks, or three or more tildes. A backtick fence may have an info string. The closing line is only that same marker, at least as long, with optional surrounding spaces. Tildes do not close backticks, and backticks do not close tildes. The info string stays in the exact slice. Do not reopen.
- **REQ-63** A run of lines indented by four or more spaces is one paragraph, including a blank line that sits between those lines. A `#` on an indented line is not a heading. The Chunk text is the exact slice. A block longer than the length limit is split inside that block, and the overlap stays inside it. Do not reopen.
- **REQ-52** Chunks are emitted in the order section 5.3 cuts them.

### 5.4 `text/html`

- **REQ-74** HTML element names match without regard to letter case. `script` and `style` are dropped in any letter case. `pre` keeps its spaces and line breaks in any letter case. Do not reopen.
- **REQ-31** `start` and `end` mark the span in the stored HTML that produced that piece's visible text. The Chunk text is the visible text, except for a comment (REQ-79).
- **REQ-50** Each `h1`–`h6` element is one Chunk, including when the Document has no other text. Its Chunk text is the visible text. The length limit does not split one heading into several Chunks. The heading path of a heading Chunk is the headings above it. Text that follows a heading is further Chunks, and their heading path includes that heading. A heading whose text has no characters does not become a Chunk.
- **REQ-56** Each path entry is the visible text of the heading.
- **REQ-55** One `p`, `li`, `pre`, `blockquote`, `td`, `th`, or `textarea` is one paragraph (REQ-85). A `br` stays inside it. A header cell is a paragraph Chunk. The heading path comes from `h1`–`h6`. An element that is not `script` or `style`, not a heading, and not one of those paragraph elements is walked into, and loose text directly inside it is one paragraph. Attribute values, including `alt` and `title`, are not Chunk text. A paragraph's Chunk text leaves out descendant headings and paragraph elements (REQ-59). A paragraph longer than the length limit is split by that limit, and the overlap stays inside that paragraph (REQ-70).
- **REQ-59** An HTML paragraph's Chunk text includes its inline content, such as `a`, `em`, `strong`, `span`, `code`, and `br`. It leaves out the text of a descendant heading or paragraph element. Those descendants are their own Chunks. Do not reopen.
- **REQ-68** Outside `pre` and `textarea`, a paragraph's visible text collapses each run of whitespace to one space and drops whitespace at the ends. Inside `pre` and `textarea`, spaces and line breaks stay as they are. Do not reopen.
- **REQ-85** A `textarea` is one paragraph, and it keeps spaces and line breaks as `pre` does. A `title` element is a container. Loose text directly inside it is one paragraph, and that text collapses whitespace as text outside `pre` does. Do not reopen.
- **REQ-86** In an element's visible text, character references are decoded as HTML decodes them, including numeric references. A numeric reference in the C1 range uses the HTML Windows-1252 replacement, so `&#128;` is U+20AC and `&#150;` is U+2013. U+0000, a surrogate, or a code point past U+10FFFF becomes U+FFFD. An unrecognized reference stays as written. A comment body is not decoded. Do not reopen.
- **REQ-79** An HTML comment becomes one Chunk. It has no children: the comment body is not parsed as HTML. The Chunk text is the raw text between `<!--` and `-->`, and spaces and line breaks in that body stay as they are. `start` and `end` mark the whole comment in the stored HTML. A body with no characters produces no Chunk. A parent paragraph's Chunk text does not include the comment. The length limit does not split a comment. The heading path is the headings above the comment. A doctype produces no Chunk. A processing instruction produces no Chunk. Do not reopen.
- **REQ-52** Chunks are emitted in document order, and a comment is emitted where it sits.

### 5.5 `application/vnd.docling.document+json`

- **REQ-53** The statement fails and the database stays unchanged unless `text` is a JSON object whose `schema_name` is `DoclingDocument`. A JSON object with that name is stored when the walk produces no Chunks. A missing `body` or `furniture` is an empty tree. Extra fields are ignored, and the `version` string is not checked. The SQLite error message follows REQ-54 and names the failed check: the text is not JSON, the JSON value is not an object, or `schema_name` is missing or is not `DoclingDocument`. When `schema_name` is present, the message includes its value. Do not reopen.
- **REQ-49** Version 1 considers every content item. A text item, a table, a picture, and a key-value item can each become Chunks, in the body and in furniture, including a title or a section header. RAGite does not drop an item because of its label or because it belongs to furniture. An item with no characters produces no Chunk (REQ-48). A title or a section header fills the heading path of each later body Chunk in reading order, until a later heading whose level number is less than or equal to this heading's level number. A title is level 0. A section header uses its `level`, and uses level 1 when `level` is absent. The heading Chunk itself records only the headings above it. A heading whose text has no characters does not enter the path, and it still ends the headings at its level and below. Children are visited before the next sibling, so a Chunk nested under a heading follows the same rule. A furniture Chunk has a null `section` and a heading path of `[]`. Do not reopen.
- **REQ-48** An element whose text has no characters does not become a Chunk. The statement still stores the Document and the Chunks from its other elements. An element whose text is only whitespace does become a Chunk. One empty element does not fail the statement. Do not reopen.
- **REQ-51** RAGite builds one string for each content item, and `start` and `end` count Unicode scalar values in that string (REQ-20). A text item uses its `text` field. A table uses its cells in reading order: cell texts in a row are separated by a tab, and rows are separated by a newline. An empty cell is an empty field between tabs. The caption stays on the caption text item. A key-value item uses one line per pair: the key, a tab, then the value. A picture contributes no image data, placeholder, or URI. Its caption and any other text items are Chunks on their own. An item whose string has no characters produces no Chunk (REQ-48). Do not reopen.
- **REQ-97** A Docling table string is built from the cells that are present. `num_rows`, `num_cols`, and a cell's row or column index do not allocate a grid of that size. When every cell index is below the chunk length, and a declared count is used only when it is at most that length, a missing cell inside that rectangle is an empty field and a declared row with no cells is an empty row. A cell index at or above the chunk length does not insert that many empty fields: only rows that contain a cell are emitted, and a gap between two columns in one row is empty fields only when the gap itself is at most the chunk length. Unpositioned cells stay in reading order, using `num_cols` as the row width when that width is at most the chunk length.
- **REQ-20** `meta_json` is a JSON object with the keys `page`, `section`, and `element`. `page` is a page number, or null when the element has none. `section` is the section title, or null. `element` is the Docling element reference, such as `#/texts/3`, and it is how a caller finds that element in the stored JSON. `start` and `end` count Unicode scalar values in that element's text, and the Chunk text is the exact slice between them. They are not positions in the stored JSON. Do not reopen.
- **REQ-56** Each path entry is the section title as Docling stores it.
- **REQ-52** RAGite walks the body tree in child order, then the furniture tree. It visits each content item in those trees once and emits that item's Chunks before it walks the item's children. A group is only a container: it adds no Chunk, and its children are visited in order. An item that is in neither tree adds no Chunk. Pieces of a long item stay together in that order.
- **REQ-94** A `$ref` or `cref` is a JSON pointer. In each path segment, `~1` stands for `/` and `~0` stands for `~`. The walk visits the item that pointer names. Do not reopen.
- **REQ-33** One Chunk is a slice of one item's text (REQ-51). The heading path is the section path for an item in the body (REQ-49). A long item is split by the length limit, and the overlap stays inside that item (REQ-70).

## 6. Model variants

- **REQ-22** (a) **Built-in (default):** model files are embedded in the extension with `include_bytes!`, behind a Cargo feature that is on by default. Model2Vec models load with `model2vec-rs` (`StaticModel::from_bytes`), transformer models with candle.
- **REQ-23** (b) **Own model at build time:** a model path is passed to the build through an environment variable (`build.rs` or `include_bytes!(env!(...))`); the default model is switched off with `--no-default-features`. The provider of that model also provides its human-readable name, the revision of the weights, and its query prefix and passage prefix.
- **REQ-24** Version 1 does not load a model from a local path or a URL at run time. `model` is the technical identifier of the one Model built into that extension. A path or a URL fails `CREATE VIRTUAL TABLE` and fails `ragite_embed_model`. Do not reopen.
- **REQ-25** The Model is set when the Table is created and is read-only after that. The stored value is the technical identifier from REQ-46. Version 1 does not re-embed a Table in place. Search, `INSERT`, an `UPDATE` of text or format, and `ragite_embed` for that Table fail when the stored Model cannot be loaded. `SELECT` of stored columns still works. Moving the Documents onto another Model is done by creating a new Table with that Model, copying the stored rows into it with `INSERT`, and dropping the old Table. `DROP TABLE <name>` drops the Document table, the companion `<name>_chunks`, and that Table's shadow tables. Do not reopen.
- **REQ-40** `DROP TABLE <name>_chunks` fails and leaves both surfaces in place. Do not reopen.
- **REQ-90** SQLite does not copy the error text set from `xDestroy`. `DROP TABLE <name>_chunks` still fails and leaves both surfaces in place (REQ-40). The message the CLI shows is SQLite's generic logic error. Do not reopen.
- Late chunking (embed the whole document, then pool per chunk) only makes sense for transformer models, not for Model2Vec; it is a possible later option.

### 6.1 Model criteria

- **REQ-26** The default model runs well on a CPU and has a free license (MIT or Apache-2.0). English takes priority over German-only models; multilingual support is a bonus, not a requirement. Models with non-open-source terms (for example EmbeddingGemma under the Gemma terms) are excluded.
- **REQ-91** Normal CI runs a parity smoke test for the built-in Model: a handful of fixed sentences embedded through `ragite_embed_model` match committed reference vectors from the upstream Python implementation with cosine similarity of at least 0.99 each. The reference vectors are generated by `tests/parity/generate_reference.py` and are regenerated whenever the built-in Model changes.
- **REQ-92** The default Model is chosen with the benchmark in `bench/`: a preround from published MTEB retrieval scores, then for the 4-5 finalists a parity check (cosine at least 0.99 on a few hundred sentences against the PyTorch or Model2Vec reference), nDCG@10 and Recall@10 through RAGite in vector-only and hybrid mode on MTEB retrieval sets and an own frozen question set built from Docling output, and CPU costs on one fixed machine. Every result records the commit, dataset versions and the question set hash, and the report shows the Pareto front of quality against speed and size. The benchmark is not part of normal CI.

## 7. Licensing and NOTICE for models

- **REQ-27** Bundled model weights (variant a) are redistributed and keep their own license. The license texts of all bundled dependencies and models go into `THIRD_PARTY_NOTICES.md` (the NOTICE file), which ships with every artifact.

Known licenses (from the design research, to be verified against real metadata when dependencies are added): SQLite public domain; `libsqlite3-sys` MIT; sqlite-vec MIT OR Apache-2.0; candle MIT OR Apache-2.0; `model2vec-rs` MIT (LICENSE file; crates.io shows "non-standard"); `tokenizers` Apache-2.0; `text-splitter` MIT (if used). This is an assessment, not legal advice.

## 8. Open questions

### 8.1 Version 1 constants

The maintainer chose the version 1 constants. They are fixed for every caller of this build.

- **Model.** potion-retrieval-32M, a static Model2Vec model trained for retrieval, English, MIT, dimension 512. Human-readable name `potion-retrieval-32M`. Technical identifier `potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7` (the Hugging Face commit of the weights). Query prefix and passage prefix are empty. The bundled files are `config.json`, `tokenizer.json`, and `model.safetensors` from that revision. The ONNX file is not bundled.
- **Chunk length.** 512 Unicode scalars.
- **Overlap.** 64 Unicode scalars.
- **Fusion depth.** 50 Chunks from each ranking.

The table is the set that was considered. It is not a menu. Parameters, dimension, and license come from the Hugging Face model cards:

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

potion-retrieval-32M is the built-in Model. In the Model2Vec benchmark (bench/RESULTS.md) it had the best quality of the static candidates (mean nDCG@10 0.376 hybrid on SciFact, NFCorpus, FiQA, against 0.331 for potion-base-8M), it is English and MIT, and it runs as static vectors on CPU. The cost is size: the weights are ~129 MB, so the extension is ~131 MB instead of ~36 MB, and vectors take 2 KB per Chunk instead of 1 KB.

## 9. References

- Chunking: `text-splitter` (Rust, MIT; candidate to depend on), Docling `HierarchicalChunker`/`HybridChunker`.
- Hybrid search: sqlite-vec + FTS5 with Reciprocal Rank Fusion in SQL.
- Virtual tables: `sqlite-vec.c` (vec0 with `xBestIndex`/`xUpdate`), the FTS5 source (pattern for `MATCH`), `sqlite-loadable` (Rust FFI pattern; reference only, not a dependency).
- Considered and not used: `rusqlite` (its `vtab` feature is read-only), `ort` (pulls in a C++ library).

## 10. Change history

Newest first. An entry names the requirements or constraints it adds or edits, and why. A requirement or constraint marked "Do not reopen" stays closed unless an entry says it was reopened.

### 2026-10-27 — Every MATCH token is required

Reopened REQ-75 and replaced it. The previous text searched the `MATCH` string as one FTS5 phrase, so the tokens had to occur in that order and next to each other. A natural question then missed on the full-text side. REQ-75 now takes the tokens the Table's `unicode61` tokenizer produces and searches for an AND of those tokens, each quoted. A Chunk is a full-text hit only when it contains every token. Order and adjacency do not matter. `AND`, `OR`, `NOT`, `*`, and quotation marks are still not operators. A star is not a prefix query. When the tokenizer produces no tokens, the full-text side matches nothing and the statement does not fail. The vector side is unchanged. Replaced the full-text search sentence of REQ-61. The other sentences of REQ-61 stay, including that the heading path is not part of the embedding or of the FTS index. The stored full-text index already holds these tokens, so a Document does not need to be written again. TC-285 checks order, a missing token, and a query with no tokens. TC-210, TC-211, TC-212, TC-67, and TC-68 keep their results.

### 2026-10-27 — Chunkers stay independent

Added the constraint "Chunkers stay independent" in section 5. The plain-text, Markdown, HTML, and Docling chunkers keep their own scans, including newlines and blank lines. Duplicated scan code is accepted. The shared length-limit split stays, because REQ-70 is one rule. A constraint has no `REQ-n` and no test cases. No requirement was reopened.

### 2026-10-18 — `LIMIT` stays on the fused rows

Reopened the first sentence of REQ-15 and replaced it. That sentence said `xBestIndex` recognises `LIMIT`. SQLite can pass `LIMIT` and `OFFSET` to `xBestIndex` as constraints (`SQLITE_INDEX_CONSTRAINT_LIMIT` and `SQLITE_INDEX_CONSTRAINT_OFFSET`, since 3.38.0). RAGite does not consume them. Shrinking either ranking to the `LIMIT` would change the Scores (REQ-38). SQLite applies `LIMIT` to the fused rows. The rankings stay the fixed depth from section 8.1. REQ-38 already makes that depth the only cap RAGite applies. The rest of REQ-15 stays closed. TC-25 checks that `LIMIT` returns fewer fused rows and that those rows keep their Scores.

### 2026-10-18 — The extension registers sqlite-vec 0.1.9

Added REQ-104. `sqlite3_ragite_init` already called `sqlite3_vec_init`, and the user guide already used `vec_distance_cosine`, `vec_to_json`, and `vec0`. No requirement named that surface or its version. The numpy entry point is left out of the build, and the static-blob entry point is never called, so those five names stay absent. REQ-99 through REQ-103 belong to the column-filter change, so this requirement is REQ-104. TC-283 checks `vec_version()`, the registered names, and the absent entry points.

### 2026-10-18 — A join key on a RAGite column narrows the search

Reopened REQ-102 and replaced it. The previous text said SQLite does not pass a predicate from another table to `xBestIndex`. That is true of a predicate that names only the other table. It is not true of a join key. When the other table is the outer loop, SQLite offers `docs.source = pages.path` as a usable `=` on `source`, and the search runs inside that set. An inner join did not do this when the unconstrained `MATCH` cost the same as the constrained one: SQLite then keeps only the plan with fewer tables, and that plan is the unconstrained search. The same query returned different rows as a `CROSS JOIN` or a `LEFT JOIN`. REQ-99 still applies to the key. The unconstrained plan now costs more, so the inner join uses the key too. TC-279, TC-276, and TC-282 check the three join orders, the index plan, and a predicate that names only the other table.

### 2026-10-18 — A column filter selects Chunks before the rankings are cut

Removed the unnumbered sentence at the end of section 4.5: "Further `WHERE` filters, `JOIN`s with application tables and `ORDER BY` are left to SQLite." Added REQ-99, REQ-100, REQ-101, REQ-102, and REQ-103. No numbered requirement was reopened. REQ-15, REQ-38, REQ-80, and REQ-96 stay closed.

`WHERE docs MATCH 'wal mode' AND source = 'wikipedia'` asks for the best matches among Documents with that Source. The example in section 4 is that query. Each ranking keeps a fixed number of Chunks (section 8.1). Building those rankings from the whole Table and applying the Source test afterwards drops a matching Document that sits outside that depth. Equality and `IN` name the values to keep, so REQ-99 counts the depth inside that set. A Chunk that leads the restricted lists is rank 1 even when closer matches exist outside them.

The predicates that still run afterwards are separate requirements, because each has its own reason. REQ-100: `score` does not exist until the fusion has run. REQ-101: `LIMIT`, `OFFSET`, and `ORDER BY` window or reorder the fused rows, and shortening a ranking to the `LIMIT` would change Scores. REQ-102: a predicate on another table in a `JOIN` is never offered to this virtual table. REQ-103: `LIKE`, inequalities, and `BETWEEN` do not name a set of Chunk ids. The caller-facing account of all five is in the user guide, section 3, "Narrowing a search".

TESTSPEC cases TC-272 through TC-275 and TC-281 check REQ-99. TC-277 checks REQ-100. TC-278 checks REQ-101. TC-279 checks REQ-102. TC-280 checks REQ-103.
