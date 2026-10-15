//! DoclingDocument writes. Cases are numbered in TESTSPEC.md.

mod common;

use common::{cell, err, ok};
use serde_json::{Value, json};

fn insert(text: &str, source: &str) -> String {
    let text = text.replace('\'', "''");
    format!(
        "INSERT INTO docs(text, source, format) VALUES ('{text}', '{source}', 'application/vnd.docling.document+json');\n"
    )
}

fn rows_for<'a>(rows: &'a Value, source: &str) -> Vec<&'a Value> {
    rows.as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"] == source)
        .collect()
}

#[test]
fn tc_151_to_154_bad_docling_names_the_check() {
    let cases = [
        ("not json", "the text is not JSON", "not json"),
        ("[]", "the JSON value is not an object", "[]"),
        (
            "\"DoclingDocument\"",
            "the JSON value is not an object",
            "DoclingDocument",
        ),
        ("{}", "schema_name is missing", "{}"),
        (
            "{\"schema_name\":\"Other\"}",
            "schema_name is not DoclingDocument: Other",
            "{\"schema_name\":\"Other\"}",
        ),
    ];
    for (text, must, absent) in cases {
        let message = err(&format!(
            "CREATE VIRTUAL TABLE docs USING ragite;\n{}",
            insert(text, "bad")
        ));
        assert!(message.contains(must), "{text} -> {message}");
        assert!(!message.contains(absent), "{text} -> {message}");
    }
}

#[test]
fn tc_151_bad_docling_leaves_no_row() {
    let out = common::sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         INSERT INTO docs(text, format) VALUES ('not json', 'application/vnd.docling.document+json');
         INSERT INTO docs(text, format) VALUES ('[]', 'application/vnd.docling.document+json');
         INSERT INTO docs(text, format) VALUES ('\"DoclingDocument\"', 'application/vnd.docling.document+json');
         INSERT INTO docs(text, format) VALUES ('{}', 'application/vnd.docling.document+json');
         INSERT INTO docs(text, format) VALUES ('{\"schema_name\":\"Other\"}', 'application/vnd.docling.document+json');
         UPDATE docs SET text = 'not json', format = 'application/vnd.docling.document+json' WHERE doc_id = 1;
         SELECT text, format FROM docs;
         SELECT count(*) AS n FROM docs;
         SELECT count(*) AS chunks FROM docs_chunk_rows;
         SELECT count(*) AS vecs FROM docs_vec;
         SELECT count(*) AS fts FROM docs_fts;",
    );
    assert!(!out.status, "{}", out.stderr);
    assert!(
        out.stderr.contains("the text is not JSON"),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("the JSON value is not an object"),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("schema_name is missing"),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr
            .contains("schema_name is not DoclingDocument: Other"),
        "{}",
        out.stderr
    );
    assert!(!out.stderr.contains("not json"), "{}", out.stderr);
    let rows = common::parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 0, "format"), "text/plain");
    assert_eq!(cell(&rows, 1, "n"), "1");
    assert_eq!(cell(&rows, 2, "chunks"), "1");
    assert_eq!(cell(&rows, 3, "vecs"), "1");
    assert_eq!(cell(&rows, 4, "fts"), "1");
}

#[test]
fn tc_77_tc_96_tc_155_tc_156_empty_docling_is_stored() {
    let bare = r#"{"schema_name":"DoclingDocument"}"#;
    let extra = r#"{"schema_name":"DoclingDocument","version":"9.9.9","extra":true}"#;
    let empty_items = json!({
        "schema_name": "DoclingDocument",
        "texts": [
            {"self_ref": "#/texts/0", "label": "text", "text": ""},
            {"self_ref": "#/texts/1", "label": "text", "text": ""}
        ],
        "body": {"children": [{"$ref": "#/texts/0"}]},
        "furniture": {"children": [{"$ref": "#/texts/1"}]}
    })
    .to_string();
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;\n{}{}{}
         SELECT text, source FROM docs ORDER BY doc_id;
         SELECT count(*) AS chunks FROM docs_chunk_rows;
         SELECT count(*) AS vecs FROM docs_vec;
         SELECT count(*) AS fts FROM docs_fts;",
        insert(bare, "bare"),
        insert(extra, "extra"),
        insert(&empty_items, "empty-items"),
    ));
    assert_eq!(cell(&rows, 0, "text"), bare);
    assert_eq!(cell(&rows, 0, "source"), "bare");
    assert_eq!(cell(&rows, 1, "text"), extra);
    assert_eq!(cell(&rows, 1, "source"), "extra");
    assert_eq!(cell(&rows, 2, "text"), empty_items);
    assert_eq!(cell(&rows, 2, "source"), "empty-items");
    assert_eq!(cell(&rows, 3, "chunks"), "0");
    assert_eq!(cell(&rows, 4, "vecs"), "0");
    assert_eq!(cell(&rows, 5, "fts"), "0");
}

