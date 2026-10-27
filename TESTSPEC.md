# Test specification

Status: version 1. The cases in this file are implemented and run in CI. Cases carry stable IDs (`TC-n`). Tests cite these IDs, in the test name or a comment. A test may cite more than one case. New cases get the next free number. Removed numbers are not reused.

Each case checks the requirement in its heading. Headings follow [SPEC.md](SPEC.md). Terms are in [GLOSSARY.md](GLOSSARY.md). This file does not add requirements. The case IDs are TC-1 through TC-283, and TC-285.

## Conventions

Tests stay offline. Documents, hashes, and model bytes are synthetic fixtures or the weights compiled into the build under test. No accounts, tokens, or live services.

Unless a case says otherwise:

- The database is a fresh SQLite database, the extension is loaded, and the table is `docs` with companion `docs_chunks`.
- `start` is inclusive and `end` is exclusive, in Unicode scalar values (Rust `char` indexes). A one-scalar chunk at the beginning has `start` 0 and `end` 1. Where the spec says the chunk text is the exact slice, that text equals the scalars in that half-open range.
- "Fails" means the SQL statement returns an error. A failed write leaves the stored rows unchanged, except that ids already allocated stay consumed ([REQ-84](SPEC.md)).
- `L` is the version 1 chunk length limit in Unicode scalars, `N` is the overlap in scalars, and `D` is the number of Chunks taken from each ranking before fusion. All three come from SPEC section 8.1. Cases that name them read those recorded values. They do not hard-code a guess. Until section 8.1 records a value, that case is not runnable, and its ID stays reserved.
- The build's one Model is called the built-in Model below. Its human-readable name and `name@revision` come from `ragite_list_models()`, not from a literal in the test.

## REQ-1

- **TC-1** Chunking, embedding, and search fusion live in a Rust library that does not depend on `libsqlite3-sys`.
- **TC-2** The SQLite extension calls that library for chunking, embedding, and fusion.

## REQ-2

- **TC-3** The distributed artifact is a loadable SQLite extension.
- **TC-4** The extension's FFI is a thin layer on `libsqlite3-sys`.

## REQ-3

- **TC-5** `CREATE VIRTUAL TABLE docs USING ragite` accepts `INSERT` and `SELECT`.

## REQ-4

- **TC-6** A transformer Model is executed through candle. A Model2Vec Model is loaded with `model2vec-rs` (`StaticModel::from_bytes`).
- **TC-7** The extension and the Rust library do not link ONNX Runtime or another C++ inference library.

## REQ-5

- **TC-8** Inserting a Document and running `MATCH` succeed on a machine with no GPU.

## REQ-6

- **TC-9** `Cargo.toml` sets `license = "MIT OR Apache-2.0"`, and the tree contains `LICENSE-MIT` and `LICENSE-APACHE`.
- **TC-10** Every dependency, and every model weight compiled into the extension, has a permissive license compatible with MIT OR Apache-2.0. The license is read from that package's own metadata.

## REQ-7

- **TC-11** A Document is accepted only when its media type is `text/plain`, `text/markdown`, `text/html`, or `application/vnd.docling.document+json`.

## REQ-8

- **TC-12** The extension and the Rust library expose no parser for PDF, Office, or scanned pages, no LLM call, and no evaluation runner.

## REQ-9

- **TC-13** `CREATE VIRTUAL TABLE docs USING ragite` creates `docs`, `docs_chunks`, `docs_documents`, `docs_chunk_rows`, `docs_vec`, and `docs_fts` in that database. `docs_documents` has columns `doc_id`, `source`, `text`, `format`, `hash`. `docs_chunk_rows` has `chunk_id INTEGER PRIMARY KEY`, `doc_id`, `start`, `end`, `heading_path`, `text`, `meta_json`.
- **TC-14** A second table `notes` gets `notes_chunks`, `notes_documents`, `notes_chunk_rows`, `notes_vec`, and `notes_fts`, and does not reuse the `docs_` shadow tables.
- **TC-15** Creating `docs` does not create shadow tables whose names start with `docs_chunks_`.
- **TC-16** When `model` is present, its value is a technical identifier `name@revision`, and that string is what the Table stores.

## REQ-10

- **TC-17** After `INSERT`, `docs_documents` has the Document, `docs_chunk_rows` has its Chunks, `docs_vec` has one row per Chunk, and `docs_fts` has one row per Chunk. The vec and FTS `rowid` values equal `chunk_id`.

## REQ-11

- **TC-18** `BEGIN`, `INSERT`, `ROLLBACK` leaves no Document, Chunk, vector, or FTS row from that statement.
- **TC-19** `BEGIN`, an `UPDATE` that changes text, `ROLLBACK` restores the previous Chunks, vectors, and FTS rows.
- **TC-268** After a rolled-back `INSERT`, `MATCH` on `docs_chunks` does not return the rolled-back Chunk and `docs_fts` has no hit for its text. After a rolled-back `UPDATE`, `MATCH` returns the old text and `docs_fts` has no hit for the new text. After a rolled-back `DELETE`, `MATCH` on `docs` returns the Document again with a Score.

## REQ-12

- **TC-20** `UPDATE` of `text` deletes that Document's old Chunks, vectors, and FTS rows, then writes Chunks, vectors, and FTS rows for the new text. Another Document in the table is untouched.
- **TC-21** `UPDATE` of `format` from `text/plain` to `text/markdown`, without changing `text`, keeps the stored text and replaces the Chunks with the Markdown cuts of that same text.
- **TC-22** `UPDATE` of `format` to `text/csv` fails. The Document text, format, Chunks, vectors, and FTS rows stay as they were.

## REQ-13

- **TC-23** `UPDATE` of only `source`, and `UPDATE` of only `hash`, leave Chunk ids, chunk text, vectors, and FTS rows unchanged.

## REQ-14

- **TC-24** `DELETE` of one Document removes that Document and its Chunk, vector, and FTS rows, and leaves every other Document in place.

## REQ-15

- **TC-25** On `docs` and on `docs_chunks`, a `MATCH` query embeds the query and returns fused rows. `LIMIT` reduces how many of those rows come back. Each row it returns keeps the Score that row has in the same query without `LIMIT`.
- **TC-26** The fused Score uses Reciprocal Rank Fusion with k = 60 and equal weights. A test of the fusion function with known ranks matches `1/(60+rank)` summed across the lists the Chunk appears in.
- **TC-27** The caller has no argument for weights, for k, or for candidate depth. Two searches of the same table and query produce the same Scores when the stored Chunks are unchanged.
- **TC-28** A Chunk that is present in only one of the two ranked lists still receives a Score and can appear in the result.
- **TC-29** `MATCH` on `docs` returns one row per Document. That row's Score is the Score of the Document's best Chunk, including when the Document has several Chunks with lower Scores.
- **TC-30** `MATCH` on `docs_chunks` returns one row per fused Chunk.
- **TC-31** A `SELECT` without `MATCH` has `score` NULL on both surfaces.
- **TC-32** Once `D` is recorded, each list contributes at most `D` Chunks, and both lists use that same `D`.

