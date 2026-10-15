//! Model binding and embed helpers. Cases are numbered in TESTSPEC.md.

mod common;

use common::{HELLO_SHA, cell, ok, parse_json, sql};

const DIMENSION: usize = 512;
const SENTINEL: &str = "SENTINEL_ragite_text_9f3a";

struct ModelId {
    name: String,
    technical_id: String,
}

fn model_id() -> ModelId {
    let rows = ok("SELECT name, technical_id FROM ragite_list_models();");
    ModelId {
        name: cell(&rows, 0, "name"),
        technical_id: cell(&rows, 0, "technical_id"),
    }
}

fn other_revision(model: &ModelId) -> String {
    let bad = format!("{}@{}", model.name, "0".repeat(40));
    assert_ne!(bad, model.technical_id);
    bad
}

fn decode_f32_le(hex_text: &str) -> Vec<f32> {
    assert_eq!(hex_text.len() % 8, 0, "blob hex is not a whole number of f32s");
    let mut bytes = Vec::with_capacity(hex_text.len() / 2);
    let chars: Vec<char> = hex_text.chars().collect();
    for pair in chars.chunks(2) {
        let text: String = pair.iter().collect();
        bytes.push(u8::from_str_radix(&text, 16).unwrap());
    }
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

fn assert_vector(hex_text: &str) {
    let values = decode_f32_le(hex_text);
    assert_eq!(values.len(), DIMENSION, "no header, one f32 per dimension");
    assert!(values.iter().all(|value| value.is_finite()), "{hex_text}");
}

#[test]
fn tc_125_tc_130_tc_131_tc_216_catalog_before_any_table() {
    let rows = ok(
        "SELECT name, technical_id FROM ragite_list_models();
         SELECT name, technical_id FROM ragite_default_model();
         SELECT ragite_list_models() AS listed, ragite_default_model() AS default_id;
         SELECT length(ragite_embed_model(
           (SELECT technical_id FROM ragite_default_model()), 'hello', 'passage'
         )) AS n;
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         SELECT name, technical_id FROM ragite_list_models();
         SELECT name, technical_id FROM ragite_default_model();",
    );
    let name = cell(&rows, 0, "name");
    let technical_id = cell(&rows, 0, "technical_id");
    assert!(!name.is_empty());
    assert!(technical_id.starts_with(&format!("{name}@")), "{technical_id}");
    assert_eq!(technical_id.len(), name.len() + 1 + 40);
    assert!(technical_id[name.len() + 1..]
        .chars()
        .all(|c| c.is_ascii_hexdigit()));
    assert_eq!(cell(&rows, 1, "name"), name);
    assert_eq!(cell(&rows, 1, "technical_id"), technical_id);
    assert_eq!(cell(&rows, 2, "listed"), technical_id);
    assert_eq!(cell(&rows, 2, "default_id"), technical_id);
    assert_eq!(cell(&rows, 3, "n"), (DIMENSION * 4).to_string());
    assert_eq!(cell(&rows, 4, "name"), name);
    assert_eq!(cell(&rows, 4, "technical_id"), technical_id);
    assert_eq!(cell(&rows, 5, "name"), name);
    assert_eq!(cell(&rows, 5, "technical_id"), technical_id);
}

#[test]
fn tc_16_tc_184_tc_194_tc_208_tc_58_model_is_stored_and_stable() {
    let model = model_id();
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE plain USING ragite;
         CREATE VIRTUAL TABLE nulled USING ragite(model=NULL);
         CREATE VIRTUAL TABLE chosen USING ragite(model='{id}');
         SELECT ragite_table_model('plain') AS plain,
                ragite_table_model('nulled') AS nulled,
                ragite_table_model('chosen') AS chosen,
                ragite_table_model('chosen') AS again;
         SELECT technical_id AS shadow FROM chosen_model;
         INSERT INTO chosen(text, source) VALUES ('hello', 'manual');
         SELECT ragite_table_model('chosen') AS after_insert;
         UPDATE chosen SET text = 'hello world', source = 'next' WHERE doc_id = 1;
         SELECT ragite_table_model('chosen') AS after_update;
         SELECT technical_id AS shadow_after FROM chosen_model;
         SELECT count(*) AS model_column FROM pragma_table_info('chosen') WHERE name = 'model';
         CREATE VIRTUAL TABLE temp.scratch USING ragite;
         SELECT ragite_table_model('temp.scratch') AS temp_id,
                ragite_table_model('scratch') AS unqualified,
                length(ragite_embed('temp.scratch', 'hello', 'query')) AS temp_n;",
        id = model.technical_id
    ));
    assert_eq!(cell(&rows, 0, "plain"), model.technical_id);
    assert_eq!(cell(&rows, 0, "nulled"), model.technical_id);
    assert_eq!(cell(&rows, 0, "chosen"), model.technical_id);
    assert_eq!(cell(&rows, 0, "again"), model.technical_id);
    assert_eq!(cell(&rows, 1, "shadow"), model.technical_id);
    assert_eq!(cell(&rows, 2, "after_insert"), model.technical_id);
    assert_eq!(cell(&rows, 3, "after_update"), model.technical_id);
    assert_eq!(cell(&rows, 4, "shadow_after"), model.technical_id);
    assert_eq!(cell(&rows, 5, "model_column"), "0");
    assert_eq!(cell(&rows, 6, "temp_id"), model.technical_id);
    assert_eq!(cell(&rows, 6, "unqualified"), model.technical_id);
    assert_eq!(cell(&rows, 6, "temp_n"), (DIMENSION * 4).to_string());
}

