//! Plain-text writes after the first insert. Cases are numbered in TESTSPEC.md.

mod common;

use common::{HELLO_SHA, cell, err, ok};

const SPACES_SHA: &str = "0aad7da77d2ed59c396c99a74e49f3a4524dcdbcb5163251b1433d640247aeb4";

#[test]
fn tc_78_tc_79_scalar_offsets_are_not_bytes() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('a😀b');
         INSERT INTO docs(text) VALUES ('é');
         SELECT d.text AS text, c.chunk AS chunk, c.start AS start, c.end AS end
         FROM docs_chunks c JOIN docs d ON d.doc_id = c.doc_id
         ORDER BY d.doc_id;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "a😀b");
    assert_eq!(
        cell(&rows, 0, "end").parse::<i64>().unwrap()
            - cell(&rows, 0, "start").parse::<i64>().unwrap(),
        3
    );
    let text = cell(&rows, 1, "text");
    assert_eq!(text.chars().count(), 2);
    assert_eq!(cell(&rows, 1, "chunk"), text);
    assert_eq!(cell(&rows, 1, "start"), "0");
    assert_eq!(cell(&rows, 1, "end"), "2");
}

#[test]
fn tc_112_tc_113_tc_114_tc_146_tc_147_order_without_match() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('a' || char(10) || char(10) || 'b', 'manual.md');
         INSERT INTO docs(text, source) VALUES ('c', 'other');
         INSERT INTO docs(text, source) VALUES ('d', 'manual.md');
         SELECT doc_id, score FROM docs;
         SELECT doc_id FROM docs WHERE source = 'manual.md';
         SELECT doc_id, chunk, rowid FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert!(rows.as_array().unwrap()[0]["score"].is_null());
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    assert_eq!(cell(&rows, 2, "doc_id"), "3");
    assert_eq!(cell(&rows, 3, "doc_id"), "1");
    assert_eq!(cell(&rows, 4, "doc_id"), "3");
    assert_eq!(cell(&rows, 5, "chunk"), "a");
    assert_eq!(cell(&rows, 6, "chunk"), "b");
    assert_eq!(cell(&rows, 5, "doc_id"), "1");
    assert_eq!(cell(&rows, 6, "doc_id"), "1");
    let first = cell(&rows, 5, "rowid").parse::<i64>().unwrap();
    let second = cell(&rows, 6, "rowid").parse::<i64>().unwrap();
    assert!(first < second, "{first} {second}");
}

#[test]
fn tc_11_tc_83_tc_84_tc_203_tc_204_formats() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('# heading');
         INSERT INTO docs(text) VALUES ('<p>hi</p>');
         INSERT INTO docs(text) VALUES ('{\"schema_name\":\"DoclingDocument\"}');
         INSERT INTO docs(text, format) VALUES ('m', 'TEXT/MARKDOWN');
         INSERT INTO docs(text, format) VALUES ('h', 'Text/Html');
         INSERT INTO docs(text, format) VALUES ('{\"schema_name\":\"DoclingDocument\"}', 'APPLICATION/VND.DOCLING.DOCUMENT+JSON');
         SELECT format FROM docs ORDER BY doc_id;",
    );
    let formats: Vec<String> = (0..6).map(|i| cell(&rows, i, "format")).collect();
    assert_eq!(
        formats,
        vec![
            "text/plain",
            "text/plain",
            "text/plain",
            "text/markdown",
            "text/html",
            "application/vnd.docling.document+json",
        ]
    );
    for bad in ["application/json", "text/csv", "text/html; charset=utf-8", "text/plain; charset=utf-8"] {
        let message = err(&format!(
            "CREATE VIRTUAL TABLE docs USING ragite;
             INSERT INTO docs(text, format) VALUES ('x', '{bad}');"
        ));
        assert!(message.contains(bad), "{message}");
    }
}

#[test]
fn tc_93_tc_94_empty_text_fails_and_whitespace_is_stored() {
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('');",
    );
    assert!(message.to_ascii_lowercase().contains("text"), "{message}");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT count(*) AS n FROM docs;",
    );
    assert_eq!(cell(&rows, 0, "n"), "0");
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET text = '' WHERE doc_id = 1;",
    );
    assert!(message.to_ascii_lowercase().contains("text"), "{message}");
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET text = '' WHERE doc_id = 1;
         SELECT text FROM docs;
         SELECT count(*) AS n FROM docs_chunk_rows;",
    );
    assert!(!out.status, "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 1, "n"), "1");
}

