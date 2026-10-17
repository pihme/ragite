//! Every `sql` block in docs/guide.md, run in order in one sqlite3 session (TC-271).

mod common;

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use common::extension_path;

const MARKER: &str = "ragite-guide-block-";

fn guide_blocks() -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let guide = std::fs::read_to_string(root.join("docs/guide.md")).unwrap();
    // Only a fence written exactly as ```sql at the start of a line is run.
    // Fail on any other SQL fence, so a block is never skipped silently.
    for (n, line) in guide.lines().enumerate() {
        let Some(info) = line.trim_start().strip_prefix("```") else {
            continue;
        };
        if info.trim().to_ascii_lowercase().starts_with("sql") {
            assert_eq!(
                line,
                "```sql",
                "docs/guide.md line {}: write SQL fences as ```sql at the start of a line",
                n + 1
            );
        }
    }
    let mut blocks = Vec::new();
    let mut rest = guide.as_str();
    while let Some((_, after)) = rest.split_once("\n```sql\n") {
        let (block, tail) = after.split_once("\n```").expect("closed sql block");
        blocks.push(block.to_string());
        rest = tail;
    }
    blocks
}

fn expectations(block: &str) -> Vec<&str> {
    block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("-- expect:"))
        .map(str::trim)
        .collect()
}

#[test]
fn tc_271_guide_sql_blocks_run_as_written() {
    let blocks = guide_blocks();
    assert!(blocks.len() >= 10, "found only {} sql blocks", blocks.len());
    assert!(blocks[0].contains("load_extension('./libragite')"));

    let mut script = String::new();
    for (i, block) in blocks.iter().enumerate() {
        script.push_str(&format!("SELECT '{MARKER}{i}' AS marker;\n"));
        script.push_str(block);
        script.push('\n');
    }

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
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let reached = stdout.matches(MARKER).count();
    assert!(
        out.status.success(),
        "guide block {} failed: {stderr}\n--- block ---\n{}",
        reached.saturating_sub(1),
        blocks[reached.saturating_sub(1)]
    );
    assert_eq!(reached, blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        let start = stdout.find(&format!("\"{MARKER}{i}\"")).unwrap();
        let end = stdout
            .find(&format!("\"{MARKER}{}\"", i + 1))
            .unwrap_or(stdout.len());
        let output = &stdout[start..end];
        for expect in expectations(block) {
            assert!(
                output.contains(expect),
                "guide block {i}: output does not contain {expect}\n--- output ---\n{output}"
            );
        }
    }
}