#[test]
fn tc_56_tc_106_tc_121_tc_126_tc_127_tc_128_tc_185_rejected_model_on_create() {
    let model = model_id();
    let bad = other_revision(&model);
    let out = sql(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite(model='default');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(model='');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(model='/tmp/weights.bin');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(model='https://example.com/weights.bin');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(model='{bare}');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(model='{bad}');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(query_prefix='query: ', passage_prefix='passage: ');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';
         CREATE VIRTUAL TABLE docs USING ragite(revision='6fc8051fab2a1e0ee76689cf08c853792ac285e7');
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';",
        bare = model.name,
        bad = bad,
    ));
    assert!(!out.status, "{}", out.stderr);
    let rows = parse_json(&out.stdout);
    assert_eq!(rows.as_array().unwrap().len(), 8, "{}\n{}", out.stderr, out.stdout);
    for index in 0..8 {
        assert_eq!(cell(&rows, index, "n"), "0", "{}", out.stderr);
    }
    for needle in [
        "model is not accepted: default",
        "model is empty",
        "model is not accepted: /tmp/weights.bin",
        "model is not accepted: https://example.com/weights.bin",
        &format!("model cannot be loaded: {}", model.name),
        &format!("model cannot be loaded: {bad}"),
        "argument is not accepted: query_prefix",
        "argument is not accepted: revision",
    ] {
        assert!(out.stderr.contains(needle), "missing {needle} in {}", out.stderr);
    }
}