#[test]
fn tc_47_tc_132_tc_150_stored_chunks_follow_the_walk() {
    let blanks = json!({
        "schema_name": "DoclingDocument",
        "texts": [
            {"self_ref": "#/texts/0", "label": "text", "text": ""},
            {"self_ref": "#/texts/1", "label": "text", "text": "Hello", "orig": "ORIG"},
            {"self_ref": "#/texts/2", "label": "text", "text": " "}
        ],
        "body": {"children": [
            {"$ref": "#/texts/0"},
            {"$ref": "#/texts/1"},
            {"$ref": "#/texts/2"}
        ]}
    });
    let meta = json!({
        "schema_name": "DoclingDocument",
        "texts": [
            {
                "self_ref": "#/texts/0",
                "label": "title",
                "text": "  A # ",
                "prov": [{"page_no": 1}],
                "children": [{"$ref": "#/texts/1"}]
            },
            {
                "self_ref": "#/texts/1",
                "label": "section_header",
                "text": "Details",
                "children": [{"$ref": "#/texts/2"}]
            },
            {
                "self_ref": "#/texts/2",
                "label": "text",
                "text": "a😀b",
                "prov": [{"page_no": 3}, {"page_no": 9}]
            },
            {
                "self_ref": "#/texts/3",
                "label": "page_header",
                "text": "Head",
                "prov": [{"page_no": 4}]
            }
        ],
        "body": {"children": [{"$ref": "#/texts/0"}]},
        "furniture": {"children": [{"$ref": "#/texts/3"}]}
    });
    let table = json!({
        "schema_name": "DoclingDocument",
        "texts": [
            {"self_ref": "#/texts/0", "label": "caption", "text": "Caption"},
            {"self_ref": "#/texts/1", "label": "caption", "text": "NotInTree"}
        ],
        "tables": [{
            "self_ref": "#/tables/0",
            "prov": [{"page_no": 2}],
            "captions": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}],
            "children": [{"$ref": "#/texts/0"}],
            "data": {
                "num_rows": 2,
                "num_cols": 2,
                "table_cells": [
                    {"text": "D", "start_row_offset_idx": 1, "start_col_offset_idx": 1},
                    {"text": "A", "start_row_offset_idx": 0, "start_col_offset_idx": 0},
                    {"text": "", "start_row_offset_idx": 0, "start_col_offset_idx": 1},
                    {"text": "C", "start_row_offset_idx": 1, "start_col_offset_idx": 0}
                ]
            }
        }],
        "body": {"children": [{"$ref": "#/tables/0"}]}
    });
    let pairs = json!({
        "schema_name": "DoclingDocument",
        "key_value_items": [{
            "self_ref": "#/key_value_items/0",
            "graph": {
                "cells": [
                    {"cell_id": 0, "label": "key", "text": "role"},
                    {"cell_id": 1, "label": "value", "text": "admin"},
                    {"cell_id": 2, "label": "key", "text": "name"},
                    {"cell_id": 3, "label": "value", "text": "ada"}
                ],
                "links": [
                    {"label": "to_value", "source_cell_id": 2, "target_cell_id": 3},
                    {"label": "to_value", "source_cell_id": 0, "target_cell_id": 1}
                ]
            }
        }],
        "body": {"children": [{"$ref": "#/key_value_items/0"}]}
    });
    let picture = json!({
        "schema_name": "DoclingDocument",
        "texts": [{"self_ref": "#/texts/0", "label": "caption", "text": "Caption"}],
        "pictures": [{
            "self_ref": "#/pictures/0",
            "image": {"uri": "file:///sentinel.png"},
            "children": [{"$ref": "#/texts/0"}]
        }],
        "body": {"children": [{"$ref": "#/pictures/0"}]}
    });
    let order = json!({
        "schema_name": "DoclingDocument",
        "groups": [{
            "self_ref": "#/groups/0",
            "name": "NotAHeading",
            "children": [
                {"$ref": "#/texts/2"},
                {"$ref": "#/texts/3"},
                {"$ref": "#/texts/2"}
            ]
        }],
        "texts": [
            {"self_ref": "#/texts/0", "label": "text", "text": "parent", "children": [{"$ref": "#/texts/1"}]},
            {"self_ref": "#/texts/1", "label": "text", "text": "child"},
            {"self_ref": "#/texts/2", "label": "text", "text": "a"},
            {"self_ref": "#/texts/3", "label": "text", "text": "b"},
            {"self_ref": "#/texts/4", "label": "text", "text": "foot"},
            {"self_ref": "#/texts/5", "label": "text", "text": "orphan"}
        ],
        "body": {"children": [
            {"$ref": "#/texts/0"},
            {"$ref": "#/groups/0"},
            {"$ref": "#/texts/0"}
        ]},
        "furniture": {"children": [{"$ref": "#/texts/4"}]}
    });
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;\n{}{}{}{}{}{}
         SELECT d.source AS source, d.text AS doc, c.chunk AS chunk, c.start AS start, c.\"end\" AS end_at, c.heading_path AS heading_path, c.meta_json AS meta_json
         FROM docs_chunks c JOIN docs d ON d.doc_id = c.doc_id
         ORDER BY d.doc_id, c.rowid;",
        insert(&blanks.to_string(), "blanks"),
        insert(&meta.to_string(), "meta"),
        insert(&table.to_string(), "table"),
        insert(&pairs.to_string(), "pairs"),
        insert(&picture.to_string(), "picture"),
        insert(&order.to_string(), "order"),
    ));

    let blanks_rows = rows_for(&rows, "blanks");
    assert_eq!(blanks_rows.len(), 2);
    assert_eq!(blanks_rows[0]["chunk"], "Hello");
    assert_eq!(blanks_rows[1]["chunk"], " ");
    assert_eq!(blanks_rows[0]["doc"], blanks.to_string());

    let meta_rows = rows_for(&rows, "meta");
    assert_eq!(meta_rows[0]["chunk"], "  A # ");
    assert_eq!(meta_rows[0]["heading_path"], "[]");
    assert_eq!(
        meta_rows[0]["meta_json"],
        r##"{"page":1,"section":null,"element":"#/texts/0"}"##
    );
    assert_eq!(meta_rows[1]["chunk"], "Details");
    assert_eq!(meta_rows[1]["heading_path"], r#"["  A # "]"#);
    assert_eq!(
        meta_rows[1]["meta_json"],
        r##"{"page":null,"section":"  A # ","element":"#/texts/1"}"##
    );
    assert_eq!(meta_rows[2]["chunk"], "a😀b");
    assert_eq!(meta_rows[2]["start"], 0);
    assert_eq!(meta_rows[2]["end_at"], 3);
    assert_eq!(meta_rows[2]["heading_path"], r#"["  A # ","Details"]"#);
    assert_eq!(
        meta_rows[2]["meta_json"],
        r##"{"page":3,"section":"Details","element":"#/texts/2"}"##
    );
    assert_eq!(meta_rows[3]["chunk"], "Head");
    assert_eq!(meta_rows[3]["heading_path"], "[]");
    assert_eq!(
        meta_rows[3]["meta_json"],
        r##"{"page":4,"section":null,"element":"#/texts/3"}"##
    );

    let table_rows = rows_for(&rows, "table");
    assert_eq!(table_rows.len(), 2);
    assert_eq!(table_rows[0]["chunk"], "A\t\nC\tD");
    assert_eq!(table_rows[0]["start"], 0);
    assert_eq!(table_rows[0]["end_at"], 6);
    assert_eq!(
        table_rows[0]["meta_json"],
        r##"{"page":2,"section":null,"element":"#/tables/0"}"##
    );
    assert_eq!(table_rows[1]["chunk"], "Caption");

    let pair_rows = rows_for(&rows, "pairs");
    assert_eq!(pair_rows.len(), 1);
    assert_eq!(pair_rows[0]["chunk"], "name\tada\nrole\tadmin");
    assert_eq!(pair_rows[0]["start"], 0);
    assert_eq!(pair_rows[0]["end_at"], 19);
    assert_eq!(
        pair_rows[0]["meta_json"],
        r##"{"page":null,"section":null,"element":"#/key_value_items/0"}"##
    );

    let picture_rows = rows_for(&rows, "picture");
    assert_eq!(picture_rows.len(), 1);
    assert_eq!(picture_rows[0]["chunk"], "Caption");
    assert_ne!(picture_rows[0]["chunk"], "file:///sentinel.png");

    let order_rows = rows_for(&rows, "order");
    let order_text: Vec<&str> = order_rows
        .iter()
        .map(|row| row["chunk"].as_str().unwrap())
        .collect();
    assert_eq!(order_text, vec!["parent", "child", "a", "b", "foot"]);
    assert_eq!(order_rows[2]["heading_path"], "[]");
    assert_eq!(
        order_rows[4]["meta_json"],
        r##"{"page":null,"section":null,"element":"#/texts/4"}"##
    );
}

#[test]
fn tc_183_element_ref_is_not_an_fts_hit() {
    let doc = json!({
        "schema_name": "DoclingDocument",
        "texts": [
            {
                "self_ref": "#/texts/0",
                "label": "title",
                "text": "Onlyheading",
                "children": [{"$ref": "#/texts/1"}]
            },
            {"self_ref": "#/texts/1", "label": "text", "text": "Hello"}
        ],
        "body": {"children": [{"$ref": "#/texts/0"}]}
    });
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;\n{}
         SELECT c.text AS text FROM docs_chunk_rows c
         WHERE c.chunk_id IN (SELECT rowid FROM docs_fts WHERE docs_fts MATCH 'Onlyheading');
         SELECT count(*) AS n FROM docs_fts WHERE docs_fts MATCH 'texts';
         SELECT count(*) AS n FROM docs_fts WHERE docs_fts MATCH 'Hello';",
        insert(&doc.to_string(), "s")
    ));
    let hits: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("text").is_some())
        .map(|row| row["text"].as_str().unwrap())
        .collect();
    assert_eq!(hits, vec!["Onlyheading"]);
    let counts: Vec<String> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("n").is_some())
        .map(|row| row["n"].to_string())
        .collect();
    assert_eq!(counts, vec!["0".to_string(), "1".to_string()]);
}