## REQ-16

- **TC-33** `docs_chunks` exposes `chunk`, `doc_id`, `source`, `start`, `end`, `heading_path`, `meta_json`, and `score`.
- **TC-34** `heading_path` is a JSON array of strings, outermost first.
- **TC-35** A plain-text Chunk has `heading_path` `[]`. A Chunk whose format is not a DoclingDocument has `meta_json` NULL.
- **TC-258** `heading_path_json` of an empty path is `[]`. Titles `A` then `B` encode as `["A","B"]`. A title `say "hi"` encodes as `["say \"hi\""]`.

## REQ-17

- **TC-36** `ragite_embed('docs', text, 'passage')` returns the same BLOB as the vector stored for a Chunk whose text is `text`.
- **TC-37** `ragite_embed('docs_chunks', text, 'passage')` fails. `ragite_embed('missing', text, 'passage')` fails, and the error includes `missing`.
- **TC-38** `ragite_embed_model` of the built-in technical identifier, the same text, and `'passage'` returns the same BLOB as `ragite_embed` on a table bound to that Model.
- **TC-39** `ragite_embed_model('default', 'hello', 'passage')` fails. A file path fails. A URL fails. An unknown `name@revision` fails. Each error includes that short value.
- **TC-40** The two functions stay distinct: the Document table name is not accepted by `ragite_embed_model`, and a technical identifier is not accepted by `ragite_embed` as a table name.

## REQ-18

- **TC-41** Python's `sqlite3`, with `enable_load_extension`, loads the extension and runs `CREATE`, `INSERT`, and `SELECT`.
- **TC-42** A Go program loads the extension and runs `CREATE`, `INSERT`, and `SELECT`.
- **TC-43** A Node.js program loads the extension and runs `CREATE`, `INSERT`, and `SELECT`.
- **TC-44** Version 1 does not require a published pip, npm, or Go package. Those load tests link the built extension directly.

## REQ-19

- **TC-45** Every Chunk row has the Document's `doc_id`, a `start`, an `end`, and a `heading_path`. `start` and `end` follow the format's offset rule.
- **TC-46** Plain text, Markdown, and HTML Chunks have `meta_json` NULL. A DoclingDocument Chunk has the object from REQ-20.

## REQ-20

- **TC-47** A Docling Chunk's `meta_json` is a JSON object with the keys `page`, `section`, and `element`. `page` is the element's page number, or NULL when the element has none. `section` is the section title, or NULL. `element` is the Docling reference, such as `#/texts/3`.
- **TC-48** For a Docling item, `start` and `end` count scalars in that item's string, and the chunk text is the exact slice of that string. They are not offsets into the stored JSON.
- **TC-49** A furniture Chunk has `section` NULL and `heading_path` `[]`.

## REQ-21

- **TC-50** `docs_vec.rowid` and `docs_fts.rowid` equal `docs_chunk_rows.chunk_id` for every Chunk.
- **TC-51** A query that joins `docs_vec` to `docs_chunk_rows` and `docs_documents` on those ids returns the chunk text, the Source, and `start` and `end`. The join includes `WHERE embedding MATCH ?` with a vector BLOB from REQ-69.

## REQ-22

- **TC-52** The default Cargo feature embeds the built-in Model with `include_bytes!`. A Model2Vec file loads through `model2vec-rs`. A transformer file loads through candle.
- **TC-53** Building with `--no-default-features` omits that embedded file.

## REQ-23

- **TC-54** A build can take a model path from an environment variable and compile that file in, with the default Model feature off.
- **TC-55** That build's `ragite_list_models()` row uses the human-readable name and the revision the provider supplied. Embedding uses the query prefix and the passage prefix the provider supplied.

## REQ-24

- **TC-56** `CREATE VIRTUAL TABLE docs USING ragite(model='/tmp/weights.bin')` fails, and so does `model='https://example.com/weights.bin'`.
- **TC-57** `ragite_embed_model` of that path, and of that URL, fails.

## REQ-25

- **TC-58** After `CREATE`, `ragite_table_model('docs')` stays the same technical identifier across `INSERT` and `UPDATE`. There is no statement that re-embeds `docs` onto another Model.
- **TC-59** With an extension build that does not contain the stored Model, `SELECT text, source, format, hash FROM docs` still returns the stored rows, and `SELECT chunk, start, end FROM docs_chunks` still returns the stored Chunks.
- **TC-60** With that build, `INSERT`, `UPDATE` of text or format, `MATCH`, and `ragite_embed('docs', 'hello', 'query')` fail.
- **TC-61** Copying onto another Model is `CREATE` of a new table with the other build's technical identifier, `INSERT` of the stored text, source, format, and hash, then `DROP TABLE docs`. The new table assigns new `doc_id` values. `DROP TABLE docs` also drops `docs_chunks`, `docs_documents`, `docs_chunk_rows`, `docs_vec`, and `docs_fts`.

## REQ-26

- **TC-62** The Model recorded in SPEC section 8.1 has license MIT or Apache-2.0, and the benchmark that chose it ran on CPU. EmbeddingGemma is not that Model.
- **TC-63** The recorded choice is an English model, or a multilingual model chosen as the bonus case. A German-only model is not the recorded choice.

## REQ-27

- **TC-64** Bundled weights keep their own license text in `THIRD_PARTY_NOTICES.md`, taken from the weight's own license file.
- **TC-65** Every distributed artifact includes `THIRD_PARTY_NOTICES.md`.

## REQ-28

- **TC-66** `docs_fts` uses the FTS5 `unicode61` tokenizer and no stemmer.
- **TC-67** A Chunk whose text is `installation` is not found by the full-text query `install`.
- **TC-68** A Chunk whose text is `Install` is found by the full-text query `install`. A Chunk whose text is `café` is found by the full-text query `cafe`.
- **TC-69** An English Chunk and a German Chunk in the same table are indexed with that same tokenizer.

## REQ-29

- **TC-70** Two `INSERT`s with the same Source and the same text create two Documents and two `doc_id` values.
- **TC-71** `UPDATE` and `DELETE` with `WHERE doc_id = ?` affect that one Document.
- **TC-72** `DELETE FROM docs WHERE source = 'same'` removes every Document with that Source and leaves a Document with a different Source.

