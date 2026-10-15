//! SQL seams for a plain-text document. Cases are numbered in TESTSPEC.md.
//!
//! The host is the system `sqlite3` CLI. The extension is loaded into that
//! process and does not link a second copy of SQLite.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

fn extension_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("test executable");
    path.pop();
    path.pop();
    path.push("libragite.so");
    assert!(path.is_file(), "missing extension at {}", path.display());
    path
}

struct Out {
    status: bool,
    stdout: String,
    stderr: String,
}

fn sql(script: &str) -> Out {
    let mut child = Command::new("sqlite3")
        .args(["-json", "-batch", ":memory:"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sqlite3");
    let full = format!(".load {}\n{script}", extension_path().display());
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(full.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    Out {
        status: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn ok(script: &str) -> Value {
    let out = sql(script);
    assert!(
        out.status,
        "sqlite failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    parse_json(&out.stdout)
}

fn err(script: &str) -> String {
    let out = sql(script);
    assert!(!out.status, "expected failure, got {}", out.stdout);
    format!("{}{}", out.stderr, out.stdout)
}

fn parse_json(stdout: &str) -> Value {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Value::Array(Vec::new());
    }
    let mut rows = Vec::new();
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    for item in stream {
        let value = item.unwrap_or_else(|err| panic!("json parse {err} in {stdout}"));
        if let Some(array) = value.as_array() {
            rows.extend(array.clone());
        }
    }
    Value::Array(rows)
}

fn cell(rows: &Value, index: usize, key: &str) -> String {
    rows.as_array().unwrap()[index][key]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| rows.as_array().unwrap()[index][key].to_string())
}

#[test]
fn tc_237_exports_sqlite3_ragite_init() {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(extension_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("sqlite3_ragite_init"), "{text}");
}

#[test]
fn tc_238_load_extension_registers_ragite() {
    let rows = ok("SELECT name FROM pragma_module_list WHERE name = 'ragite';");
    assert_eq!(cell(&rows, 0, "name"), "ragite");
}

#[test]
fn tc_1_core_does_not_depend_on_sqlite() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ragite-core/Cargo.toml");
    let text = std::fs::read_to_string(manifest).unwrap();
    assert!(!text.contains("libsqlite3-sys"));
}

#[test]
fn tc_4_extension_uses_libsqlite3_sys() {
    let text =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    assert!(text.contains("libsqlite3-sys"));
}

#[test]
fn tc_9_license_is_dual() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(workspace.contains("MIT OR Apache-2.0"));
    assert!(root.join("LICENSE-MIT").is_file());
    assert!(root.join("LICENSE-APACHE").is_file());
}

#[test]
fn tc_7_and_tc_12_no_onnx_pdf_or_llm() {
    let lock = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
    )
    .unwrap();
    for denied in [
        "name = \"ort\"",
        "name = \"onnxruntime\"",
        "name = \"lopdf\"",
        "name = \"pdfium\"",
        "name = \"genai\"",
    ] {
        assert!(!lock.contains(denied), "{denied} is linked");
    }
}

#[test]
fn tc_5_tc_77_tc_90_tc_98_insert_plain_text() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         SELECT text, source, format, hash FROM docs;",
    );
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 0, "source"), "");
    assert_eq!(cell(&rows, 0, "format"), "text/plain");
    assert_eq!(cell(&rows, 0, "hash"), HELLO_SHA);
}

#[test]
fn tc_13_tc_15_tc_174_shadow_tables() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT name, sql FROM sqlite_master WHERE name GLOB 'docs*' ORDER BY name;",
    );
    let names: Vec<String> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap().to_string())
        .collect();
    for required in [
        "docs",
        "docs_chunks",
        "docs_chunk_rows",
        "docs_documents",
        "docs_fts",
        "docs_vec",
    ] {
        assert!(names.iter().any(|name| name == required), "{names:?}");
    }
    assert!(names.iter().all(|name| !name.starts_with("docs_chunks_")));
    let doc_sql = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "docs_documents")
        .unwrap()["sql"]
        .as_str()
        .unwrap();
    assert!(doc_sql.contains("AUTOINCREMENT"), "{doc_sql}");
    assert!(doc_sql.contains("doc_id") && doc_sql.contains("hash"));
    let chunk_sql = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "docs_chunk_rows")
        .unwrap()["sql"]
        .as_str()
        .unwrap();
    assert!(chunk_sql.contains("AUTOINCREMENT"), "{chunk_sql}");
    assert!(chunk_sql.contains("heading_path") && chunk_sql.contains("meta_json"));
}

