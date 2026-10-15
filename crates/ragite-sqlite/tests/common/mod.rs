//! Shared sqlite3 CLI harness for extension tests.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

pub const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

pub fn extension_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("test executable");
    path.pop();
    path.pop();
    path.push("libragite.so");
    assert!(path.is_file(), "missing extension at {}", path.display());
    path
}

pub struct Out {
    pub status: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn sql(script: &str) -> Out {
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

pub fn ok(script: &str) -> Value {
    let out = sql(script);
    assert!(
        out.status,
        "sqlite failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    parse_json(&out.stdout)
}

pub fn err(script: &str) -> String {
    let out = sql(script);
    assert!(!out.status, "expected failure, got {}", out.stdout);
    format!("{}{}", out.stderr, out.stdout)
}

pub fn parse_json(stdout: &str) -> Value {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Value::Array(Vec::new());
    }
    let mut rows = Vec::new();
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    for item in stream {
        let value = item.unwrap_or_else(|e| panic!("json parse {e} in {stdout}"));
        if let Some(array) = value.as_array() {
            rows.extend(array.clone());
        }
    }
    Value::Array(rows)
}

pub fn cell(rows: &Value, index: usize, key: &str) -> String {
    rows.as_array().unwrap()[index][key]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| rows.as_array().unwrap()[index][key].to_string())
}