## REQ-30

- **TC-73** `CREATE VIRTUAL TABLE docs USING ragite` creates `docs_chunks`. `docs.rowid` is `doc_id`. Writes are addressed to `docs`.
- **TC-74** `INSERT`, `UPDATE`, and `DELETE` on `docs_chunks` fail, and the stored rows stay in place.
- **TC-75** `SELECT doc_id FROM docs WHERE source = 'manual.md'` returns every Document with that Source.
- **TC-76** `MATCH` on `docs` returns each matching Document once, ordered by its best Chunk. `MATCH` on `docs_chunks` returns Chunk rows.

## REQ-31

- **TC-77** `SELECT text FROM docs` returns the inserted text unchanged, including when the format is Markdown, HTML, or a DoclingDocument.
- **TC-78** For the stored text `a😀b`, the Chunk that holds the whole text has `end - start` = 3. For `e` plus U+0301, one grapheme, `end - start` = 2. The offsets are not UTF-8 byte indexes.
- **TC-79** For `text/plain` and `text/markdown`, the chunk text equals the stored scalars from `start` to `end`.
- **TC-80** For `text/html`, `start` and `end` mark the span of stored HTML that produced the piece. The chunk text is the visible text, except for a comment, whose text and span follow REQ-79.
- **TC-81** For a DoclingDocument, the offsets follow TC-48.

## REQ-32

- **TC-82** `format` is stored on the Document. `docs_chunks` has no `format` column.
- **TC-83** An `INSERT` that omits `format` stores `text/plain` when the text is `# heading`, when it is `<p>hi</p>`, and when it is a DoclingDocument JSON object.
- **TC-84** The four accepted types are stored. `application/json`, `text/csv`, and `text/html; charset=utf-8` fail.

## REQ-33

- **TC-85** `CREATE` and `INSERT` take no chunk size. `CREATE VIRTUAL TABLE docs USING ragite(chunk_size=100)` fails and leaves no table.
- **TC-86** A Chunk's text does not contain scalars from the next section. Overlap scalars stay inside the section that was split.
- **TC-87** Once `L` is recorded, a paragraph, fenced block, indented block, or Docling item longer than `L` becomes more than one Chunk. A heading or an HTML comment longer than `L` stays one Chunk. Split pieces follow TC-200.

## REQ-34

- **TC-88** `docs` returns `doc_id`, `text`, `source`, `format`, `hash`, and `score`, and `rowid` equals `doc_id`. It has no `meta_json` column.
- **TC-89** An `INSERT` that sets `hash` to `not-a-digest` stores `not-a-digest`. A second Document with the same hash and the same text is also stored.
- **TC-90** An `INSERT` of `hello` that omits `hash` stores 64 hex characters which decode to the SHA-256 of the UTF-8 bytes of `hello`.
- **TC-91** An `UPDATE` that changes `text` and does not set `hash` leaves the previous hash in place, including when that hash was computed and no longer matches the new text.
- **TC-92** An `UPDATE` that changes only `hash` leaves Chunk ids and vectors unchanged.

## REQ-35

- **TC-93** `INSERT` of `text` `''` fails and leaves no row. `UPDATE` that sets `text` to `''` fails and leaves the previous Document and its Chunks in place.
- **TC-94** `INSERT` of `text` `'   '` succeeds and stores those three spaces.

## REQ-36

- **TC-95** `INSERT` of `<script>secret</script><style>x</style>` as `text/html` stores the Document, writes no Chunk, vector, or FTS row, returns the Document for `WHERE source = 'scripts'`, and returns no row for `MATCH`.
- **TC-96** A DoclingDocument whose content items all have empty strings stores the Document and writes no Chunk. `MATCH` does not return it. `WHERE source = 'empty-items'` does.
- **TC-97** A Markdown Document whose only content is `# Only`, and an HTML Document whose only content is `<h1>Only</h1>`, each store one Chunk.

## REQ-37

- **TC-98** `INSERT INTO docs(text) VALUES ('hello')` succeeds. The Source is empty, the format is `text/plain`, and the hash is the computed SHA-256.
- **TC-99** `INSERT` of `source` `''` stores an empty Source, the same as omitting `source`.
- **TC-100** `INSERT INTO docs(source) VALUES ('orphan')` fails, and so does `INSERT INTO docs DEFAULT VALUES`.
- **TC-101** `INSERT` and `UPDATE` that set `doc_id` or `score` fail and leave the database unchanged.
- **TC-102** `UPDATE` can set `text`, `source`, `format`, and `hash` together in one statement.

## REQ-38

- **TC-103** `MATCH` without `LIMIT` returns every fused row: every fused Chunk on `docs_chunks`, and every matching Document on `docs`.
- **TC-104** `LIMIT 2` on a `MATCH` that has more than two fused rows returns two rows.
- **TC-105** Once `D` is recorded, a Chunk outside both top-`D` lists is absent even when the query has no `LIMIT`.

## REQ-39

- **TC-106** `CREATE` with a technical identifier the build cannot load fails. No Document table, companion, or shadow table remains under that name.
- **TC-107** If `docs_chunks` already exists, `CREATE VIRTUAL TABLE docs USING ragite` fails. The pre-existing `docs_chunks` remains, and no `docs_documents` is created.
- **TC-108** If `docs_fts` already exists, `CREATE VIRTUAL TABLE docs USING ragite` fails and does not leave a new `docs` or `docs_chunks`.

## REQ-40

- **TC-109** `DROP TABLE docs_chunks` fails. `docs` and `docs_chunks` both still accept `SELECT`.

## REQ-41

- **TC-110** `MATCH` rows come back with the highest Score first, on both surfaces.
- **TC-111** When two Document Scores are equal, `docs` orders those rows by `doc_id` ascending. When two Chunk Scores are equal, `docs_chunks` orders them by `doc_id` ascending, then by Chunk id ascending. A ranker test may supply the equal Scores directly.

## REQ-42

- **TC-112** With no `MATCH`, `docs` returns Documents in `doc_id` ascending order, and `score` is NULL.
- **TC-113** With no `MATCH`, `docs_chunks` returns rows in `doc_id` ascending order, then Chunk id ascending. Those Chunk ids follow emission order.
- **TC-114** `WHERE source = 'manual.md'` without `MATCH` keeps `doc_id` ascending order among the rows it returns.

## REQ-43

- **TC-115** `MATCH ''` returns no rows and does not fail, including when the stored Model cannot be loaded.
- **TC-116** `MATCH '   '` is embedded as those spaces. On a table that has Chunks and a loadable Model, it returns rows. On a table whose Model cannot be loaded, it fails.
- **TC-117** `MATCH ''` does not call the Model. A test double of the Model records no embed for that query.

