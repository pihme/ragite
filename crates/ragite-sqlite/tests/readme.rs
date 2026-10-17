//! The README quickstart, run as written (TC-267).

mod common;

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use common::{extension_path, parse_json};

fn quickstart_sql() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    let section = readme
        .split_once("## Quickstart")
        .expect("README has a Quickstart section")
        .1;
    let block = section
        .split_once("```sql\n")
        .expect("Quickstart has an sql block")
        .1;
    block.split_once("```").expect("closed sql block").0.to_string()
}

#[test]
fn tc_267_readme_quickstart_runs_as_written() {
    let script = quickstart_sql();
    assert!(script.contains("load_extension('./libragite')"), "{script}");
    let dir = extension_path().parent().unwrap().to_path_buf();
    let mut child = Command::new("sqlite3")
        .args(["-json", "-bail", "-batch", ":memory:"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sqlite3");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "quickstart failed\nstderr: {}\nstdout: {stdout}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows = parse_json(&stdout);
    let rows = rows.as_array().unwrap();

    let model = rows
        .iter()
        .find(|r| r.get("technical_id").is_some())
        .expect("default model row");
    let id = model["technical_id"].as_str().unwrap();
    let readme = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
    )
    .unwrap();
    assert!(
        readme.contains(&format!("technical id `{id}`")),
        "README does not name the reported default id {id}"
    );
    assert_eq!(model["name"].as_str().unwrap(), id.split('@').next().unwrap());

    let doc = rows
        .iter()
        .find(|r| r.get("text").is_some() && r.get("score").is_none())
        .expect("document row");
    assert!(doc["text"].as_str().unwrap().starts_with("# Installation"));
    assert_eq!(doc["source"], "manual.md");

    let hits: Vec<_> = rows
        .iter()
        .filter(|r| r.get("score").is_some())
        .collect();
    assert!(!hits.is_empty(), "MATCH returned no chunk");
    assert!(hits.iter().all(|r| r["source"] == "manual.md"));
    assert!(hits.iter().all(|r| r["score"].is_number()));
}
