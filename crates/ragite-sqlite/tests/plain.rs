//! SQL seams for a plain-text document. Cases are numbered in TESTSPEC.md.
//!
//! The host is the system `sqlite3` CLI. The extension is loaded into that
//! process and does not link a second copy of SQLite.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{HELLO_SHA, cell, err, extension_path, ok, parse_json, sql};

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
fn tc_10_tc_64_tc_65_notices_and_dependency_licenses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let notices = std::fs::read_to_string(root.join("THIRD_PARTY_NOTICES.md")).unwrap();
    assert!(notices.contains("potion-base-8M"));
    assert!(notices.contains("license: mit"));
    assert!(notices.contains("bf8b056651a2c21b8d2565580b8569da283cab23"));
    assert!(notices.contains("model2vec-rs"));
    assert!(root.join("THIRD_PARTY_NOTICES.md").is_file());
    assert!(!root.join("models/potion-base-8M/LICENSE").exists());
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("THIRD_PARTY_NOTICES.md"));
    for extra in [".github/release.json", "package.json", "Dockerfile"] {
        let path = root.join(extra);
        if path.is_file() {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(
                text.contains("THIRD_PARTY_NOTICES.md"),
                "{extra} does not ship THIRD_PARTY_NOTICES.md"
            );
        }
    }

    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--offline",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for package in meta["packages"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        if name == "ragite-core" || name == "ragite-sqlite" {
            continue;
        }
        match package["license"].as_str() {
            None => {
                assert_eq!(name, "model2vec-rs", "{name} has no license metadata");
                assert!(notices.contains("LICENSE file"));
            }
            Some(license) => assert!(
                permissive_license(license),
                "{name} license {license} is not compatible with MIT OR Apache-2.0"
            ),
        }
    }
}

fn permissive_license(license: &str) -> bool {
    matches!(
        license,
        "MIT"
            | "MIT OR Apache-2.0"
            | "MIT/Apache-2.0"
            | "Apache-2.0"
            | "Apache-2.0 / MIT"
            | "Apache-2.0/MIT"
            | "Apache-2.0 OR MIT"
            | "Apache-2.0 OR BSL-1.0"
            | "Apache-2.0 OR MIT OR Zlib"
            | "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT"
            | "(MIT OR Apache-2.0) AND Unicode-3.0"
            | "MIT OR Apache-2.0 OR LGPL-2.1-or-later"
            | "BSD-2-Clause OR Apache-2.0 OR MIT"
            | "Unlicense OR MIT"
    )
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
fn tc_73_tc_74_tc_109_tc_245_companion_is_read_only() {
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
    assert!(
        out.stderr.to_ascii_lowercase().contains("logic error"),
        "{}",
        out.stderr
    );
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

#[test]
fn tc_242_rename_is_refused_and_the_old_name_still_works() {
    let out = sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('before rename');
         ALTER TABLE docs RENAME TO docs9;
         SELECT text FROM docs;
         SELECT count(*) AS renamed FROM sqlite_master WHERE name GLOB 'docs9*';
         ALTER TABLE docs_chunks RENAME TO chunks9;
         SELECT chunk FROM docs_chunks;
         DROP TABLE docs;
         SELECT count(*) AS left FROM sqlite_master WHERE name GLOB 'docs*';",
    );
    assert!(!out.status, "rename was expected to fail: {}", out.stderr);
    assert!(
        out.stderr.contains("rename is not accepted"),
        "{}",
        out.stderr
    );
    let rows = parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "text"), "before rename");
    assert_eq!(cell(&rows, 1, "renamed"), "0");
    assert_eq!(cell(&rows, 2, "chunk"), "before rename");
    assert_eq!(cell(&rows, 3, "left"), "0");
}

#[test]
fn tc_243_caller_cannot_create_a_companion() {
    let out = sql(
        ".bail off
         CREATE VIRTUAL TABLE docs USING ragite;
         CREATE VIRTUAL TABLE evil USING ragite(companion=1, parent=docs);
         SELECT count(*) AS n FROM sqlite_master WHERE name = 'evil';
         CREATE VIRTUAL TABLE other USING ragite(parent=docs);
         SELECT count(*) AS n FROM sqlite_master WHERE name = 'other';
         SELECT count(*) AS n FROM docs;
         SELECT count(*) AS n FROM docs_chunks;",
    );
    assert!(!out.status, "{}", out.stderr);
    assert!(
        out.stderr.contains("argument is not accepted: companion"),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("argument is not accepted: parent"),
        "{}",
        out.stderr
    );
    let rows = parse_json(&out.stdout);
    assert_eq!(cell(&rows, 0, "n"), "0");
    assert_eq!(cell(&rows, 1, "n"), "0");
    assert_eq!(cell(&rows, 2, "n"), "0");
    assert_eq!(cell(&rows, 3, "n"), "0");

    let path = std::env::temp_dir().join(format!("ragite-reopen-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let ext = extension_path();
    let open = |script: &str| {
        let mut child = Command::new("sqlite3")
            .args(["-json", "-batch"])
            .arg(&path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("sqlite3");
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!(".load {}\n{script}", ext.display()).as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "stderr: {}\nstdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    open("CREATE VIRTUAL TABLE docs USING ragite;\nINSERT INTO docs(text) VALUES ('hello');\n");
    let rows = parse_json(&open("SELECT chunk FROM docs_chunks;\n"));
    assert_eq!(cell(&rows, 0, "chunk"), "hello");
    let _ = std::fs::remove_file(&path);
}