## REQ-44

- **TC-118** `ragite_embed` and `ragite_embed_model` of `''` fail and return no BLOB.
- **TC-119** Both functions of `'   '` return a float32 BLOB of the length in REQ-69.

## REQ-45

- **TC-120** The built-in Model's query prefix and passage prefix are the strings shipped with that Model, and either string may be empty. They are not columns in `docs_documents` or `docs_chunk_rows`.
- **TC-121** `CREATE VIRTUAL TABLE docs USING ragite(query_prefix='query: ', passage_prefix='passage: ')` fails and leaves no table.
- **TC-122** Stored chunk text equals the inserted slice. It does not grow a prefix. `ragite_embed` applies the prefix for the role after it receives the text argument.
- **TC-123** The vector stored for a Chunk equals `ragite_embed('docs', chunk_text, 'passage')`. The query vector used by `MATCH` equals `ragite_embed_model(technical_id, query_text, 'query')`.
- **TC-124** `ragite_embed` and `ragite_embed_model` without a role fail. The role `document`, the role `Query`, and the role `''` fail. The error includes the offending role when one was supplied.

## REQ-46

- **TC-125** `list_models()` and `get_default_model()` return one description: a human-readable name and a technical identifier `name@revision`. In SQL, `ragite_list_models()` returns one row with columns `name` and `technical_id`, and `ragite_default_model()` returns that same row.
- **TC-126** For a Hugging Face Model, `revision` is the full commit SHA of the compiled weights: 40 hexadecimal characters. For a build-time Model, `revision` is the revision the provider supplied. The caller does not pass a separate revision argument.
- **TC-127** `CREATE` with `model` set to the `technical_id` from `ragite_default_model()` succeeds. `CREATE` with only the name portion, without `@revision`, fails.
- **TC-128** The word `default` is rejected by `CREATE` and by `ragite_embed_model`. The value stored on the Table is the `technical_id`.
- **TC-129** An extension built with the same model name and a different revision does not `INSERT` or `MATCH` against a Table that stores the old `technical_id`. `SELECT` of stored columns still returns the rows.

## REQ-47

- **TC-130** On a connection where the extension is loaded and no RAGite table exists, `ragite_list_models()` and `ragite_default_model()` return the built-in Model.
- **TC-131** Creating and filling a RAGite table does not change the row those two functions return.

## REQ-48

- **TC-132** A Docling element whose text is `''` produces no Chunk. A sibling element whose text is `Hello` still produces a Chunk, and the Document is stored.
- **TC-133** A Docling element whose text is ` ` (one space) produces a Chunk whose text is that space.
- **TC-134** One empty element does not fail the statement.

## REQ-49

- **TC-135** A body text item, a table, a picture's caption text item, and a key-value item each produce Chunks. The same four kinds produce Chunks when they sit in furniture.
- **TC-136** A title item and a section-header item produce Chunks. Their text is the heading path of the body Chunks that follow them in reading order, until a later heading of the same level or a level above. An item is kept when its label is a header, and when it belongs to furniture.
- **TC-241** Sibling headings in the body, with empty `children`, still fill `heading_path` and `meta_json` `section` for the items that follow. A title is above a level-1 section header. A level-1 header ends a previous level-1 header and keeps the title. A level-2 header sits under the open level-1 header. A missing `level` is level 1. A heading Chunk's own path lists only the headings above it. A nested child of a heading and the next sibling after that heading see the same path.
- **TC-137** An item with no characters produces no Chunk, on either tree.

## REQ-50

- **TC-138** Markdown `# Only` stores one Chunk, the heading line. HTML `<h1>Only</h1>` stores one Chunk, `Only`.
- **TC-139** Under a Markdown heading, the heading is one Chunk and the following paragraph is another. The heading Chunk's path is the headings above it. The paragraph's path includes that heading. The same split holds for `<h1>Title</h1><p>Body</p>`: chunk texts `Title` and `Body`.
- **TC-140** Once `L` is recorded, a heading whose line or visible text is longer than `L` is still one Chunk.
- **TC-141** Markdown `# ` (a mark, a space, and no title) produces no Chunk. HTML `<h1></h1>` and `<h2>   </h2>` produce no Chunk.
- **TC-255** `<h1>A</h1><h2>B</h2><h3>C</h3><h4>D</h4><h5>E</h5><h6>F</h6><p>G</p>` produces seven Chunks, `A` through `G`. The path of `A` is empty. Each later heading's path is only the headings above it. The path of `G` is `A`, `B`, `C`, `D`, `E`, `F`.
- **TC-262** In `<div><h1>T</h1></div> after`, the heading Chunk is `T` with an empty path, and the following text is a Chunk `after` whose path is `T`.

## REQ-51

- **TC-142** A Docling text item uses its `text` field as the item string.
- **TC-143** A table with cells `A`, empty, `C`, `D` in reading order, two columns, becomes the item string `A\t\nC\tD`. The caption is not appended to that string. The caption text item is its own Chunk.
- **TC-144** A key-value item with pairs (`name`, `ada`) and (`role`, `admin`) becomes `name\tada\nrole\tadmin`.
- **TC-257** Two pairs linked by `to_key`, with keys listed as `name` then `role` and values listed as `admin` then `ada`, still become `name\tada\nrole\tadmin`. Zipping the two lists in that cell order would swap the values. The link, not that zip, chooses the value.
- **TC-145** A picture item contributes no Chunk for its image bytes, a placeholder, or a URI. Its caption text item is a Chunk.

## REQ-52

- **TC-146** Chunk ids increase in emission order. A `SELECT` with no `MATCH` lists one Document's Chunks in that same order.
- **TC-147** Plain text emits a Chunk per paragraph, in paragraph order.
- **TC-148** Markdown emits Chunks in the order section 5.3 cuts them: a heading, then the paragraphs and code blocks under it, then the next heading.
- **TC-149** HTML emits Chunks in document order. In `<p>A</p><!--x--><p>B</p>`, the Chunks are `A`, then the comment, then `B`.
- **TC-150** A DoclingDocument walks the body tree in child order, then the furniture tree. A node's Chunks are emitted before its children's Chunks. Pieces of a long item stay together in that order.
- **TC-239** A group is not a Chunk. Its children are emitted in child order.
- **TC-240** A content item that is in neither the body tree nor the furniture tree adds no Chunk. An item that the walk can reach twice is still emitted once.

## REQ-53