#[test]
fn tc_36_tc_38_tc_119_tc_120_tc_122_tc_123_tc_199_tc_225_embed_blob() {
    let model = model_id();
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello' || char(10) || char(10) || 'world');
         SELECT c.text AS chunk,
                hex(v.embedding) AS stored,
                hex(ragite_embed('docs', c.text, 'passage')) AS via_table,
                hex(ragite_embed_model('{id}', c.text, 'passage')) AS via_model
         FROM docs_chunk_rows c
         JOIN docs_vec v ON v.rowid = c.chunk_id
         ORDER BY c.chunk_id;
         SELECT length(ragite_embed('docs', '   ', 'passage')) AS spaces,
                length(ragite_embed_model('{id}', '   ', 'query')) AS spaces_model,
                hex(ragite_embed('docs', 'hello', 'query')) =
                  hex(ragite_embed('docs', 'hello', 'passage')) AS same_prefix,
                hex(ragite_embed('docs', 'hello', 'passage')) =
                  hex(ragite_embed('docs', 'hello', 'passage')) AS same_call,
                hex(ragite_embed_model('{id}', 'hello', 'query')) =
                  hex(ragite_embed_model('{id}', 'hello', 'query')) AS same_model,
                hex(ragite_embed_model('{id}', 'hello', 'passage')) AS sample;
         SELECT group_concat(name, ',') AS cols FROM pragma_table_info('docs_documents');
         SELECT group_concat(name, ',') AS cols FROM pragma_table_info('docs_chunk_rows');",
        id = model.technical_id
    ));
    assert_eq!(cell(&rows, 0, "chunk"), "hello");
    assert_eq!(cell(&rows, 0, "stored"), cell(&rows, 0, "via_table"));
    assert_eq!(cell(&rows, 0, "stored"), cell(&rows, 0, "via_model"));
    assert_eq!(cell(&rows, 1, "chunk"), "world");
    assert_eq!(cell(&rows, 1, "stored"), cell(&rows, 1, "via_table"));
    assert_eq!(cell(&rows, 1, "stored"), cell(&rows, 1, "via_model"));
    assert_vector(&cell(&rows, 0, "stored"));
    assert_vector(&cell(&rows, 1, "stored"));
    assert_eq!(cell(&rows, 2, "spaces"), (DIMENSION * 4).to_string());
    assert_eq!(cell(&rows, 2, "spaces_model"), (DIMENSION * 4).to_string());
    assert_eq!(cell(&rows, 2, "same_prefix"), "1");
    assert_eq!(cell(&rows, 2, "same_call"), "1");
    assert_eq!(cell(&rows, 2, "same_model"), "1");
    assert_vector(&cell(&rows, 2, "sample"));
    let documents = cell(&rows, 3, "cols");
    let chunks = cell(&rows, 4, "cols");
    assert!(!documents.to_ascii_lowercase().contains("prefix"), "{documents}");
    assert!(!chunks.to_ascii_lowercase().contains("prefix"), "{chunks}");
}

#[test]
fn tc_37_tc_39_tc_40_tc_57_tc_118_tc_124_tc_157_tc_158_tc_159_tc_196_rejections() {
    let model = model_id();
    let bad = other_revision(&model);
    let out = sql(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT ragite_embed('docs_chunks', '{sentinel}', 'passage');
         SELECT ragite_embed('missing', 'hello', 'passage');
         SELECT ragite_table_model('docs_chunks');
         SELECT ragite_table_model('missing');
         SELECT ragite_embed('nosuch.docs', 'hello', 'passage');
         SELECT ragite_embed_model('docs', 'hello', 'passage');
         SELECT ragite_embed('{id}', 'hello', 'passage');
         SELECT ragite_embed_model('default', 'hello', 'passage');
         SELECT ragite_embed_model('/tmp/weights.bin', 'hello', 'passage');
         SELECT ragite_embed_model('https://example.com/weights.bin', 'hello', 'passage');
         SELECT ragite_embed_model('{bad}', 'hello', 'passage');
         SELECT ragite_embed_model('{bare}', 'hello', 'passage');
         SELECT ragite_embed('docs', '', 'passage');
         SELECT ragite_embed_model('{id}', '', 'query');
         SELECT ragite_embed('docs', '{sentinel}', 'document');
         SELECT ragite_embed('docs', '{sentinel}', 'Query');
         SELECT ragite_embed_model('{id}', '{sentinel}', '');
         SELECT ragite_embed('docs', 'hello');
         SELECT ragite_embed_model('{id}', 'hello');
         INSERT INTO docs(text, format) VALUES ('{sentinel}', 'nope/type');
         UPDATE docs SET model = 'other';",
        sentinel = SENTINEL,
        id = model.technical_id,
        bad = bad,
        bare = model.name,
    ));
    assert!(!out.status, "{}", out.stdout);
    assert!(out.stdout.trim().is_empty(), "{}", out.stdout);
    assert!(!out.stderr.contains(SENTINEL), "{}", out.stderr);
    for needle in [
        "table is not a document table: docs_chunks",
        "table was not found: missing",
        "schema was not found: nosuch",
        "model cannot be loaded: docs",
        &format!("table was not found: {}", model.technical_id),
        "model is not accepted: default",
        "model is not accepted: /tmp/weights.bin",
        "model is not accepted: https://example.com/weights.bin",
        &format!("model cannot be loaded: {bad}"),
        &format!("model cannot be loaded: {}", model.name),
        "text is empty",
        "role is not accepted: document",
        "role is not accepted: Query",
        "role is not accepted: ''",
        "role is required",
        "format is not accepted: nope/type",
        "no such column: model",
    ] {
        assert!(
            out.stderr.contains(needle),
            "missing {needle}\n{}",
            out.stderr
        );
    }
}

