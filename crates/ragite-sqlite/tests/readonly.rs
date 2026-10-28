//! Load and query on a connection that cannot write. Cases are numbered in TESTSPEC.md.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{Out, extension_path, parse_json};
use serde_json::Value;

const SETUP: &str = "\
CREATE VIRTUAL TABLE docs USING ragite;
INSERT INTO docs(text) VALUES ('Install the backup');
INSERT INTO docs(text) VALUES ('café menu');
";

const QUERIES: &str = "\
SELECT doc_id, text FROM docs;
SELECT doc_id, score FROM docs WHERE docs MATCH 'install';
SELECT doc_id, chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'install';
SELECT doc_id, score FROM docs WHERE docs MATCH 'cafe';
SELECT doc_id, chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'cafe';
SELECT doc_id, score FROM docs WHERE docs MATCH '***';
";

/// `PRAGMA query_only` is set before the extension loads. Datasette does this
/// on its in-memory `_internal` database, and extension init used to write.
fn sql_query_only(script: &str) -> Out {
    let mut child = Command::new("sqlite3")
        .args(["-json", "-batch", ":memory:"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sqlite3");
    let full = format!(
        "PRAGMA query_only=ON;\n.load {}\n{script}",
        extension_path().display()
    );
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

#[test]
fn tc_287_extension_loads_when_the_connection_cannot_write() {
    let out = sql_query_only("SELECT 'loaded' AS ok;");
    assert!(
        out.status,
        "sqlite failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    let rows = parse_json(&out.stdout);
    assert_eq!(
        rows.as_array().unwrap()[0]["ok"].as_str().unwrap(),
        "loaded"
    );
}

fn run_sqlite(database: &str, prefix: &str, script: &str) -> Out {
    let mut child = Command::new("sqlite3")
        .args(["-json", "-batch", database])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sqlite3");
    let full = format!("{prefix}{script}");
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

fn load_line() -> String {
    format!(".load {}\n", extension_path().display())
}

fn fixture(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ragite-ro-{label}-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let out = run_sqlite(&path.display().to_string(), &load_line(), SETUP);
    assert!(
        out.status,
        "setup failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    path
}

fn writable_rows(path: &Path) -> Value {
    let out = run_sqlite(&path.display().to_string(), &load_line(), QUERIES);
    assert!(
        out.status,
        "writable query failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    parse_json(&out.stdout)
}

fn assert_same_rows(label: &str, got: &Out, expect: &Value) {
    assert!(
        got.status,
        "{label} failed\nstderr: {}\nstdout: {}",
        got.stderr, got.stdout
    );
    assert_eq!(parse_json(&got.stdout), *expect, "{label}");
}

#[test]
fn tc_288_queries_on_a_query_only_connection_match_a_writable_one() {
    let path = fixture("query-only");
    let expect = writable_rows(&path);
    let out = run_sqlite(
        &path.display().to_string(),
        &format!("PRAGMA query_only=ON;\n{}", load_line()),
        QUERIES,
    );
    assert_same_rows("query_only", &out, &expect);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn tc_289_queries_on_mode_ro_match_a_writable_connection() {
    let path = fixture("mode-ro");
    let expect = writable_rows(&path);
    let uri = format!("file:{}?mode=ro", path.display());
    let out = run_sqlite(&uri, &load_line(), QUERIES);
    assert_same_rows("mode=ro", &out, &expect);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn tc_290_queries_on_immutable_match_a_writable_connection() {
    let path = fixture("immutable");
    let expect = writable_rows(&path);
    let uri = format!("file:{}?immutable=1", path.display());
    let out = run_sqlite(&uri, &load_line(), QUERIES);
    assert_same_rows("immutable=1", &out, &expect);
    let _ = std::fs::remove_file(&path);
}