- **TC-151** Format `application/vnd.docling.document+json` with text `not json` fails. The error says the text is not JSON and does not include the document text. No row is stored.
- **TC-152** Text `[]`, and text `"DoclingDocument"`, fail. The error says the JSON value is not an object.
- **TC-153** Text `{}` fails. The error says `schema_name` is missing.
- **TC-154** Text `{"schema_name":"Other"}` fails. The error includes `Other`.
- **TC-155** Text `{"schema_name":"DoclingDocument"}` succeeds and stores the Document when `body` and `furniture` are absent. Text `{"schema_name":"DoclingDocument","version":"9.9.9","extra":true}` also succeeds. The `version` string is not checked.
- **TC-156** A stored DoclingDocument with no content items has no Chunks.

## REQ-54

- **TC-157** A failed statement's SQLite error names the check that failed.
- **TC-158** The error includes a short offending format, model identifier, role, table name, or `schema_name`, for the checks that use those values.
- **TC-159** The error omits the document text, the chunk text, the vector bytes, and the prefix strings. A unique sentinel placed in the document text does not appear in the error.

## REQ-55

- **TC-160** Plain text `a\nb\n\nc` produces two Chunks, `a\nb` and `c`, in that order. The blank line is not a Chunk.
- **TC-161** Plain text `a\n\n\nb` produces `a` and `b`. A run of blank lines is not a Chunk.
- **TC-162** Markdown between headings uses those same blank-line cuts.
- **TC-163** A Markdown fenced block is one paragraph (TC-175). An indented code block is one paragraph (TC-186).
- **TC-164** HTML `<p>a<br>b</p>` is one Chunk. The Chunk contains `a` and `b` in that order.
- **TC-165** Each of `p`, `li`, `pre`, `blockquote`, `td`, `th`, and `textarea` produces its own paragraph Chunk. `<th>A</th><td>B</td>` produces `A` and `B`. The heading path still comes only from `h1`–`h6`.
- **TC-166** `<div>before <p>in</p> after</div>` produces a Chunk for the loose text and a Chunk `in`. The loose-text Chunk's visible text is `before after`. `<ul>`, `<ol>`, `<table>`, `<tr>`, and `<div>` are not themselves paragraph Chunks of their descendants' text.
- **TC-167** `<img alt="hidden"><p title="nope">Yes</p>` produces one Chunk, `Yes`.
- **TC-263** `<div><em>a</em> b</div>` is one Chunk, `a b`. `<div>a</div>b` is two Chunks, `a` and `b`.
- **TC-264** `<p title="a>b">c</p>` is one Chunk, `c`. The attribute value is not Chunk text.

## REQ-56

- **TC-168** Markdown `## Title` has path entry `Title`. Markdown `  ## Title ##` has a path entry that does not contain `#` or a line break, and the heading Chunk text remains the exact line.
- **TC-169** For `# A` then `## B` then `body`, the path of the `A` heading is `[]`, the path of the `B` heading is `["A"]`, and the path of `body` is `["A","B"]`.
- **TC-170** HTML path entries are the visible text of the headings. `<h1>A <em>B</em></h1>` contributes `A B`.
- **TC-171** A Docling path entry is the section title as the document stores it.

## REQ-57

- **TC-172** Every `doc_id` and every Chunk id is a positive integer.
- **TC-173** After `DELETE`, a later `INSERT` does not receive the deleted `doc_id`. After an `UPDATE` that rewrites Chunks, the new Chunk ids are not the deleted Chunk ids.
- **TC-174** Both sequences are `INTEGER PRIMARY KEY AUTOINCREMENT`. A fresh table's first `doc_id` is 1, and its first Chunk id is 1.

## REQ-58

- **TC-175** A fence from an opening ` ``` ` or `~~~` through its matching close is one Chunk. Its text is the exact slice from the first fence character through the last character of the closing marker, including the info string and the newlines inside the block, and excluding the line ending after the closing marker. Lines inside are not headings. Blank lines inside are not paragraph breaks. A document `before`, blank line, a `rust` backtick fence with an interior blank line and a line `# not a heading`, blank line, `after` yields three Chunks: `before`, that fence slice, and `after`.
- **TC-176** Once `L` is recorded, a fence longer than `L` splits inside the fence, and the overlap stays inside it.

## REQ-59

- **TC-177** `<p>See <a>this</a> <em>and</em> <code>x</code></p>` is one Chunk, `See this and x`.
- **TC-178** `<p>Before <h2>Mid</h2> after <span>end</span></p>` produces a paragraph Chunk `Before after end` and a heading Chunk `Mid`.

## REQ-60

- **TC-179** `a\r\n\r\nb` produces Chunks `a` and `b`. `a\n \nb` and `a\n\t\nb` produce Chunks `a` and `b`. A tab or space is a blank line only when it sits between two line endings.
- **TC-180** `a\rb\n\nc` produces a Chunk `a\rb`. The stored Document text still contains the `\r`.

## REQ-61

- **TC-181** The BLOB in `docs_vec` for a Chunk equals the embedding of the passage prefix followed by that Chunk's text. The query vector for `MATCH` equals the embedding of the query prefix followed by the query text.
- **TC-182** The FTS index of a Chunk matches that Chunk's text and does not match a token that appears only in a prefix, the heading path, or `meta_json`.
- **TC-183** A body Chunk under a heading whose title is absent from the body is not an FTS hit for that title. A Docling Chunk is not an FTS hit for its `element` reference when that reference is absent from the item text.

## REQ-62

- **TC-184** `CREATE VIRTUAL TABLE docs USING ragite` with no `model` argument stores the `technical_id` from `ragite_default_model()`.
- **TC-185** `CREATE` with `model='default'` fails. `CREATE` with `model=''` fails. Neither word is stored.

## REQ-63

- **TC-186** Lines `    a`, then a blank line, then `    # b` are one Chunk. The slice keeps the four spaces, the interior blank line, and `# b`. That `#` does not start a heading.
- **TC-187** Once `L` is recorded, an indented block longer than `L` splits inside the block, and the overlap stays inside it.

## REQ-64

- **TC-188** `doc_id` values are unique among Documents. Chunk ids are unique among Chunks. The same number may be both a `doc_id` and a Chunk id at the same time.
- **TC-189** Inserting a Document with three Chunks uses the next `doc_id` and the next three Chunk ids. The following Document takes the next `doc_id` and does not skip ahead by the Chunk count.

## REQ-65

- **TC-190** `SELECT rowid, doc_id FROM docs` returns equal values. `SELECT rowid FROM docs_chunks` returns the Chunk id.
- **TC-191** `SELECT chunk_id FROM docs` and `SELECT chunk_id FROM docs_chunks` fail. Neither surface declares `chunk_id`.

## REQ-66

- **TC-192** `docs_vec` is a sqlite-vec `vec0` table with `distance_metric=cosine` and `embedding float[dimension]`.
- **TC-193** The vector ranking order is cosine similarity order.