#[test]
fn tc_226_generated_column_and_index() {
    let model = model_id();
    let rows = ok(
        "CREATE TABLE side(
           body TEXT NOT NULL,
           technical_id TEXT AS (ragite_default_model()) STORED,
           vec BLOB AS (ragite_embed_model(ragite_default_model(), body, 'passage')) STORED
         );
         CREATE INDEX side_id ON side(technical_id);
         CREATE INDEX side_expr ON side(
           length(ragite_embed_model(ragite_default_model(), body, 'passage'))
         );
         INSERT INTO side(body) VALUES ('hello');
         SELECT technical_id, length(vec) AS n, hex(vec) AS vec FROM side;",
    );
    assert_eq!(cell(&rows, 0, "technical_id"), model.technical_id);
    assert_eq!(cell(&rows, 0, "n"), (DIMENSION * 4).to_string());
    assert_vector(&cell(&rows, 0, "vec"));
    let direct = sql(
        "CREATE TABLE bodies(body TEXT NOT NULL);
         CREATE INDEX bodies_vec ON bodies(
           ragite_embed_model(ragite_default_model(), body, 'passage')
         );
         INSERT INTO bodies(body) VALUES ('hello');
         SELECT count(*) AS n FROM bodies;",
    );
    if direct.status {
        let rows = parse_json(&direct.stdout);
        assert_eq!(cell(&rows, 0, "n"), "1");
    } else {
        assert!(
            direct.stderr.contains("too big") || direct.stderr.contains("too large"),
            "index on the vector was rejected for another reason: {}",
            direct.stderr
        );
    }
}

#[test]
fn tc_59_tc_60_tc_115_tc_116_tc_129_tc_195_unloaded_model() {
    let model = model_id();
    let bad = other_revision(&model);
    let out = sql(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', 'manual');
         SELECT count(*) AS n FROM docs WHERE docs MATCH '   ';
         SELECT count(*) AS n FROM docs WHERE docs MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '   ';
         UPDATE docs_model SET technical_id = '{bad}';
         SELECT text, source, format, hash FROM docs;
         SELECT chunk, start, end FROM docs_chunks;
         SELECT ragite_table_model('docs') AS technical_id;
         UPDATE docs SET source = 'later' WHERE doc_id = 1;
         SELECT source FROM docs;
         INSERT INTO docs(text) VALUES ('more');
         UPDATE docs SET text = '{sentinel}' WHERE doc_id = 1;
         UPDATE docs SET format = 'text/markdown' WHERE doc_id = 1;
         SELECT doc_id FROM docs WHERE docs MATCH 'hello';
         SELECT doc_id FROM docs WHERE docs MATCH '   ';
         SELECT chunk FROM docs_chunks WHERE docs_chunks MATCH 'hello';
         SELECT ragite_embed('docs', 'hello', 'query');
         SELECT count(*) AS n FROM docs WHERE docs MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '';
         SELECT text, source FROM docs;
         SELECT count(*) AS n FROM docs;",
        bad = bad,
        sentinel = SENTINEL,
    ));
    assert!(!out.status, "expected model-load failures\n{}", out.stderr);
    assert!(!out.stderr.contains(SENTINEL), "{}", out.stderr);
    assert!(!out.stderr.contains("no such column"), "{}", out.stderr);
    assert!(out.stderr.contains("model cannot be loaded"), "{}", out.stderr);
    assert!(out.stderr.contains(&bad), "{}", out.stderr);
    let rows = parse_json(&out.stdout);
    assert_eq!(
        cell(&rows, 0, "n"),
        "1",
        "whitespace MATCH should return the stored row\n{}\n{}",
        out.stderr,
        out.stdout
    );
    assert_eq!(cell(&rows, 1, "n"), "0", "{}", out.stdout);
    assert_eq!(cell(&rows, 2, "n"), "0", "empty companion MATCH\n{}", out.stdout);
    assert_eq!(cell(&rows, 3, "n"), "1", "whitespace companion MATCH\n{}", out.stdout);
    assert_eq!(cell(&rows, 4, "text"), "hello");
    assert_eq!(cell(&rows, 4, "source"), "manual");
    assert_eq!(cell(&rows, 4, "format"), "text/plain");
    assert_eq!(cell(&rows, 4, "hash"), HELLO_SHA);
    assert_eq!(cell(&rows, 5, "chunk"), "hello");
    assert_eq!(cell(&rows, 5, "start"), "0");
    assert_eq!(cell(&rows, 5, "end"), "5");
    assert_eq!(cell(&rows, 6, "technical_id"), bad);
    assert_eq!(cell(&rows, 7, "source"), "later");
    assert_eq!(cell(&rows, 8, "n"), "0", "empty MATCH stays empty\n{}", out.stderr);
    assert_eq!(cell(&rows, 9, "n"), "0", "empty companion MATCH stays empty\n{}", out.stderr);
    assert_eq!(cell(&rows, 10, "text"), "hello");
    assert_eq!(cell(&rows, 10, "source"), "later");
    assert_eq!(cell(&rows, 11, "n"), "1");
}