#[test]
fn tc_14_second_table_has_its_own_shadows() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         CREATE VIRTUAL TABLE notes USING ragite;
         INSERT INTO notes(text) VALUES ('note');
         SELECT
           (SELECT count(*) FROM sqlite_master WHERE name GLOB 'notes_*') AS notes,
           (SELECT count(*) FROM docs_documents) AS docs;",
    );
    assert!(cell(&rows, 0, "notes").parse::<i64>().unwrap() >= 5);
    assert_eq!(cell(&rows, 0, "docs"), "0");
}

#[test]
fn tc_17_tc_50_rows_share_chunk_id() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', 'manual');
         SELECT c.chunk_id AS chunk_id, v.rowid AS vec_row, f.rowid AS fts_row
         FROM docs_chunk_rows c, docs_vec v, docs_fts f
         WHERE v.rowid = c.chunk_id AND f.rowid = c.chunk_id;",
    );
    assert_eq!(cell(&rows, 0, "chunk_id"), cell(&rows, 0, "vec_row"));
    assert_eq!(cell(&rows, 0, "chunk_id"), cell(&rows, 0, "fts_row"));
}

#[test]
fn tc_18_rollback_removes_the_insert() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         BEGIN;
         INSERT INTO docs(text) VALUES ('hello');
         ROLLBACK;
         SELECT
           (SELECT count(*) FROM docs_documents) AS docs,
           (SELECT count(*) FROM docs_chunk_rows) AS chunks,
           (SELECT count(*) FROM docs_vec) AS vecs,
           (SELECT count(*) FROM docs_fts) AS fts;",
    );
    assert_eq!(cell(&rows, 0, "docs"), "0");
    assert_eq!(cell(&rows, 0, "chunks"), "0");
    assert_eq!(cell(&rows, 0, "vecs"), "0");
    assert_eq!(cell(&rows, 0, "fts"), "0");
}

#[test]
fn tc_33_tc_34_tc_35_tc_45_plain_chunk_shape() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', 's');
         SELECT chunk, doc_id, source, start, end, heading_path, meta_json, score
         FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "hello");
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_eq!(cell(&rows, 0, "source"), "s");
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "5");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[0]["meta_json"].is_null());
    assert!(rows.as_array().unwrap()[0]["score"].is_null());
}

#[test]
fn tc_51_join_vector_back_to_text() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('hello', 'manual');
         SELECT c.text AS text, d.source AS source, c.start AS start, c.end AS end
         FROM docs_vec v
         JOIN docs_chunk_rows c ON c.chunk_id = v.rowid
         JOIN docs_documents d ON d.doc_id = c.doc_id
         WHERE v.embedding MATCH (SELECT embedding FROM docs_vec LIMIT 1)
           AND k = 1;",
    );
    assert_eq!(cell(&rows, 0, "text"), "hello");
    assert_eq!(cell(&rows, 0, "source"), "manual");
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "5");
}

#[test]
fn tc_66_tc_192_fts_and_vec_definitions() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT name, sql FROM sqlite_master WHERE name IN ('docs_fts', 'docs_vec');",
    );
    let fts = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "docs_fts")
        .unwrap()["sql"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(fts.contains("unicode61"), "{fts}");
    assert!(!fts.contains("porter"), "{fts}");
    let vec_sql = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "docs_vec")
        .unwrap()["sql"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(vec_sql.contains("vec0"), "{vec_sql}");
    assert!(vec_sql.contains("cosine"), "{vec_sql}");
    assert!(vec_sql.contains("float[256]"), "{vec_sql}");
}

#[test]
fn tc_73_tc_74_tc_109_companion_is_read_only() {
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs_chunks(chunk) VALUES ('no');",
    );
    assert!(message.contains("docs_chunks"), "{message}");
    let out = sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         DROP TABLE docs_chunks;
         SELECT count(*) AS n FROM docs;
         SELECT count(*) AS n FROM docs_chunks;",
    );
    assert!(!out.status, "drop was expected to fail: {}", out.stderr);
    let rows = parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "n"), "0");
    assert_eq!(cell(&rows, 1, "n"), "0");
}

#[test]
fn tc_82_tc_88_tc_217_tc_218_declared_columns() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT 'docs' AS surface, cid, name, type FROM pragma_table_info('docs')
         UNION ALL
         SELECT 'chunks', cid, name, type FROM pragma_table_info('docs_chunks')
         ORDER BY surface, cid;"
    );
    let docs: Vec<(String, String)> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["surface"] == "docs")
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["type"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        docs,
        vec![
            ("doc_id".into(), "INTEGER".into()),
            ("text".into(), "TEXT".into()),
            ("source".into(), "TEXT".into()),
            ("format".into(), "TEXT".into()),
            ("hash".into(), "TEXT".into()),
            ("score".into(), "REAL".into()),
        ]
    );
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT format FROM docs_chunks;",
    );
    assert!(!message.is_empty());
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT meta_json FROM docs;",
    );
    assert!(!message.is_empty());
}

