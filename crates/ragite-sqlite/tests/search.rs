//! MATCH on the document table and the companion. Cases are numbered in TESTSPEC.md.

mod common;

use std::path::Path;
use std::process::{Command, Stdio};

use common::{Out, cell, err, extension_path, ok, parse_json};

fn score(rows: &serde_json::Value, index: usize) -> f64 {
    cell(rows, index, "score").parse().expect("score")
}

fn assert_score(got: f64, expected: f64) {
    assert!((got - expected).abs() < 1e-12, "score {got} != {expected}");
}

fn sql_file(path: &Path, script: &str) -> Out {
    let mut child = Command::new("sqlite3")
        .args(["-json", "-batch"])
        .arg(path)
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

#[test]
fn tc_25_tc_31_match_fuses_and_unmatched_score_is_null() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello world');
         SELECT score FROM docs;
         SELECT score FROM docs_chunks;
         SELECT doc_id, score FROM docs WHERE docs MATCH 'hello world';
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'hello world';");
    assert!(rows.as_array().unwrap()[0]["score"].is_null());
    assert!(rows.as_array().unwrap()[1]["score"].is_null());
    assert_eq!(cell(&rows, 2, "doc_id"), "1");
    assert_score(score(&rows, 2), 2.0 / 61.0);
    assert_eq!(cell(&rows, 3, "chunk"), "hello world");
    assert_score(score(&rows, 3), 2.0 / 61.0);
}

#[test]
fn tc_28_tc_29_tc_30_tc_76_tc_110_tc_224_best_chunk_and_one_list() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('unrelated zebra');
         INSERT INTO docs(text, source) VALUES ('alpha unique' || char(10) || char(10) || 'beta other', 'manual');
         SELECT doc_id, source, score FROM docs WHERE docs MATCH 'alpha unique';
         SELECT doc_id, chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'alpha unique';",
    );
    let docs = rows.as_array().unwrap();
    assert_eq!(cell(&rows, 0, "doc_id"), "2");
    assert_eq!(cell(&rows, 0, "source"), "manual");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "doc_id"), "1");
    let other = score(&rows, 1);
    assert!(
        (other - 1.0 / 62.0).abs() < 1e-12 || (other - 1.0 / 63.0).abs() < 1e-12,
        "vector-only document score {other}"
    );

    assert_eq!(cell(&rows, 2, "chunk"), "alpha unique");
    assert_score(score(&rows, 2), 2.0 / 61.0);
    assert_eq!(cell(&rows, 2, "doc_id"), "2");
    let mut rest = vec![score(&rows, 3), score(&rows, 4)];
    rest.sort_by(|left, right| right.partial_cmp(left).unwrap());
    assert_score(rest[0], 1.0 / 62.0);
    assert_score(rest[1], 1.0 / 63.0);
    assert_eq!(docs.len(), 5);
}

#[test]
fn tc_67_tc_68_tc_193_fts_folding_and_cosine_rank() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('installation');
         SELECT score FROM docs_chunks WHERE docs_chunks MATCH 'install';
         SELECT score FROM docs_chunks WHERE docs_chunks MATCH 'installation';
         SELECT score FROM docs_chunks WHERE docs_chunks MATCH 'install*';");
    assert_score(score(&rows, 0), 1.0 / 61.0);
    assert_score(score(&rows, 1), 2.0 / 61.0);
    assert_score(score(&rows, 2), 1.0 / 61.0);

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('Install');
         INSERT INTO docs(text) VALUES ('café');
         SELECT doc_id, score FROM docs WHERE docs MATCH 'install';
         SELECT doc_id, score FROM docs WHERE docs MATCH 'cafe';");
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    let cafe_neighbor = score(&rows, 1);
    assert!(
        (cafe_neighbor - 1.0 / 62.0).abs() < 1e-12,
        "café query should rank Install second, got {cafe_neighbor}"
    );
    assert_eq!(cell(&rows, 2, "doc_id"), "2");
    assert_score(score(&rows, 2), 2.0 / 61.0);
}