#[test]
fn tc_61_copy_assigns_new_document_ids_and_drop_removes_shadows() {
    let model = model_id();
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite(model='{id}');
         INSERT INTO docs(text, source, format, hash) VALUES ('one', 's1', 'text/plain', 'h1');
         INSERT INTO docs(text, source, format, hash) VALUES ('two', 's2', 'text/markdown', 'h2');
         DELETE FROM docs WHERE doc_id = 1;
         CREATE VIRTUAL TABLE copy USING ragite(model='{id}');
         INSERT INTO copy(text, source, format, hash)
           SELECT text, source, format, hash FROM docs;
         SELECT doc_id, text, source, format, hash FROM copy;
         SELECT doc_id AS old_id FROM docs;
         DROP TABLE docs;
         SELECT name FROM sqlite_master ORDER BY name;",
        id = model.technical_id
    ));
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 0, "text"), "two");
    assert_eq!(cell(&rows, 0, "source"), "s2");
    assert_eq!(cell(&rows, 0, "format"), "text/markdown");
    assert_eq!(cell(&rows, 0, "hash"), "h2");
    assert_eq!(cell(&rows, 1, "old_id"), "2");
    let names: Vec<String> = rows
        .as_array()
        .unwrap()
        .iter()
        .skip(2)
        .map(|row| row["name"].as_str().unwrap().to_string())
        .collect();
    for gone in [
        "docs",
        "docs_chunks",
        "docs_documents",
        "docs_chunk_rows",
        "docs_vec",
        "docs_fts",
        "docs_model",
    ] {
        assert!(!names.iter().any(|name| name == gone), "{names:?}");
    }
    for kept in [
        "copy",
        "copy_chunks",
        "copy_documents",
        "copy_chunk_rows",
        "copy_vec",
        "copy_fts",
        "copy_model",
    ] {
        assert!(names.iter().any(|name| name == kept), "{names:?}");
    }
}

#[test]
fn tc_244_table_name_matches_without_ascii_case() {
    let rows = ok(
        "CREATE VIRTUAL TABLE Mixed USING ragite;
         INSERT INTO Mixed(text) VALUES ('hello');
         SELECT ragite_table_model('mixed') AS stored,
                ragite_table_model('MIXED') AS upper_name,
                (SELECT technical_id FROM ragite_default_model()) AS builtin;
         SELECT length(ragite_embed('mixed', 'hello', 'query')) AS n;
         SELECT length(ragite_embed('MAIN.Mixed', 'hello', 'passage')) AS n;",
    );
    assert_eq!(cell(&rows, 0, "stored"), cell(&rows, 0, "builtin"));
    assert_eq!(cell(&rows, 0, "upper_name"), cell(&rows, 0, "builtin"));
    assert_eq!(cell(&rows, 1, "n"), (DIMENSION * 4).to_string());
    assert_eq!(cell(&rows, 2, "n"), (DIMENSION * 4).to_string());
}