#[test]
fn tc_94_whitespace_is_stored() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('   ');
         SELECT text, hash FROM docs;",
    );
    assert_eq!(cell(&rows, 0, "text"), "   ");
    assert_eq!(cell(&rows, 0, "hash"), SPACES_SHA);
}

#[test]
fn tc_99_tc_100_tc_101_tc_205_tc_206_nulls_and_rejected_columns() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', '');
         INSERT INTO docs(text, source, format, hash) VALUES ('hello', NULL, NULL, NULL);
         SELECT source, format, hash FROM docs ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "source"), "");
    assert_eq!(cell(&rows, 1, "source"), "");
    assert_eq!(cell(&rows, 1, "format"), "text/plain");
    assert_eq!(cell(&rows, 1, "hash"), HELLO_SHA);
    for script in [
        "INSERT INTO docs(source) VALUES ('orphan');",
        "INSERT INTO docs DEFAULT VALUES;",
        "INSERT INTO docs(text) VALUES (NULL);",
        "INSERT INTO docs(doc_id, text) VALUES (9, 'hello');",
        "INSERT INTO docs(text, score) VALUES ('hello', 1);",
    ] {
        let message = err(&format!(
            "CREATE VIRTUAL TABLE docs USING ragite;\n{script}"
        ));
        assert!(!message.is_empty(), "{script}");
    }
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET text = NULL WHERE doc_id = 1;",
    );
    assert!(message.to_ascii_lowercase().contains("text"), "{message}");
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET doc_id = 9 WHERE doc_id = 1;",
    );
    assert!(message.contains("doc_id"), "{message}");
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET score = 1 WHERE doc_id = 1;",
    );
    assert!(message.contains("score"), "{message}");
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET doc_id = 9 WHERE doc_id = 1;
         UPDATE docs SET score = 1 WHERE doc_id = 1;
         SELECT doc_id, text FROM docs;
         SELECT count(*) AS n FROM docs_chunk_rows;",
    );
    assert!(!out.status, "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 1, "n"), "1");
}

#[test]
fn tc_102_tc_207_update_columns_and_null_format() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# A' || char(10) || char(10) || 'body', 'text/markdown');
         UPDATE docs SET text = 'kept', source = 's', format = 'text/html', hash = 'h' WHERE doc_id = 1;
         SELECT text, source, format, hash FROM docs;",
    );
    assert_eq!(cell(&rows, 0, "text"), "kept");
    assert_eq!(cell(&rows, 0, "source"), "s");
    assert_eq!(cell(&rows, 0, "format"), "text/html");
    assert_eq!(cell(&rows, 0, "hash"), "h");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# A' || char(10) || char(10) || 'body', 'text/markdown');
         UPDATE docs SET format = NULL WHERE doc_id = 1;
         SELECT d.format AS format, c.chunk AS chunk
         FROM docs_chunks c JOIN docs d ON d.doc_id = c.doc_id
         ORDER BY c.rowid;",
    );
    assert_eq!(cell(&rows, 0, "format"), "text/plain");
    assert_eq!(cell(&rows, 0, "chunk"), "# A");
    assert_eq!(cell(&rows, 1, "chunk"), "body");
}

#[test]
fn tc_23_tc_89_tc_91_tc_92_hash_and_source_do_not_rechunk() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, hash) VALUES ('hello', 'not-a-digest');
         INSERT INTO docs(text, hash) VALUES ('hello', 'not-a-digest');
         SELECT hash, doc_id FROM docs ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "hash"), "not-a-digest");
    assert_eq!(cell(&rows, 1, "hash"), "not-a-digest");
    assert_ne!(cell(&rows, 0, "doc_id"), cell(&rows, 1, "doc_id"));
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         SELECT rowid AS id FROM docs_chunks;
         UPDATE docs SET text = 'hello!' WHERE doc_id = 1;
         SELECT hash FROM docs;
         SELECT rowid AS id FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 1, "hash"), HELLO_SHA);
    assert_ne!(cell(&rows, 0, "id"), cell(&rows, 2, "id"));
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', 'old');
         SELECT rowid AS id, embedding FROM docs_chunk_rows c JOIN docs_vec v ON v.rowid = c.chunk_id;
         UPDATE docs SET source = 'new' WHERE doc_id = 1;
         UPDATE docs SET hash = 'renamed' WHERE doc_id = 1;
         SELECT source, hash FROM docs;
         SELECT rowid AS id FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 1, "source"), "new");
    assert_eq!(cell(&rows, 1, "hash"), "renamed");
    assert_eq!(cell(&rows, 0, "id"), cell(&rows, 2, "id"));
}