#[test]
fn tc_69_one_tokenizer_for_english_and_german() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('The cat sat');
         INSERT INTO docs(text) VALUES ('Die Katze sitzt');
         SELECT doc_id, score FROM docs WHERE docs MATCH 'The cat sat';
         SELECT doc_id, score FROM docs WHERE docs MATCH 'Die Katze sitzt';");
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    assert_score(score(&rows, 1), 1.0 / 62.0);
    assert_eq!(cell(&rows, 2, "doc_id"), "2");
    assert_score(score(&rows, 2), 2.0 / 61.0);
    assert_eq!(cell(&rows, 3, "doc_id"), "1");
    assert_score(score(&rows, 3), 1.0 / 62.0);
}

#[test]
fn tc_103_tc_104_limit_shortens_the_fused_list() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('zebra tail');
         INSERT INTO docs(text) VALUES ('other thing');
         INSERT INTO docs(text) VALUES ('target phrase');
         SELECT doc_id, score FROM docs WHERE docs MATCH 'target phrase';
         SELECT doc_id FROM docs WHERE docs MATCH 'target phrase' LIMIT 2;
         SELECT doc_id FROM docs_chunks WHERE docs_chunks MATCH 'target phrase' LIMIT 1;");
    assert_eq!(cell(&rows, 0, "doc_id"), "3");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    let mut rest = vec![cell(&rows, 1, "doc_id"), cell(&rows, 2, "doc_id")];
    rest.sort();
    assert_eq!(rest, vec!["1".to_string(), "2".to_string()]);
    assert_eq!(rows.as_array().unwrap().len(), 6);
    assert_eq!(cell(&rows, 3, "doc_id"), "3");
    assert_eq!(cell(&rows, 5, "doc_id"), "3");
}

#[test]
fn tc_27_same_scores_and_fixed_depth() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello world');
         SELECT score FROM docs WHERE docs MATCH 'hello world';
         SELECT score FROM docs WHERE text MATCH 'hello world';");
    assert_score(score(&rows, 0), score(&rows, 1));
    assert_score(score(&rows, 0), 2.0 / 61.0);
    let message = err("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello world');
         SELECT score FROM docs WHERE docs MATCH 'hello world' AND k = 1;");
    assert!(
        message.contains("k") || message.contains("no such column"),
        "{message}"
    );

    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for i in 1..=51 {
        script.push_str(&format!(
            "INSERT INTO docs(text) VALUES ('distinct chunk {i}');\n"
        ));
    }
    script.push_str(
        "SELECT count(*) AS n FROM docs WHERE docs MATCH 'no-such-token';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH 'no-such-token';",
    );
    let rows = ok(&script);
    assert_eq!(cell(&rows, 0, "n"), "50");
    assert_eq!(cell(&rows, 1, "n"), "50");
}

#[test]
fn tc_210_tc_211_tc_212_match_text_is_one_phrase() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('cats AND dogs');
         INSERT INTO docs(text) VALUES ('cats dogs');
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'cats AND dogs';");
    assert_eq!(cell(&rows, 0, "chunk"), "cats AND dogs");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "chunk"), "cats dogs");
    assert_score(score(&rows, 1), 1.0 / 62.0);

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('not cats');
         INSERT INTO docs(text) VALUES ('cats');
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'NOT cats';");
    assert_eq!(cell(&rows, 0, "chunk"), "not cats");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "chunk"), "cats");
    assert_score(score(&rows, 1), 1.0 / 62.0);

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('say \"hello\" there');
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'say \"hello\" there';");
    assert_eq!(cell(&rows, 0, "chunk"), "say \"hello\" there");
    assert_score(score(&rows, 0), 2.0 / 61.0);
}

