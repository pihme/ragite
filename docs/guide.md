# ragite user guide

This guide covers everyday use of the ragite SQLite extension: storing documents, searching them, linking results back to your own data, the embedding functions, and models. Every `sql` block below is run in CI, top to bottom in one `sqlite3` session (TC-271), so the examples work as written. Lines starting with `-- expect:` are checked by that test; they are ordinary SQL comments for you.

The behaviour described here is specified in [SPEC.md](../SPEC.md); requirement ids such as REQ-75 point there.

Contents:

1. [Loading the extension](#1-loading-the-extension)
2. [Storing documents](#2-storing-documents)
3. [Searching](#3-searching)
4. [Linking results to your own data](#4-linking-results-to-your-own-data)
5. [Embedding functions](#5-embedding-functions)
6. [Models](#6-models)

## 1. Loading the extension

Start `sqlite3` in the directory that holds the extension and load it without the file suffix (`libragite.so` on Linux, `libragite.dylib` on macOS, `ragite.dll` on Windows, loaded as `./ragite`). From Python, call `conn.enable_load_extension(True)` first, then `conn.load_extension("./libragite")`.

```sql
SELECT load_extension('./libragite');
SELECT name, technical_id FROM ragite_default_model();
-- expect: "name":"potion-retrieval-32M"
```

## 2. Storing documents

### Creating a table

`CREATE VIRTUAL TABLE <name> USING ragite` creates two tables you use:

- `<name>`, the **document table**: one row per document, with the columns `doc_id`, `text`, `source`, `format`, `hash`, and `score`. You write here.
- `<name>_chunks`, the **chunk table**: read-only, one row per chunk, with the columns `chunk`, `doc_id`, `source`, `start`, `end`, `heading_path`, `meta_json`, and `score`.

It also creates internal tables, such as `<name>_documents`, `<name>_chunk_rows`, `<name>_vec`, and `<name>_fts`. They are an implementation detail: do not query or write them directly, and do not rely on their names or columns. The table uses the build's default model unless you pass `model` (see [section 6](#6-models)).

```sql
CREATE VIRTUAL TABLE docs USING ragite;
SELECT ragite_table_model('docs') AS model;
-- expect: "model":"potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7"
```

One database can hold several ragite tables. `ALTER TABLE ... RENAME` is not supported on them.

### Inserting

An `INSERT` takes the whole document as `text`. ragite stores it, cuts it into chunks, embeds each chunk, and indexes it for full-text search, all inside your transaction. `source` is your own label for the document (a path, URL, title, or the id of a row in your own table); ragite returns it with every chunk. `format` is the media type; `hash` defaults to the SHA-256 of the text. `doc_id` is assigned by ragite.

```sql
INSERT INTO docs(text, source) VALUES
  ('Plain text is cut at blank lines.

Each paragraph becomes one chunk.', 'notes.txt');

INSERT INTO docs(text, source, format) VALUES
  ('# Installation

Download the file and run the installer.

## Upgrading

Stop the service before you replace the binary.', 'manual.md', 'text/markdown');

INSERT INTO docs(text, source, format) VALUES
  ('<h1>FAQ</h1><p>The extension runs on the <b>CPU</b>; no GPU is needed.</p><script>ignored()</script>',
   'https://example.com/faq', 'text/html');

SELECT doc_id, source, format, length(hash) AS hash_len FROM docs ORDER BY doc_id;
-- expect: "doc_id":3,"source":"https://example.com/faq","format":"text/html","hash_len":64
```

The accepted formats are `text/plain` (the default), `text/markdown`, `text/html`, and `application/vnd.docling.document+json` (a [Docling](https://github.com/docling-project/docling) document as JSON). Case does not matter, and ragite stores the lowercase form. A parameter such as `; charset=utf-8` or any other type fails the statement.

A DoclingDocument is the way to bring in PDFs and Office files: convert them with Docling, then insert the JSON. Its chunks carry `meta_json` with the page, section, and the Docling element reference:

```sql
INSERT INTO docs(text, source, format) VALUES (
  '{"schema_name": "DoclingDocument",
    "texts": [
      {"self_ref": "#/texts/0", "label": "section_header", "text": "Warranty", "prov": [{"page_no": 2}]},
      {"self_ref": "#/texts/1", "label": "text", "text": "The warranty lasts two years.", "prov": [{"page_no": 2}]}
    ],
    "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}]}}',
  'contract.pdf', 'application/vnd.docling.document+json');

SELECT chunk, heading_path, meta_json FROM docs_chunks WHERE source = 'contract.pdf';
-- expect: "chunk":"The warranty lasts two years.","heading_path":"[\"Warranty\"]","meta_json":"{\"page\":2,\"section\":\"Warranty\",\"element\":\"#/texts/1\"}"
```

### How documents are chunked

Chunking is fixed for a build; there are no chunking parameters to pass. In version 1:

- A chunk holds at most **512 Unicode characters**. A longer paragraph is split, and each next piece repeats the last **64 characters** of the previous one.
- `text/plain`: one chunk per paragraph (paragraphs are separated by a blank line).
- `text/markdown`: each heading line is its own chunk; the text under it is cut at blank lines. A fenced or indented code block stays one paragraph. `heading_path` lists the headings above a chunk.
- `text/html`: each `h1`–`h6` is a chunk; `p`, `li`, `pre`, `blockquote`, `td`, `th`, and `textarea` are paragraphs. `script` and `style` are dropped; whitespace is collapsed outside `pre`.
- DoclingDocument: each text item, table, and key-value item is cut on its own; section headers fill `heading_path`.

`start` and `end` give the chunk's position in characters (Unicode scalar values, not bytes), counted from 0 with `end` exclusive. For plain text and Markdown the chunk is exactly that slice of the stored text, so `substr(text, start + 1, end - start)` gives it back:

```sql
SELECT c.chunk, c.heading_path, c.start, c.end,
       substr(d.text, c.start + 1, c.end - c.start) = c.chunk AS same
FROM docs_chunks AS c JOIN docs AS d ON d.rowid = c.doc_id
WHERE c.source = 'manual.md';
-- expect: "chunk":"Stop the service before you replace the binary.","heading_path":"[\"Installation\",\"Upgrading\"]"
-- expect: "same":1
```

For HTML, `start` and `end` mark the span of HTML that produced the visible text; for a DoclingDocument they count inside that element's text.

### Updating and deleting

`UPDATE` and `DELETE` name a document by `doc_id` (or select documents with any `WHERE`). Changing `text` or `format` re-chunks and re-embeds the document; changing only `source` or `hash` recomputes nothing. A re-chunked document gets new chunk ids. `DELETE` removes the document with its chunks, vectors, and full-text rows.

```sql
UPDATE docs SET source = 'notes/readme.txt' WHERE doc_id = 1;
UPDATE docs SET text = 'Plain text is cut at blank lines. A single newline stays in the paragraph.'
WHERE doc_id = 1;
SELECT count(*) AS chunks FROM docs_chunks WHERE doc_id = 1;
-- expect: "chunks":1
```

### Transactions

All writes of a statement happen in your transaction. Wrap bulk loads in one transaction for speed, and `ROLLBACK` undoes everything, including the chunks, vectors, and full-text rows:

```sql
BEGIN;
INSERT INTO docs(text, source) VALUES ('A draft that will be rolled back.', 'draft.txt');
DELETE FROM docs WHERE source = 'notes/readme.txt';
ROLLBACK;
SELECT (SELECT count(*) FROM docs WHERE source = 'draft.txt') AS drafts,
       (SELECT count(*) FROM docs WHERE source = 'notes/readme.txt') AS kept;
-- expect: "drafts":0,"kept":1
```

## 3. Searching

### Hybrid search with MATCH

`MATCH` runs ragite's search: it embeds the query, takes the 50 nearest chunks by cosine similarity and the 50 best full-text (BM25) chunks, and fuses the two rankings with Reciprocal Rank Fusion (k = 60, equal weights). The `score` column is that fused score; higher is better, and rows come back best first.

- On `<name>_chunks`, `MATCH` returns chunks.
- On `<name>`, it returns each matching document once, scored by its best chunk.
- Use `LIMIT` to take fewer rows. Without it you get every fused row (at most the candidate depth).
- Further `WHERE` filters, joins, and `ORDER BY` work as usual in SQLite. Note that a filter such as `source = '...'` is applied after the search, to the fused candidates.

```sql
SELECT chunk, source, round(score, 4) AS score
FROM docs_chunks WHERE docs_chunks MATCH 'How do I upgrade?' LIMIT 3;

SELECT doc_id, source, round(score, 4) AS score
FROM docs WHERE docs MATCH 'Does it need a GPU?' LIMIT 1;
-- expect: "source":"https://example.com/faq"
```

The score is a rank-fusion number, not a similarity: a chunk ranked first in one list gets `1/61` (about 0.0164) from that list, and a chunk in both lists adds both terms (at most `2/61`, about 0.0328). Compare scores within one query, not across queries, and do not use them as an absolute threshold.

### What the full-text side matches

The query text is searched in the full-text index **as one exact phrase** (REQ-75). FTS5 operators (`AND`, `OR`, `NOT`, `*`, quotes) are plain text, and there is no stemming (`install` does not match `installation`). A natural-language question rarely appears word for word in a document, so for such queries the result is effectively the vector ranking; the full-text side helps when the query quotes the document, such as an error message, a name, or a code identifier:

```sql
SELECT chunk, round(score, 4) AS score
FROM docs_chunks WHERE docs_chunks MATCH 'replace the binary' LIMIT 1;
-- expect: "chunk":"Stop the service before you replace the binary."
```

## 4. Linking results to your own data

`source` is yours: store whatever lets you find the original, and keep everything else (URL, title, author, permissions) in your own tables. Two common patterns:

**Join on `source`.** Use your own key as `source` and join it back:

```sql
CREATE TABLE pages (id INTEGER PRIMARY KEY, url TEXT, title TEXT);
INSERT INTO pages(id, url, title) VALUES
  (10, 'https://example.com/billing', 'Billing'),
  (11, 'https://example.com/shipping', 'Shipping');

INSERT INTO docs(text, source) VALUES
  ('Invoices are sent on the first day of each month.', 'page:10'),
  ('Orders ship within two business days.', 'page:11');

SELECT p.title, p.url, c.chunk, round(c.score, 4) AS score
FROM docs_chunks AS c
JOIN pages AS p ON c.source = 'page:' || p.id
WHERE docs_chunks MATCH 'When will my order arrive?'
LIMIT 3;
-- expect: "title":"Shipping","url":"https://example.com/shipping"
```

**Keep a mapping from `doc_id`.** `INSERT` assigns `doc_id`, which `last_insert_rowid()` returns right after the insert; store it in your table. Every chunk carries its `doc_id`, and `rowid` on the document table is `doc_id` (a direct lookup):

```sql
CREATE TABLE articles (id INTEGER PRIMARY KEY, title TEXT, path TEXT, doc_id INTEGER);
INSERT INTO docs(text, source) VALUES ('Back up the database before every upgrade.', 'kb/backup.md');
INSERT INTO articles(title, path, doc_id) VALUES ('Backups', 'kb/backup.md', last_insert_rowid());

SELECT a.title, a.path, c.chunk, c.heading_path
FROM docs_chunks AS c
JOIN articles AS a ON a.doc_id = c.doc_id
WHERE docs_chunks MATCH 'backup before upgrading'
LIMIT 1;
-- expect: "title":"Backups","path":"kb/backup.md"
```

To show a chunk in context, read the whole document by `doc_id`, and use `start`/`end` to highlight it (plain text and Markdown; see [chunking](#how-documents-are-chunked)):

```sql
SELECT d.source, d.text AS document, c.start, c.end
FROM docs_chunks AS c JOIN docs AS d ON d.rowid = c.doc_id
WHERE docs_chunks MATCH 'Stop the service' LIMIT 1;
-- expect: "source":"manual.md"
```

Remember to delete the ragite document when you delete the row it belongs to (for example in your application code or with a trigger).

## 5. Embedding functions

ragite exposes the model it uses, so you can embed text for your own purposes:

- `ragite_embed(table, text, role)` embeds with the model of that ragite document table.
- `ragite_embed_model(technical_id, text, role)` embeds with a model by its technical id; it works before any table exists.
- `role` is `'query'` or `'passage'`. It selects the model's query or passage prefix, as ragite uses them for searches and chunks. potion-retrieval-32M has empty prefixes, but other models may not, so pass the role that matches the use.

Both return a BLOB of little-endian float32 values, one per dimension (512 for potion-retrieval-32M, so 2048 bytes), the layout sqlite-vec uses. Both are deterministic, so you can use them in generated columns and indexes. Empty text fails.

```sql
SELECT length(ragite_embed('docs', 'hello world', 'passage')) AS bytes,
       length(ragite_embed_model('potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7', 'hello world', 'query')) AS bytes_too;
-- expect: "bytes":2048,"bytes_too":2048
```

Uses:

**Similarity between two texts.** The extension also loads sqlite-vec's scalar functions, such as `vec_distance_cosine` (0 = same direction):

```sql
SELECT round(vec_distance_cosine(ragite_embed('docs', 'reset my password', 'query'),
                                 ragite_embed('docs', 'How can I change my password?', 'passage')), 4) AS close,
       round(vec_distance_cosine(ragite_embed('docs', 'reset my password', 'query'),
                                 ragite_embed('docs', 'Orders ship within two business days.', 'passage')), 4) AS far,
       vec_distance_cosine(ragite_embed('docs', 'reset my password', 'query'),
                           ragite_embed('docs', 'How can I change my password?', 'passage'))
       < vec_distance_cosine(ragite_embed('docs', 'reset my password', 'query'),
                             ragite_embed('docs', 'Orders ship within two business days.', 'passage')) AS closer;
-- expect: "closer":1
```

**Your own vector index**, for example over product titles that you do not want as ragite documents:

```sql
CREATE TABLE products (id INTEGER PRIMARY KEY, title TEXT);
INSERT INTO products(title) VALUES ('Stainless steel water bottle'), ('Wireless keyboard'), ('Hiking backpack');
CREATE VIRTUAL TABLE product_vec USING vec0(embedding float[512] distance_metric=cosine);
INSERT INTO product_vec(rowid, embedding)
  SELECT id, ragite_embed('docs', title, 'passage') FROM products;

SELECT p.title FROM product_vec AS v JOIN products AS p ON p.id = v.rowid
WHERE v.embedding MATCH ragite_embed('docs', 'something to drink from', 'query') AND k = 1;
-- expect: "title":"Stainless steel water bottle"
```

**Precomputed or exported vectors.** Store the BLOB once (for example a frequent query, or vectors for another tool) and reuse it; `vec_to_json` from sqlite-vec turns it into a JSON array:

```sql
CREATE TABLE saved_queries (q TEXT PRIMARY KEY, v BLOB);
INSERT INTO saved_queries VALUES ('upgrade', ragite_embed('docs', 'How do I upgrade?', 'query'));
SELECT json_array_length(vec_to_json(v)) AS dims FROM saved_queries;
-- expect: "dims":512
```

In Python, `struct.unpack(f"<{len(blob) // 4}f", blob)` or `numpy.frombuffer(blob, "<f4")` reads the BLOB.

## 6. Models

### The built-in model and its technical id

A build of ragite contains exactly **one** model. The release build embeds potion-retrieval-32M (a static [Model2Vec](https://github.com/MinishLab/model2vec) model, English, MIT, 512 dimensions). Its **technical id** is `name@revision`, where the revision is the Hugging Face commit of the weights:

```sql
SELECT name, technical_id FROM ragite_list_models();
-- expect: "technical_id":"potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7"
```

A table stores the technical id of its model when it is created and keeps it; the model of a table cannot be changed afterwards. Pin the id explicitly if you want a mismatch to be loud rather than silent:

```sql
CREATE VIRTUAL TABLE pinned USING ragite(model='potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7');
SELECT ragite_table_model('pinned') AS model;
-- expect: "model":"potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7"
```

`model='default'`, an empty value, a file path, or a URL fail. An id that this build does not contain fails too.

### Opening a database with a different build

If a database was indexed with a model that the loaded extension does not contain (for example a table from ragite 0.2, which used potion-base-8M with 256 dimensions), `SELECT` of the stored columns still works, but `MATCH`, `INSERT`, an `UPDATE` of `text` or `format`, and `ragite_embed` on that table fail. `ragite_table_model('<name>')` tells you which model a table needs.

### Switching to another model (re-indexing)

Vectors from different models are not comparable, so switching means re-indexing. ragite does not re-embed a table in place; create a new table, copy the documents with `INSERT ... SELECT`, and drop the old one:

```sql
CREATE VIRTUAL TABLE docs_new USING ragite;
INSERT INTO docs_new(text, source, format, hash) SELECT text, source, format, hash FROM docs;
SELECT (SELECT count(*) FROM docs) = (SELECT count(*) FROM docs_new) AS copied;
-- expect: "copied":1
DROP TABLE docs_new;
```

`DROP TABLE <name>` removes the document table, its chunk table, and its internal tables. New `doc_id` values are assigned in the new table, so update any mapping you keep (see [section 4](#4-linking-results-to-your-own-data)). Copy from a table whose model can no longer be loaded works too, because it only reads stored columns. Because a table cannot be renamed, give the new table its final name, or keep the old and new names in your application configuration.

### Using a different model

Version 1 does **not** load a model at run time, from a path, a URL, or the Hugging Face hub. A different model is chosen **at build time**:

- The built-in model can be replaced by a Model2Vec model directory (`config.json`, `tokenizer.json`, `model.safetensors`) when building: build with `--no-default-features` and set `RAGITE_MODEL_PATH` to that directory, `RAGITE_MODEL_NAME` and `RAGITE_MODEL_REVISION` to the name and revision that make up its technical id, and `RAGITE_QUERY_PREFIX` / `RAGITE_PASSAGE_PREFIX` to the prefixes from the model card (may be empty). The dimension comes from the weights.

  ```bash
  RAGITE_MODEL_PATH=/path/to/potion-base-32M \
  RAGITE_MODEL_NAME=potion-base-32M \
  RAGITE_MODEL_REVISION=1e5a03f8eeb2c98b928fbbd846f22f816360919f \
  RAGITE_QUERY_PREFIX= RAGITE_PASSAGE_PREFIX= \
  cargo build -p ragite-sqlite --release --no-default-features
  ```

  The resulting extension reports `potion-base-32M@1e5a03f8…` as its one model and default. `bench/build_model.sh` does this for a Hugging Face repo.
- Transformer models (for example bge-small or e5) are planned through candle but are **not supported yet**.

Such a build is a separate extension file. Tables created with it store its technical id and can only be searched with it.