#[test]
fn tc_19_tc_20_tc_22_tc_24_update_and_delete() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         INSERT INTO docs(text) VALUES ('other');
         BEGIN;
         UPDATE docs SET text = 'hello!' WHERE doc_id = 1;
         ROLLBACK;
         SELECT text FROM docs ORDER BY doc_id;
         SELECT text FROM docs_chunk_rows ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 1, "text"), "other");
    assert_eq!(cell(&rows, 2, "text"), "hello");
    assert_eq!(cell(&rows, 3, "text"), "other");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         INSERT INTO docs(text) VALUES ('other');
         UPDATE docs SET text = 'hello!' WHERE doc_id = 1;
         SELECT d.doc_id, c.text FROM docs_documents d LEFT JOIN docs_chunk_rows c ON c.doc_id = d.doc_id ORDER BY d.doc_id;",
    );
    assert_eq!(cell(&rows, 0, "text"), "hello!");
    assert_eq!(cell(&rows, 1, "text"), "other");
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET format = 'text/csv' WHERE doc_id = 1;",
    );
    assert!(message.contains("text/csv"), "{message}");
}

#[test]
fn tc_22_invalid_format_leaves_the_document() {
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs SET format = 'text/csv' WHERE doc_id = 1;
         SELECT text, format FROM docs;
         SELECT count(*) AS n FROM docs_chunk_rows;
         SELECT count(*) AS vecs FROM docs_vec;
         SELECT count(*) AS fts FROM docs_fts;",
    );
    assert!(!out.status, "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 0, "format"), "text/plain");
    assert_eq!(cell(&rows, 1, "n"), "1");
    assert_eq!(cell(&rows, 2, "vecs"), "1");
    assert_eq!(cell(&rows, 3, "fts"), "1");
}

#[test]
fn tc_24_tc_70_tc_71_tc_72_tc_75_source_is_not_identity() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         INSERT INTO docs(text, source) VALUES ('b', 'other');
         SELECT doc_id FROM docs WHERE source = 'same' ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         INSERT INTO docs(text, source) VALUES ('b', 'other');
         UPDATE docs SET text = 'changed' WHERE doc_id = 1;
         DELETE FROM docs WHERE doc_id = 2;
         SELECT doc_id, text FROM docs ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 0, "text"), "changed");
    assert_eq!(cell(&rows, 1, "doc_id"), "3");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         INSERT INTO docs(text, source) VALUES ('b', 'same');
         INSERT INTO docs(text, source) VALUES ('c', 'other');
         DELETE FROM docs WHERE source = 'same';
         SELECT text FROM docs;",
    );
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(cell(&rows, 0, "text"), "c");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('a', 'same');
         DELETE FROM docs WHERE doc_id = 1;
         SELECT count(*) AS docs FROM docs_documents;
         SELECT count(*) AS chunks FROM docs_chunk_rows;
         SELECT count(*) AS vecs FROM docs_vec;
         SELECT count(*) AS fts FROM docs_fts;",
    );
    assert_eq!(cell(&rows, 0, "docs"), "0");
    assert_eq!(cell(&rows, 1, "chunks"), "0");
    assert_eq!(cell(&rows, 2, "vecs"), "0");
    assert_eq!(cell(&rows, 3, "fts"), "0");
}

#[test]
fn tc_85_create_takes_no_chunk_size() {
    let message = err("CREATE VIRTUAL TABLE docs USING ragite(chunk_size=100);");
    assert!(message.contains("chunk_size"), "{message}");
}

#[test]
fn tc_85_failed_create_leaves_no_table() {
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite(chunk_size=100);
         SELECT count(*) AS n FROM sqlite_master WHERE name GLOB 'docs*';",
    );
    assert!(!out.status, "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "n"), "0");
}

#[test]
fn tc_173_tc_188_tc_189_ids_are_not_reused_after_delete() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('a' || char(10) || char(10) || 'b' || char(10) || char(10) || 'c');
         SELECT doc_id FROM docs;
         SELECT rowid AS id FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 1, "id"), "1");
    assert_eq!(cell(&rows, 2, "id"), "2");
    assert_eq!(cell(&rows, 3, "id"), "3");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('a' || char(10) || char(10) || 'b');
         UPDATE docs SET text = 'z' WHERE doc_id = 1;
         INSERT INTO docs(text) VALUES ('next');
         SELECT doc_id FROM docs ORDER BY doc_id;
         SELECT rowid AS id FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    let ids: Vec<String> = (2..rows.as_array().unwrap().len())
        .map(|i| cell(&rows, i, "id"))
        .collect();
    assert!(!ids.contains(&"1".to_string()), "{ids:?}");
    assert!(!ids.contains(&"2".to_string()), "{ids:?}");
}

