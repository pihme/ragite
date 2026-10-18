# Tinkering with RAGite

This guide is for contributors and for anyone running a fork. It explains where the chunkers and the search live in the code and what to change for four common experiments:

1. [Add a new document type](#1-add-a-new-document-type)
2. [Write a new chunker](#2-write-a-new-chunker)
3. [Change an existing chunker](#3-change-an-existing-chunker)
4. [Influence search results and the fusion](#4-influence-search-results-and-the-fusion)

For using RAGite as it is, read the [user guide](guide.md) instead. Every name below is a real file, type, or function on `main`. If one has moved, trust the code and fix this file in the same pull request.

## Before you start: presets are fixed on purpose

A RAGite build has no runtime switch for chunk size, overlap, fusion constant, weights, or candidate depth. The [README](../README.md#limitations) says so, and [SPEC.md](../SPEC.md) marks most of these choices *Do not reopen*. Every change in this guide is therefore a **code change plus a spec change**, never a setting.

- In a fork, change the code and the requirements together.
- Upstream, open an issue first ([CONTRIBUTING.md](../CONTRIBUTING.md#chunker-scoring-and-other-presets)). A proposal that comes with benchmark numbers (see [Measure the effect](#measure-the-effect)) is reviewed first. A new requirement takes the next free `REQ-n`, and a new test case takes the next free `TC-n`. Numbers are never reused.

### Code map

| What | Where |
| --- | --- |
| `Chunk` struct, `chunk_plain`, `chunk_markdown`, `chunk_html`, shared splitter `split_range` | `crates/ragite-core/src/chunk.rs` |
| Markdown cutter `cut` | `crates/ragite-core/src/markdown.rs` |
| HTML cutter `cut` / `cut_pieces`, its own small parser `Parser` | `crates/ragite-core/src/html.rs` (+ `html_entities.rs`) |
| Docling cutter `chunk_docling` | `crates/ragite-core/src/docling.rs` |
| Constants `CHUNK_LIMIT` (512), `OVERLAP` (64), `FUSION_DEPTH` (50) and the public re-exports | `crates/ragite-core/src/lib.rs` |
| Fusion: `reciprocal_rank_fusion`, `order_fused_chunks`, `best_document_scores`, private `RRF_K` (60) | `crates/ragite-core/src/fusion.rs` |
| Model, prefixes, `Role`, `BuiltinModel::embed_blob` | `crates/ragite-core/src/model.rs` |
| Chunk invariants used by the fuzz targets and TC-250 | `crates/ragite-core/src/invariants.rs` |
| Format check `canonical_format`, chunker dispatch `write_chunks`, row writes `insert_chunk` | `crates/ragite-sqlite/src/vtab.rs` |
| Search: `best_index`, `search_rows`, `vector_ids`, `fts_ids`, `fts_phrase` | `crates/ragite-sqlite/src/vtab.rs` |
| Shadow tables, FTS5 tokenizer, vector distance metric | `create_shadows` in `crates/ragite-sqlite/src/vtab.rs` |

`ragite-core` must not link SQLite (REQ-1). Chunking and fusion logic goes there, and `ragite-sqlite` only calls it.

## 1. Add a new document type

A document type is the `format` value on `INSERT`/`UPDATE` (a media type). It decides which chunker cuts the text. Today exactly four are accepted: `text/plain`, `text/markdown`, `text/html`, and `application/vnd.docling.document+json`.

**Spec first.** REQ-7 and REQ-32 list exactly those four and say *Do not reopen*. A fifth type needs a decision and a spec change. Update REQ-7 and REQ-32, add a section `5.6` that states the cut rules for the type (what a unit is, what `start`/`end` count, what the heading path is, what order Chunks come in), and extend REQ-95 if the type gets a fuzz target. Parsing binary source formats such as PDF or Office is out of scope (REQ-8). Convert those outside RAGite, for example with Docling, and insert the result.

Then the code, in this order:

1. **Chunker in `ragite-core`.** Add a module, for example `crates/ragite-core/src/asciidoc.rs`, with `pub(crate) fn cut(text: &str) -> Vec<Chunk>`. If some inputs are invalid for the type, return `Result<Vec<Chunk>, String>` the way `chunk_docling` does. The `Err` string becomes the SQLite error message, so it must name the failed check and must not contain document text (REQ-54). See [section 2](#2-write-a-new-chunker) for the rules a chunker has to follow.
2. **Export it.** In `crates/ragite-core/src/lib.rs`, add `mod asciidoc;` and a public entry point, either a wrapper in `chunk.rs` next to `chunk_html` or a `pub fn chunk_asciidoc` in the module itself, and add it to the `pub use` line.
3. **Accept the media type.** In `canonical_format` (`crates/ragite-sqlite/src/vtab.rs`), add an arm that maps the lowercased value to its canonical spelling. Anything without an arm fails with `format is not accepted: …`, and that includes media-type parameters (REQ-72).
4. **Dispatch to the chunker.** In `write_chunks` (same file), add a `match` arm for the canonical string. The last arm is `_ => chunk_plain(text)`, so by design a type without its own arm is cut as plain text. If your type needs its own cut, add the arm and check the cut with an integration test (step 6).
5. **Leave the rest alone.** `insert_chunk` writes the chunk row, the vector, and the FTS row for any `Chunk`. `update_document` re-chunks when `text` or `format` changes. Neither needs a change. `meta_json` is reserved for Docling (REQ-16, REQ-19). A new type that wants its own metadata needs a spec change for that as well.
6. **Tests.**
   - Unit tests in the new module (`#[cfg(test)] mod tests`, named `tc_<n>_…` after the TESTSPEC cases they check). The existing modules are the model.
   - An integration test file `crates/ragite-sqlite/tests/asciidoc.rs` that drives the `sqlite3` shell through the shared harness (`mod common; use common::{cell, ok};`), as `tests/markdown.rs` and `tests/html.rs` do. Check the stored `format`, the chunk texts, `start`/`end`, `heading_path`, and that `MATCH` finds the content.
   - Update the format tests that list the accepted types: `tc_11_tc_83_tc_84_tc_203_tc_204_formats` and `tc_269_format_with_a_parameter_fails_for_every_type` in `tests/plain_writes.rs`, and `tc_77_stored_text_is_unchanged_for_each_format` in `tests/plain.rs`. TC-84 names `application/json` and `text/csv` as rejected, so if your type is one of those, that case changes too.
   - In `TESTSPEC.md`, extend TC-11 and TC-84, and add cases under the new and changed REQ headings.
   - Optional but recommended: a `check_asciidoc` in `invariants.rs` (exported under `#[cfg(any(test, feature = "fuzz-invariants"))]` in `lib.rs`), a fuzz target `fuzz/fuzz_targets/asciidoc.rs` with its `[[bin]]` entry in `fuzz/Cargo.toml`, seeds in `fuzz/corpus/asciidoc/`, and a loop for that corpus in `tc_250_seed_corpus_obeys_chunk_invariants`.
7. **Docs.** Update the input list in `README.md` (*Out of scope … Input is …*). If you add a dependency, such as a parser crate, add it to `THIRD_PARTY_NOTICES.md` in the same commit, with its license from the real metadata. The license test `tc_10_tc_64_tc_65_notices_and_dependency_licenses` in `tests/plain.rs` reads the dependencies through `cargo metadata` and checks them against that file.

## 2. Write a new chunker

There is no `Chunker` trait. A chunker is a plain function from the stored text to a list of `Chunk`s, and `write_chunks` picks one with a `match` on the canonical format:

```rust
// crates/ragite-core/src/chunk.rs
pub struct Chunk {
    pub text: String,
    pub start: usize,               // inclusive, Unicode scalar index
    pub end: usize,                 // exclusive, Unicode scalar index
    pub heading_path: Vec<String>,  // outermost heading first; [] when there is none
    pub meta_json: Option<String>,  // None except for Docling
}
```

A sketch that follows the patterns in `markdown.rs` and `docling.rs`:

```rust
use crate::chunk::{Chunk, split_range};

pub(crate) fn cut(text: &str) -> Vec<Chunk> {
    // Work on scalars, not bytes: start/end count `char`s (REQ-31).
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut path: Vec<String> = Vec::new();
    for (unit_start, unit_end) in units(&chars, &mut path) {
        // split_range applies CHUNK_LIMIT and OVERLAP and keeps the overlap inside the unit.
        for (start, end) in split_range(unit_start, unit_end) {
            out.push(Chunk {
                text: chars[start..end].iter().collect(),
                start,
                end,
                heading_path: path.clone(),
                meta_json: None,
            });
        }
    }
    out
}
```

`units` stands for your format's own logic, which finds paragraphs, sections, or code blocks and keeps the heading path up to date.

Rules every chunker on `main` follows. A new one should follow them too, or the spec has to say otherwise:

- **Offsets are Unicode scalars**, the same units as a Rust `char` (REQ-31). For a text format, `text` is the exact slice `chars[start..end]`, and `invariants.rs` (`check_exact_slice`) relies on that. HTML is the documented exception: `start`/`end` cover the source span and `text` is the visible text. Docling counts inside each item's own string.
- **Length and overlap come from `split_range`.** It cuts a unit longer than `CHUNK_LIMIT` into pieces of `CHUNK_LIMIT` scalars, and each next piece starts `OVERLAP` scalars before the previous one ends (REQ-70, REQ-95). Call it on one unit at a time, so the overlap never crosses a unit boundary. A unit the spec keeps whole, such as a heading, is pushed without `split_range`, as `push_whole` in `markdown.rs` does.
- **No empty Chunks.** A unit with no characters produces nothing (REQ-36, REQ-48). A whitespace-only unit is still text where the spec says so.
- **Emission order is the Chunk id order** (REQ-52). `insert_chunk` writes Chunks in the order the `Vec` holds them. If you collect out of order, sort before returning, as `html::cut_pieces` sorts by `start`.
- **The heading path holds titles only** (REQ-56): no `#` marks and no tags, outermost first. `ragite-sqlite` serialises it with `heading_path_json`.
- **Never panic.** The chunker must finish on any Unicode input (REQ-95). The fuzz targets check this. The `guard` around the virtual-table callbacks catches a panic, but the statement then fails with a generic error instead of a message that names the check.
- **What gets embedded and indexed is `Chunk.text` only** (REQ-61). The heading path and `meta_json` are not part of the vector or the FTS row. If you want headings to affect search, that is a spec change in `insert_chunk`, not in the chunker.

Tests for a new chunker: unit tests with exact `text`/`start`/`end`/`heading_path` expectations (see `tc_160_plain_blank_line` in `lib.rs` and the tests in `markdown.rs`), a case longer than 512 scalars that checks the split and the 64-scalar overlap (the model is `tc_200_overlap_copies_the_last_n_scalars`), an empty input, and a `check_…` invariant plus fuzz target as described in step 6 above.

## 3. Change an existing chunker

Find the rule you want to change in SPEC section 5 (5.2 plain, 5.3 Markdown, 5.4 HTML, 5.5 Docling), then the function that implements it:

| Format | Rule | Function(s) |
| --- | --- | --- |
| plain | paragraph breaks, blank lines, `\r\n` | `chunk_plain`, `paragraph_ranges`, `is_blank_run`, `consume_blank_run`, `newline_len` in `chunk.rs` |
| Markdown | headings (REQ-71) | `heading_at`, `heading_title` |
| Markdown | fences (REQ-58, REQ-83) | `fence_open_at`, `fence_end`, `fence_close_end` |
| Markdown | indented code (REQ-63) | `is_indented_code_line`, `push_indented`, `indented_follows` |
| Markdown | paragraphs | `push_paragraph`, `push_split`, `push_whole` |
| HTML | which elements are paragraphs, headings, dropped, inline, or void | `is_paragraph`, `heading_level`, `is_dropped`, `is_inline`, `is_void` in `html.rs` |
| HTML | whitespace collapse (REQ-68), character references (REQ-86) | `collapse`, `decode`, `character_reference`, `from_numeric` |
| HTML | comments (REQ-79) | `emit_comment`, `Parser` |
| Docling | walk order and refs (REQ-52, REQ-94) | `walk`, `walk_ref`, `resolve`, `decode_pointer`, `child_refs` |
| Docling | item text, tables, key-value items (REQ-51, REQ-97) | `item_text`, `table_text`, `render_bounded`, `render_sparse`, `kv_text` |
| Docling | heading path and `meta_json` (REQ-49, REQ-20) | `heading_path_for`, `heading_level`, `meta_for`, `meta_json` |
| all | length limit and overlap | `CHUNK_LIMIT`, `OVERLAP` in `lib.rs`, used by `split_range` |

Things to know before you change anything:

- **Most of these rules are marked *Do not reopen*.** Upstream, a change starts with an issue and a spec change. In a fork, edit the REQ text together with the code.
- **`CHUNK_LIMIT` and `OVERLAP` are not only in code.** SPEC section 8.1, the literal numbers in REQ-95, the README *Status* paragraph, and `fuzz/README.md` all state 512 and 64. TESTSPEC uses `L` and `N`, and the Rust tests read the constants, so those follow. REQ-33 says the values are chosen by the benchmark. Bring numbers (see [Measure the effect](#measure-the-effect)).
- **Existing tables are not re-chunked.** Chunks are cut only on `INSERT` and on an `UPDATE` that changes `text` or `format` (`update_document`). A build with a new chunker leaves old Chunks as they are, so one table can end up mixing old and new cuts. Users re-index by copying the rows into a new table, as REQ-25 describes for a Model change. A chunker change that makes users re-index is a breaking change: mark the commit with `!` and a `BREAKING CHANGE:` footer.
- **Which tests break.** The unit tests in the module you change, the matching integration file in `crates/ragite-sqlite/tests/` (`plain.rs`, `markdown.rs`, `html.rs`, `docling.rs`), the invariants in `invariants.rs` if the rule they encode changes, and TC-250 if a seed in `fuzz/corpus/` no longer satisfies them. Update the TESTSPEC case text to the new rule and keep its number. A removed case keeps its number reserved.

## 4. Influence search results and the fusion

### How a `MATCH` runs today

`best_index` turns a usable `MATCH` constraint into the search plan (`IDX_MATCH`). `filter` then calls `search_rows`, which does the following:

1. Returns no rows for an empty query, without running the Model (REQ-43).
2. Embeds the query with `Role::Query`, so the query prefix is added (REQ-45).
3. `vector_ids`: a sqlite-vec KNN on `<name>_vec` with `k = FUSION_DEPTH`, ordered by cosine distance.
4. `fts_ids`: FTS5 on `<name>_fts` with the query wrapped as one phrase by `fts_phrase` (REQ-75), ordered by `bm25()`, `LIMIT FUSION_DEPTH`.
5. `reciprocal_rank_fusion(&vector, &full_text)`: each list adds `1/(RRF_K + rank)` with rank starting at 1, and a Chunk on both lists sums its two terms.
6. On `<name>_chunks`, `order_fused_chunks` sorts by Score, then `doc_id`, then Chunk id. On `<name>`, `best_document_scores` keeps each Document's **best** Chunk Score, not a sum.

### What is fixed and where it lives

None of these can be set from SQL or at `CREATE VIRTUAL TABLE`. Each one is a code change, and most also need a spec change.

| Knob | Value | Where | Spec |
| --- | --- | --- | --- |
| RRF constant k | 60 | `const RRF_K` in `fusion.rs` (private) | REQ-15, REQ-66, REQ-80 |
| Weights of the two lists | equal, implicitly | `add_ranks` is called with the same formula for both lists, and there is no weight parameter | REQ-15, REQ-66 |
| Candidate depth per list | 50 | `FUSION_DEPTH` in `lib.rs`, used by `search_rows` for both lists and by `best_index` as `estimatedRows` | REQ-15, REQ-38, section 8.1 |
| Document score | best Chunk (max) | `best_document_scores` | section 4.5 intro, REQ-80 |
| Tie order | `doc_id`, then Chunk id | `order_fused_chunks`, `best_document_scores` | REQ-41 |
| Vector metric | cosine | `distance_metric=cosine` in `create_shadows` | REQ-66 |
| FTS query | one plain phrase, no operators | `fts_phrase` | REQ-75 |
| FTS tokenizer | `unicode61`, no stemming | `tokenize = 'unicode61'` in `create_shadows` | REQ-28 |
| BM25 column weights | FTS5 defaults (one column) | `bm25({name})` in `fts_ids` | none |
| Query/passage prefixes | empty for the built-in Model | `QUERY_PREFIX`/`PASSAGE_PREFIX` in `model.rs`; a provider build sets `RAGITE_QUERY_PREFIX`/`RAGITE_PASSAGE_PREFIX` | REQ-45, REQ-23 |
| Score cutoff | none | there is no threshold anywhere | section 4.5 intro |
| Reranking | none | not implemented | section 3 |

Points that tend to surprise people:

- **There is no score cutoff, and the vector side always returns neighbours.** KNN returns the `FUSION_DEPTH` nearest Chunks whatever they contain, so a `MATCH` on a table with at least 50 Chunks returns 50 rows even for a query that matches no word. `tc_27_same_scores_and_fixed_depth` in `tests/search.rs` checks exactly that.
- **`LIMIT` and other `WHERE` filters apply after the fusion.** `best_index` consumes only the `MATCH` constraint and does not use `LIMIT`. Both lists are always fetched at full depth and fused, and SQLite then applies `LIMIT`, `source = '…'`, `JOIN`s, and `ORDER BY` to those rows. So `WHERE docs MATCH 'q' AND source = 'a.md'` can return fewer rows than exist for `a.md`, or none, when that source's Chunks are not among the top 50 of either list. To filter before ranking, you would have to push the filter into `vector_ids`/`fts_ids`. That is a code and spec change.
- **Changing `create_shadows` affects only new tables.** The tokenizer and the distance metric are fixed into each table when it is created. Existing databases keep what they were created with.
- **Changing `RRF_K` changes every expected Score in the tests.** `fusion.rs` unit tests and `tests/search.rs` assert literal values such as `2.0 / 61.0`.

### Ways to influence results without a code change

Everything SQL can do on top of the returned rows:

- **Take fewer rows** with `LIMIT`, or search Chunks (`<name>_chunks`) instead of Documents when several hits per Document matter.
- **Re-rank in SQL.** `score` is an ordinary column on the result. Join with your own tables and order by a mix of `score` and your own signals, such as recency or a per-source boost:

  ```sql
  SELECT c.chunk, c.source, c.score * coalesce(b.boost, 1.0) AS ranked
  FROM docs_chunks c LEFT JOIN source_boost b ON b.source = c.source
  WHERE docs_chunks MATCH 'how do I install it'
  ORDER BY ranked DESC
  LIMIT 5;
  ```

  This re-orders only the at most 50 fused rows. It cannot bring back a Chunk that fell outside the depth.
- **Shape the input.** Splitting documents differently before `INSERT`, or choosing `text/markdown` over `text/plain` so headings become their own Chunks, changes what each Chunk contains, and so what each ranking sees.
- **Use your own vectors next to RAGite.** `ragite_embed(table, text, role)` and `ragite_embed_model(id, text, role)` return the same vectors RAGite stores (REQ-17, REQ-69), so an application can keep its own vector table and combine it with `MATCH` results in SQL. RAGite's own shadow tables (`<name>_vec`, `<name>_fts`, `<name>_chunk_rows`) are internal: their shape is not a public interface, and writing to them breaks the consistency between them.

### Changing the fusion in code

- **k or weights:** change `RRF_K` and `add_ranks` in `fusion.rs`. For weights, a natural shape is `reciprocal_rank_fusion(vector, full_text)` calling a weighted `add_ranks(scores, ids, weight)`. Keep the function free of SQLite. Update REQ-15, REQ-66, REQ-80, TC-26, TC-223, and TC-224, and the literal Scores in `fusion.rs` and `tests/search.rs`.
- **Depth:** change `FUSION_DEPTH` in `lib.rs`. Update section 8.1 and the README *Status* paragraph. `tc_27_same_scores_and_fixed_depth` inserts 51 documents and expects 50 rows, so it has to follow the new value. TC-32 reads `D` from section 8.1.
- **Document score** (for example a sum instead of the best Chunk): `best_document_scores`, plus the section 4.5 intro, REQ-80, TC-29, TC-224, and TC-256.
- **Exposing a knob in SQL** (a hidden column, a `CREATE VIRTUAL TABLE` argument, or a `MATCH` syntax) contradicts TC-27, which requires that the caller has no argument for weights, k, or depth, and REQ-75, which accepts plain text only. That is a spec decision, not a refactor.

### Measure the effect

Search changes are judged on numbers. The benchmark in [bench/README.md](../bench/README.md) runs the hybrid `MATCH` path (`bench/run_quality.py`, mode `hybrid`) on MTEB SciFact, NFCorpus, and FiQA and reports nDCG@10 and Recall@10. Run it on your branch through the manual **Model benchmark** workflow, for example `gh workflow run bench.yml --ref <your-branch> -f model=potion-retrieval-32M`. Compare against `bench/RESULTS.md` and put the numbers in the pull request.

## Running the tests

A full `make test` builds the extension three times and embeds the 129 MB Model, so it needs a few GB of RAM and disk (see [CONTRIBUTING.md](../CONTRIBUTING.md#build-and-test-locally)). CI runs it on every pull request. While you work, the smaller loops are usually enough:

```bash
# Chunker and fusion unit tests, without embedding the Model (fast, low memory)
cargo test -p ragite-core --no-default-features --lib

# One integration file against the debug extension (builds it once)
cargo build -p ragite-sqlite
cargo test -p ragite-sqlite --test markdown
cargo test -p ragite-sqlite --test search

# Fuzz a chunker for 30 seconds (nightly + cargo-fuzz, see fuzz/README.md)
cargo +nightly fuzz run markdown -- -max_total_time=30
```

Tests cite their TESTSPEC cases in the function name (`tc_200_…`, `tc_26_tc_223_…`). Add new cases to `TESTSPEC.md` under the REQ they check, with the next free number, and cite them the same way.