#[test]
fn tc_115_tc_116_empty_skips_the_model_and_whitespace_is_embedded() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello world');
         SELECT count(*) AS n FROM docs WHERE docs MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '';
         SELECT count(*) AS n, score FROM docs_chunks WHERE docs_chunks MATCH '   ';");
    assert_eq!(cell(&rows, 0, "n"), "0");
    assert_eq!(cell(&rows, 1, "n"), "0");
    assert_eq!(cell(&rows, 2, "n"), "1");
    assert_score(score(&rows, 2), 1.0 / 61.0);

    let path = std::env::temp_dir().join(format!("ragite-search-model-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let created = sql_file(
        &path,
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello');
         UPDATE docs_model SET technical_id = 'missing@dead';",
    );
    assert!(created.status, "{}{}", created.stderr, created.stdout);
    let empty = sql_file(&path, "SELECT count(*) AS n FROM docs WHERE docs MATCH '';");
    assert!(empty.status, "{}{}", empty.stderr, empty.stdout);
    let rows = parse_json(&empty.stdout);
    assert_eq!(cell(&rows, 0, "n"), "0");
    let spaces = sql_file(&path, "SELECT score FROM docs WHERE docs MATCH '   ';");
    assert!(!spaces.status, "whitespace was expected to fail");
    let message = format!("{}{}", spaces.stderr, spaces.stdout);
    assert!(message.contains("model cannot be loaded"), "{message}");
    let text = sql_file(&path, "SELECT score FROM docs WHERE docs MATCH 'hello';");
    assert!(!text.status, "non-empty match was expected to fail");
    let message = format!("{}{}", text.stderr, text.stdout);
    assert!(message.contains("model cannot be loaded"), "{message}");
    let kept = sql_file(&path, "SELECT text FROM docs;");
    assert!(kept.status, "{}{}", kept.stderr, kept.stdout);
    let rows = parse_json(&kept.stdout);
    assert_eq!(cell(&rows, 0, "text"), "hello");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn tc_182_fts_indexes_chunk_text_only() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('body token only');
         SELECT count(*) AS n FROM docs_fts WHERE docs_fts MATCH '\"body token only\"';
         SELECT count(*) AS n FROM docs_fts WHERE docs_fts MATCH '\"not-in-the-chunk\"';
         SELECT score FROM docs_chunks WHERE docs_chunks MATCH 'not-in-the-chunk';
         SELECT score FROM docs_chunks WHERE docs_chunks MATCH 'body token only';");
    assert_eq!(cell(&rows, 0, "n"), "1");
    assert_eq!(cell(&rows, 1, "n"), "0");
    assert_score(score(&rows, 2), 1.0 / 61.0);
    assert_score(score(&rows, 3), 2.0 / 61.0);
}

#[test]
fn tc_227_match_column_does_not_change_the_search() {
    let script = "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('alpha unique' || char(10) || char(10) || 'beta other', 'manual');
         SELECT doc_id, score FROM docs WHERE docs MATCH 'alpha unique';
         SELECT doc_id, score FROM docs WHERE text MATCH 'alpha unique';
         SELECT doc_id, score FROM docs WHERE source MATCH 'alpha unique';
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'alpha unique';
         SELECT chunk, score FROM docs_chunks WHERE chunk MATCH 'alpha unique';";
    let rows = ok(script);
    assert_eq!(cell(&rows, 0, "doc_id"), cell(&rows, 1, "doc_id"));
    assert_eq!(cell(&rows, 0, "doc_id"), cell(&rows, 2, "doc_id"));
    assert_score(score(&rows, 0), score(&rows, 1));
    assert_score(score(&rows, 0), score(&rows, 2));
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 3, "chunk"), cell(&rows, 5, "chunk"));
    assert_score(score(&rows, 3), score(&rows, 5));
    assert_eq!(cell(&rows, 4, "chunk"), cell(&rows, 6, "chunk"));
    assert_score(score(&rows, 4), score(&rows, 6));
    assert_eq!(rows.as_array().unwrap().len(), 7);

    let filtered = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('target phrase', 'keep');
         INSERT INTO docs(text, source) VALUES ('unrelated zebra', 'drop');
         SELECT source FROM docs WHERE docs MATCH 'target phrase' AND source = 'keep';
         SELECT source FROM docs WHERE docs MATCH 'target phrase' AND source = 'drop';");
    assert_eq!(filtered.as_array().unwrap().len(), 2);
    assert_eq!(cell(&filtered, 0, "source"), "keep");
    assert_eq!(cell(&filtered, 1, "source"), "drop");
}