#[test]
fn tc_231_tc_232_rollback_returns_ids_to_sqlite() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         BEGIN;
         INSERT INTO docs(text) VALUES ('hello');
         ROLLBACK;
         INSERT INTO docs(text) VALUES ('next');
         SELECT doc_id FROM docs;
         SELECT rowid AS id FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 1, "id"), "1");
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('hello', 'text/plain'), ('x', 'text/csv');
         INSERT INTO docs(text) VALUES ('after');
         SELECT doc_id, text FROM docs;",
    );
    assert!(!out.status, "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 0, "text"), "after");
}

#[test]
fn tc_268_rollback_is_visible_to_match() {
    let out = common::sql(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello world', 'a');
         BEGIN;
         INSERT INTO docs(text, source) VALUES ('zebra stripes', 'b');
         ROLLBACK;
         SELECT 'insert' AS step, count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH 'zebra stripes';
         SELECT 'kept' AS step, chunk FROM docs_chunks WHERE docs_chunks MATCH 'hello world';
         SELECT 'insert_fts' AS step, count(*) AS n FROM docs_fts WHERE docs_fts MATCH '\"zebra stripes\"';
         BEGIN;
         UPDATE docs SET text = 'zebra stripes' WHERE doc_id = 1;
         ROLLBACK;
         SELECT 'update' AS step, chunk, source FROM docs_chunks WHERE docs_chunks MATCH 'hello world';
         SELECT 'update_fts' AS step, count(*) AS n FROM docs_fts WHERE docs_fts MATCH '\"zebra stripes\"';
         BEGIN;
         DELETE FROM docs WHERE doc_id = 1;
         ROLLBACK;
         SELECT 'delete' AS step, doc_id, score FROM docs WHERE docs MATCH 'hello world';",
    );
    assert!(out.status, "{}{}", out.stderr, out.stdout);
    let rows = common::parse_json(&out.stdout);
    let rows = rows.as_array().unwrap();
    let step = |name: &str| -> Vec<&serde_json::Value> {
        rows.iter().filter(|r| r["step"] == name).collect()
    };
    // The rolled-back Chunk is an exact match for this query, so a hit would mean
    // it was still stored. "hello world" is farther than the maximum and is not
    // a full-text hit, so the query returns no rows. The committed Chunk remains.
    assert_eq!(step("insert")[0]["n"], 0);
    assert_eq!(step("kept")[0]["chunk"], "hello world");
    assert_eq!(step("insert_fts")[0]["n"], 0);
    let update = step("update");
    assert_eq!(update.len(), 1);
    assert_eq!(update[0]["chunk"], "hello world");
    assert_eq!(step("update_fts")[0]["n"], 0);
    let delete = step("delete");
    assert_eq!(delete.len(), 1);
    assert_eq!(delete[0]["doc_id"], 1);
    assert!(delete[0]["score"].is_number());
}

#[test]
fn tc_269_format_with_a_parameter_fails_for_every_type() {
    for format in [
        "text/markdown; charset=utf-8",
        "TEXT/HTML;charset=UTF-8",
        "application/vnd.docling.document+json; version=1",
    ] {
        let message = common::err(&format!(
            "CREATE VIRTUAL TABLE docs USING ragite;
             INSERT INTO docs(text, format) VALUES ('# m', '{format}');"
        ));
        assert!(
            message.contains(&format.to_lowercase()) || message.contains(format),
            "{format}: {message}"
        );
        let out = common::sql(&format!(
            "CREATE VIRTUAL TABLE docs USING ragite;
             INSERT INTO docs(text, format) VALUES ('# m', 'text/markdown');
             UPDATE docs SET format = '{format}' WHERE doc_id = 1;
             SELECT format, (SELECT count(*) FROM docs_chunk_rows) AS chunks FROM docs;"
        ));
        assert!(!out.status, "{format}: UPDATE was accepted");
        let rows = common::parse_json(&out.stdout);
        assert_eq!(common::cell(&rows, 0, "format"), "text/markdown", "{format}");
        assert_eq!(common::cell(&rows, 0, "chunks"), "1", "{format}");
    }
}