## REQ-67

- **TC-194** `ragite_table_model('docs')` returns the technical identifier stored at `CREATE`.
- **TC-195** It still returns that identifier when the loaded extension cannot load that Model.
- **TC-196** `ragite_table_model('docs_chunks')` and `ragite_table_model('missing')` fail, and the error includes that name.
- **TC-244** After `CREATE VIRTUAL TABLE Mixed USING ragite`, `ragite_table_model('mixed')` and `ragite_embed('mixed', ...)` use that table. `MAIN.Mixed` matches it as well. The returned technical identifier is the one stored for `Mixed`.

## REQ-68

- **TC-197** `<p>  Hello   \n  world  </p>` has chunk text `Hello world`.
- **TC-198** `<pre>  a \n b  </pre>` has chunk text `  a \n b  `. `<textarea>` keeps spaces and line breaks the same way.

## REQ-69

- **TC-199** `ragite_embed` and `ragite_embed_model` return a BLOB of little-endian IEEE-754 float32 values, with no header. Its length is 4 times the Model's dimension.
- **TC-259** The built-in Model's `dimension()` is the dimension recorded in SPEC section 8.1 (`crate::DIMENSION`, 512 for potion-retrieval-32M). An embedding of that Model has that many floats.

## REQ-70

- **TC-200** Once `N` is recorded, a split paragraph, code block, or Docling item copies the last `N` scalars of one piece onto the start of the next. Both Chunk texts contain that run. Their ranges overlap by `N`. The first piece has no copied prefix. The copied run does not include scalars from outside that paragraph, block, or item.

## REQ-71

- **TC-201** A line with zero to three leading spaces, then one to six `#`, then a space, then a title, is a heading. Optional closing `#` marks may follow. Stored `## Title\n\nBody` yields Chunks `## Title` and `Body`. The heading slice does not include the line ending that terminates the heading line.
- **TC-202** A line with four leading spaces before `#` is not a heading. A line with seven `#` marks is not a heading. A line `#NoSpace` is not a heading. Each of those lines stays in a normal paragraph or code chunk.

## REQ-72

- **TC-203** `TEXT/MARKDOWN`, `Text/Html`, and `APPLICATION/VND.DOCLING.DOCUMENT+JSON` are accepted and stored as the canonical lowercase types.
- **TC-204** `text/plain; charset=utf-8` fails, and the error includes that format string.
- **TC-269** A parameter fails for every accepted type, on `INSERT` and on `UPDATE`: `text/markdown; charset=utf-8`, `TEXT/HTML;charset=UTF-8`, and `application/vnd.docling.document+json; version=1`. The `INSERT` error includes the format string (lowercased). A failed `UPDATE` leaves the stored format and Chunks unchanged.

## REQ-73

- **TC-205** `NULL` `source` stores an empty Source. `NULL` `format` stores `text/plain`. `NULL` `hash` stores the computed SHA-256. Each matches the omitted-column case.
- **TC-206** `NULL` `text` on `INSERT` or `UPDATE` fails.
- **TC-207** `UPDATE` that sets `format` to NULL stores `text/plain` and re-chunks the stored text with the plain-text rules.
- **TC-208** `CREATE VIRTUAL TABLE docs USING ragite(model=NULL)` stores the built-in technical identifier.

## REQ-74

- **TC-209** `<H1>Title</H1><P>Body</P><SCRIPT>no</SCRIPT><STYLE>no</STYLE><PRE> keep </PRE>` produces a heading `Title`, a paragraph `Body`, and a pre Chunk ` keep `. It produces no Chunk for the script or the style.

## REQ-75

- **TC-210** `MATCH 'cats AND dogs'` finds a Chunk whose text is `cats AND dogs` and does not find a Chunk whose text is `cats dogs`, because the token `and` is absent.
- **TC-211** `MATCH 'NOT cats'` finds a Chunk `not cats` and does not find a Chunk whose text is only `cats`. `MATCH 'install*'` does not find `installation`.
- **TC-212** `MATCH 'say "hello" there'` does not fail. The full-text tokens are `say`, `hello`, and `there`. The vector side embeds the same characters, plus the query prefix.
- **TC-285** `MATCH 'dogs AND cats'` and `MATCH 'cats dogs'` both find a Chunk `cats AND dogs`. `MATCH 'cats dogs birds'` does not. `MATCH '***'` and `MATCH '"'` do not fail, match nothing on the full-text side, and still produce a vector Score.

## REQ-76

- **TC-213** `CREATE VIRTUAL TABLE docs USING ragite` in `main` creates the companion and the shadow tables in `main`.
- **TC-214** `CREATE VIRTUAL TABLE temp.scratch USING ragite` creates `scratch_chunks` and the shadow tables in `temp`, and does not create them in `main`.
- **TC-215** After `ATTACH`, `CREATE VIRTUAL TABLE archive.docs USING ragite` creates the companion and the shadow tables in `archive`. Their stored names are unqualified inside that database.

## REQ-77

- **TC-216** On a connection with the extension loaded and no RAGite table, `ragite_embed_model` of the built-in technical identifier returns a vector BLOB.

## REQ-78

- **TC-217** The declared types on `docs` are `doc_id INTEGER`, `text TEXT`, `source TEXT`, `format TEXT`, `hash TEXT`, and `score REAL`.
- **TC-218** The declared types on `docs_chunks` are `chunk TEXT`, `doc_id INTEGER`, `source TEXT`, `start INTEGER`, `end INTEGER`, `heading_path TEXT`, `meta_json TEXT`, and `score REAL`.

## REQ-79

- **TC-219** `<!-- raw &amp; <b>no</b>\n-->` is one Chunk whose text is ` raw &amp; <b>no</b>\n`. The body is not parsed as HTML and `&amp;` is not decoded. `start` and `end` cover the whole comment, from `<` of `<!--` through `>` of `-->`.
- **TC-220** `<!---->` produces no Chunk. `<!DOCTYPE html>` produces no Chunk. `<?note ?>` produces no Chunk.
- **TC-265** `<!DOCTYPE html><p>Hi</p>` produces one Chunk, `Hi`. The doctype is not a Chunk and does not swallow the paragraph. `<!DOCTYPE "a>b"><p>Hi</p>` is also just `Hi`: a `>` inside a quoted declaration does not end it.
- **TC-221** `<p>A<!--x-->B</p>` has a paragraph Chunk `AB` and a comment Chunk `x`. The paragraph text does not contain `x`.
- **TC-222** Once `L` is recorded, a comment longer than `L` is still one Chunk. Its heading path is the headings above it.

## REQ-80