#[test]
fn tc_107_existing_companion_blocks_create() {
    let message = err(
        "CREATE TABLE docs_chunks(x);
         CREATE VIRTUAL TABLE docs USING ragite;",
    );
    assert!(message.contains("docs_chunks"), "{message}");
    let rows = ok(
        "CREATE TABLE docs_chunks(x);
         SELECT
           (SELECT count(*) FROM sqlite_master WHERE name = 'docs_documents') AS docs,
           (SELECT count(*) FROM docs_chunks) AS still;",
    );
    assert_eq!(cell(&rows, 0, "docs"), "0");
    assert_eq!(cell(&rows, 0, "still"), "0");
}

#[test]
fn tc_108_existing_fts_blocks_create() {
    let message = err(
        "CREATE VIRTUAL TABLE docs_fts USING fts5(text);
         CREATE VIRTUAL TABLE docs USING ragite;",
    );
    assert!(message.contains("docs_fts"), "{message}");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs_fts USING fts5(text);
         SELECT count(*) AS n FROM sqlite_master
         WHERE name IN ('docs', 'docs_chunks', 'docs_documents');",
    );
    assert_eq!(cell(&rows, 0, "n"), "0");
}

#[test]
fn tc_172_tc_190_tc_191_ids_start_at_one() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         SELECT rowid, doc_id FROM docs;",
    );
    assert_eq!(cell(&rows, 0, "rowid"), "1");
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         SELECT rowid FROM docs_chunks;",
    );
    assert_eq!(cell(&rows, 0, "rowid"), "1");
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT chunk_id FROM docs;",
    );
    assert!(!message.is_empty());
    let message = err(
        "CREATE VIRTUAL TABLE docs USING ragite;
         SELECT chunk_id FROM docs_chunks;",
    );
    assert!(!message.is_empty());
}

#[test]
fn tc_77_stored_text_is_unchanged_for_each_format() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# Title', 'text/markdown');
         INSERT INTO docs(text, format) VALUES ('<p>hi</p>', 'text/html');
         INSERT INTO docs(text, format) VALUES ('{\"schema_name\":\"DoclingDocument\"}', 'application/vnd.docling.document+json');
         SELECT text, format FROM docs ORDER BY doc_id;",
    );
    assert_eq!(cell(&rows, 0, "text"), "# Title");
    assert_eq!(cell(&rows, 1, "text"), "<p>hi</p>");
    assert_eq!(
        cell(&rows, 2, "text"),
        "{\"schema_name\":\"DoclingDocument\"}"
    );
}

#[test]
fn tc_213_tc_214_tc_215_main_temp_and_attached() {
    let dir = std::env::temp_dir().join(format!("ragite-archive-{}", std::process::id()));
    let _ = std::fs::remove_file(&dir);
    let path = dir.to_string_lossy().replace('\'', "''");
    let rows = ok(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;
         CREATE VIRTUAL TABLE temp.scratch USING ragite;
         ATTACH '{path}' AS archive;
         CREATE VIRTUAL TABLE archive.docs USING ragite;
         SELECT 'main' AS db, name FROM main.sqlite_master WHERE name GLOB 'docs*' OR name GLOB 'scratch*'
         UNION ALL
         SELECT 'temp', name FROM temp.sqlite_master WHERE name GLOB 'scratch*'
         UNION ALL
         SELECT 'archive', name FROM archive.sqlite_master;",
    ));
    let names = |db: &str| -> Vec<String> {
        rows.as_array()
            .unwrap()
            .iter()
            .filter(|row| row["db"] == db)
            .map(|row| row["name"].as_str().unwrap().to_string())
            .collect()
    };
    let main = names("main");
    assert!(main.contains(&"docs_chunks".to_string()), "{main:?}");
    assert!(main.iter().all(|name| !name.starts_with("scratch")), "{main:?}");
    let temp = names("temp");
    assert!(temp.contains(&"scratch_chunks".to_string()), "{temp:?}");
    let archive = names("archive");
    assert!(archive.contains(&"docs".to_string()), "{archive:?}");
    assert!(archive.contains(&"docs_chunks".to_string()), "{archive:?}");
    assert!(archive.iter().all(|name| !name.contains('.')), "{archive:?}");
    let _ = std::fs::remove_file(&dir);
}