- **TC-223** A Chunk at rank 1 of a list contributes `1/61`. A Chunk at rank 2 contributes `1/62`. A Chunk at rank 1 on both lists scores `2/61`. A Chunk at rank 1 on one list and absent from the other scores `1/61`.
- **TC-224** A Document whose Chunk Scores are `2/61` and `1/61` has Document Score `2/61`.
- **TC-256** Those same two Scores still produce Document Score `2/61` when the Chunk with `1/61` is seen first.

## REQ-81

- **TC-225** Two calls to `ragite_embed`, and two calls to `ragite_embed_model`, with the same arguments return identical BLOBs. Two calls to `ragite_list_models`, `ragite_default_model`, and `ragite_table_model` return identical rows.
- **TC-226** SQLite accepts a generated column, and an index expression, that calls `ragite_embed_model` or `ragite_default_model`.

## REQ-82

- **TC-227** On `docs`, `docs MATCH 'q'`, `text MATCH 'q'`, and `source MATCH 'q'` return the same rows and Scores. On `docs_chunks`, `docs_chunks MATCH 'q'` and `chunk MATCH 'q'` return the same rows and Scores. Column choice does not change the hybrid search. The Document surface still returns one row per Document, and the companion still returns one row per Chunk.

## REQ-83

- **TC-228** An opening fence is three or more backticks, or three or more tildes. `` ```rust `` keeps `rust` in the Chunk slice. A closing line of the same marker, at least as long, with only optional surrounding spaces, ends the fence.
- **TC-229** A longer closing marker closes a shorter opening marker of the same character. A line that adds text after the marker does not close it.
- **TC-230** A tilde line does not close a backtick fence. A backtick line does not close a tilde fence.

## REQ-84

- **TC-231** `BEGIN`, `INSERT` of a Document that has Chunks, `ROLLBACK` removes those rows. The next `INSERT` follows SQLite `AUTOINCREMENT`. On a fresh Table that next `doc_id` and Chunk id are 1.
- **TC-232** An `INSERT` of two rows whose second format is `text/csv` fails and leaves no Document. The next `INSERT` on that fresh Table receives `doc_id` 1.

## REQ-85

- **TC-233** `<textarea>  a \n b  </textarea>` is one Chunk with text `  a \n b  `.
- **TC-234** `<title>  Hello   there </title>` is one Chunk with text `Hello there`. A `title` that contains a `p` keeps the `p` as its own Chunk, and the loose text of the `title` is a separate paragraph.

## REQ-86

- **TC-235** `<p>&amp; &#65; &#x42; &unknown;</p>` has chunk text `& A B &unknown;`.
- **TC-236** `<!-- &amp; -->` has chunk text ` &amp; `. `<p>&lt;!-- not --&gt;</p>` is a paragraph whose visible text is `<!-- not -->`, and it is not a comment Chunk.
- **TC-248** `<p>&#128; &#150; &#0; &#xD800; &#1114112;</p>` has chunk text `€ – U+FFFD U+FFFD U+FFFD`. `&#128;` is U+20AC, `&#150;` is U+2013, and `&#0;`, the surrogate `&#xD800;`, and `&#1114112;` are each U+FFFD. `<!-- &#128; -->` keeps the reference as written.
- **TC-260** `&#1114111;` is U+10FFFF. A code point past U+10FFFF is U+FFFD (TC-248); this last valid code point is not.
- **TC-261** `<p>&notin x</p>` has chunk text U+00AC, then `in x`. The name `notin` needs a semicolon. Without one, only the legacy name `not` is decoded.

## REQ-87

- **TC-237** The extension exports the symbol `sqlite3_ragite_init`.
- **TC-238** `SELECT load_extension('ragite')` loads that symbol and registers the `ragite` virtual table module.

## REQ-88

- **TC-243** `CREATE VIRTUAL TABLE evil USING ragite(companion=1, parent=docs)` fails with `argument is not accepted: companion` and leaves no `evil` table. `CREATE VIRTUAL TABLE other USING ragite(parent=docs)` fails with `argument is not accepted: parent`. The companion RAGite created for `docs` still reads after that database is opened again.

## REQ-89

- **TC-242** `ALTER TABLE docs RENAME TO docs9` fails with `rename is not accepted`. `docs` still returns the inserted text, no `docs9` table exists, and `DROP TABLE docs` removes the table. `ALTER TABLE docs_chunks RENAME` fails the same way, and `docs_chunks` still reads.

## REQ-90

- **TC-245** `DROP TABLE docs_chunks` fails with SQLite's generic logic error. `docs` and `docs_chunks` both still accept `SELECT`.

## REQ-91

- **TC-246** For every sentence in `tests/parity/reference.json`, `ragite_embed_model` with the default Model and role `passage` returns a vector of the same dimension whose cosine with the reference vector is at least 0.99. The file's `model` equals the default technical identifier. The macOS and Windows CI jobs build the default extension and run this case.

## REQ-93

- **TC-247** `INSERT INTO docs(text, source) VALUES (3.7, -2.5)` stores text `3.7` and source `-2.5`. The plain-text Chunk is `3.7`. An integer `4` is stored as `4`.

## REQ-94

- **TC-249** A DoclingDocument whose text items are named `a/b` and `a~b` is reached by `$ref` values `#/texts/a~1b` and `#/texts/a~0b`. The Chunks are `slash` and `tilde`, in that order. `meta_json` `element` is the reference that was walked.

## REQ-95

- **TC-250** Every file under `fuzz/corpus/html`, `fuzz/corpus/markdown`, and `fuzz/corpus/docling` satisfies REQ-95. The markdown seeds are also checked as plain text. A Docling seed that is not a DoclingDocument passes when its error names the failed check and does not repeat a long document. A heading, an HTML comment, and a paragraph longer than 512 scalars are included, and that paragraph is split.

## REQ-96

- **TC-251** `SELECT` from `docs_chunks` with `rowid` equal to a Chunk id returns that Chunk and no other. `EXPLAIN QUERY PLAN` reports `INDEX 2:rowid`. The same equality on `docs.rowid` returns that Document. When `MATCH` is also usable, the plan stays the search (`INDEX 1:`). An empty `MATCH` combined with a `rowid` equality returns no row. `rowid = 2.5` and `rowid = '2.5'` return no row on `docs` and on `docs_chunks`, and that plan is still `INDEX 2:rowid`. `rowid = 2.0` and `rowid = '2'` return the row whose id is 2.
- **TC-252** `rowid` equal to an id that is not stored returns no row and does not fail, on `docs` and on `docs_chunks`.
- **TC-253** A query with no `rowid` equality returns every Chunk in `doc_id` order, then Chunk id order. `EXPLAIN QUERY PLAN` reports `INDEX 0:`. An inequality such as `rowid > 1` still returns the later Chunks and does not use the unique lookup.
- **TC-254** A KNN query on `docs_vec` joined to `docs_chunks` on `docs_chunks.rowid` returns that Chunk. `EXPLAIN QUERY PLAN` uses `INDEX 2:rowid` for the companion.

## REQ-97

- **TC-266** A table with `num_rows` 3 and a `num_cols` that does not fit in a grid produces one Chunk, `A` followed by two empty rows, and does not panic. A table declared as 200000 by 200000 with one cell, and a single cell positioned at row 200000 and column 200000, each produce one Chunk, `A`. A 2 by 2 table with cells only at the two corners is still `A`, a tab, a newline, a tab, and `D`.

## REQ-99

- **TC-272** Once `D` is recorded, a Table holds `D` Documents whose text is the query and whose Source is `other`, and one Document whose text is that query followed by more words in the same paragraph and whose Source is `keep`. Each Document is one Chunk. An unfiltered `MATCH` on `docs` returns no row with Source `keep`. `WHERE docs MATCH` the query `AND source = 'keep'` returns that Document. Its Score equals the Score the same text receives for that query when it is the only Document in a Table. The same predicate on `docs_chunks` returns that Chunk and no Chunk whose Source is `other`, with that same Score.
- **TC-273** A Table holds `D` Documents with Source `keep`, format `text/html`, and text equal to the query, and one Document with Source `keep`, format `text/plain`, and the query inside a longer paragraph. `MATCH` with `source = 'keep'` returns no row for the plain Document. `MATCH` with `source = 'keep' AND format = 'text/plain'` returns that Document, and its Score equals the Score of that text searched alone. `hash =` its stored hash returns it. `text =` its stored text returns it. `format = 'text/html' AND source = 'keep'` returns html Documents and no plain Document.
- **TC-274** Extend the table from TC-272 with one more Document, Source `also`, built like `keep`. The unfiltered `MATCH` returns neither `keep` nor `also`. `doc_id =` the `keep` id returns that Document. `rowid =` that id returns it, and `EXPLAIN QUERY PLAN` reports `INDEX 1:`. `source IN ('keep', 'also')` returns both Documents. Each Score equals the Score that Document receives for the same query when the Table holds only those two.
- **TC-275** A Table holds the `D` exact-query Documents from TC-272 and one Markdown Document `# Kept`, a blank line, then a paragraph that contains the query. The unfiltered `MATCH` on `docs_chunks` returns neither the heading Chunk nor the body Chunk. `chunk =` the body text returns the body Chunk. `start =` the body's start and `end =` the body's end returns it. `heading_path = '["Kept"]'` returns it. Those predicates name a different start, a different end, or `heading_path = '[]'` and return no body row. `doc_id =` the Markdown Document returns the body Chunk at the Score that text receives when searched alone, and may also return the heading Chunk, because a constraint on the Document keeps every Chunk of that Document. `rowid =` the body Chunk returns only that Chunk, at that Score. A Table built the same way with one DoclingDocument instead of the Markdown Document has a content Chunk the unfiltered `MATCH` omits. `meta_json =` that Chunk's stored object returns it. A different `meta_json` object returns no row.
- **TC-281** On the table from TC-272, `source = 'missing'` with `MATCH` returns no row and does not fail. An empty `MATCH` with `source = 'keep'` returns no row. With no `MATCH`, `source = 'keep'` returns that Document.

## REQ-100

- **TC-277** On the table from TC-272, `MATCH AND score > 0` does not return the `keep` Document.

## REQ-101

- **TC-278** On the table from TC-272, `LIMIT 1` returns one fused row, and that row's Score equals its Score in the same `MATCH` with no `LIMIT`. `LIMIT 1 OFFSET 1` returns the next fused row with that row's unlimited Score. `ORDER BY source` returns the same Documents as the `MATCH` without `ORDER BY`.

## REQ-102

- **TC-279** On the table from TC-272, an ordinary table `pages(path, project)` holds `('keep', 'wiki')`, `('x1', 'a')`, and `('x2', 'b')`. `docs` joined to `pages` on `path = source`, with `pages.project = 'wiki'` and `docs MATCH` the query, returns the `keep` Document. Its Score equals the Score that text receives when searched alone.
- **TC-276** On the table from TC-279, `pages CROSS JOIN docs` on `docs.source = pages.path`, and `pages LEFT JOIN docs` on that same key with the `MATCH` in the `ON` clause, each return the `keep` Document at the Score from TC-279. `EXPLAIN QUERY PLAN` for the inner join reports `INDEX 1:2`. The same plan is reported after `CREATE INDEX` on `pages(project)`.
- **TC-282** On the table from TC-279, joining `docs` to `pages` on `pages.project = 'wiki'` with no RAGite column in the join does not return the `keep` Document. The same `MATCH` with `pages.project = 'wiki'` in the `WHERE` clause and no join key does not return it either.

## REQ-103

- **TC-280** On the table from TC-272, `source LIKE 'keep%'`, `source != 'other'`, and `rowid > 0` do not return the `keep` Document.

## Documentation and setup

- **TC-267** The SQL block under *Quickstart* in `README.md` runs unchanged in the `sqlite3` shell started in the directory of the built extension, including `load_extension('./libragite')`. The default Model row it prints matches the technical id named in the README, the Document row has Source `manual.md`, and `MATCH` returns at least one Chunk with a Score.
- **TC-270** On a fresh checkout with Git LFS, the commands in CONTRIBUTING.md (`cargo fetch --locked`, `cargo build -p ragite-sqlite`, `cargo test --workspace`) pass without the Makefile. CI job `fresh-checkout` runs exactly those commands.
- **TC-271** Every `sql` block in `docs/guide.md` runs in order in one `sqlite3` session started in the directory of the built extension, without an error. Each `-- expect:` line in a block names text that appears in that block's JSON output.

## REQ-104

- **TC-283** After `load_extension`, `vec_version()` returns `v0.1.9`. The scalar functions on that connection are `vec_version`, `vec_debug`, `vec_distance_l2`, `vec_distance_l1`, `vec_distance_hamming`, `vec_distance_cosine`, `vec_length`, `vec_type`, `vec_to_json`, `vec_add`, `vec_sub`, `vec_slice`, `vec_normalize`, `vec_f32`, `vec_bit`, `vec_int8`, `vec_quantize_int8`, and `vec_quantize_binary`. The modules are `vec0` and `vec_each`. `vec_npy_file`, `vec_npy_each`, `vec_static_blob_from_raw`, `vec_static_blobs`, and `vec_static_blob_entries` are absent.
