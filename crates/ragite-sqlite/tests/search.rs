//! MATCH on the document table and the companion. Cases are numbered in TESTSPEC.md.

mod common;

use std::path::Path;
use std::process::{Command, Stdio};

use common::{Out, cell, err, extension_path, ok, parse_json, sql};

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

fn plan_text(script: &str) -> String {
    let out = sql(script);
    assert!(
        out.status,
        "explain failed\nstderr: {}\nstdout: {}",
        out.stderr, out.stdout
    );
    out.stdout
}

#[test]
fn tc_251_to_254_rowid_equality_is_a_unique_lookup() {
    let setup = "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source) VALUES ('alpha unique', 'a');
         INSERT INTO docs(text, source) VALUES ('beta other', 'b');";

    let hit = ok(&format!(
        "{setup}
         SELECT chunk, doc_id, source, score FROM docs_chunks WHERE rowid = 2;
         SELECT text, source, score FROM docs WHERE rowid = 2;"
    ));
    assert_eq!(cell(&hit, 0, "chunk"), "beta other");
    assert_eq!(cell(&hit, 0, "doc_id"), "2");
    assert_eq!(cell(&hit, 0, "source"), "b");
    assert!(hit.as_array().unwrap()[0]["score"].is_null());
    assert_eq!(cell(&hit, 1, "text"), "beta other");
    assert_eq!(cell(&hit, 1, "source"), "b");
    assert!(hit.as_array().unwrap()[1]["score"].is_null());

    let fractional = ok(&format!(
        "{setup}
         SELECT rowid AS id FROM docs_chunks WHERE rowid = 2.5;
         SELECT rowid AS id FROM docs WHERE rowid = 2.5;
         SELECT rowid AS id FROM docs_chunks WHERE rowid = '2.5';
         SELECT rowid AS id FROM docs WHERE rowid = '2.5';"
    ));
    assert!(
        fractional.as_array().unwrap().is_empty(),
        "non-integral rowid returned {fractional}"
    );
    let fractional_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN SELECT chunk FROM docs_chunks WHERE rowid = 2.5;"
    ));
    assert!(
        fractional_plan.contains("INDEX 2:rowid"),
        "non-integral plan: {fractional_plan}"
    );

    let integral = ok(&format!(
        "{setup}
         SELECT rowid AS id FROM docs_chunks WHERE rowid = 2.0;
         SELECT rowid AS id FROM docs WHERE rowid = 2.0;
         SELECT rowid AS id FROM docs_chunks WHERE rowid = '2';
         SELECT rowid AS id FROM docs WHERE rowid = '2';"
    ));
    assert_eq!(integral.as_array().unwrap().len(), 4);
    for index in 0..4 {
        assert_eq!(cell(&integral, index, "id"), "2");
    }

    // An empty MATCH returns no rows (REQ-43). A rowid-only plan would still
    // return the chunk, so this locks the search plan when both are usable.
    let matched = ok(&format!(
        "{setup}
         SELECT chunk FROM docs_chunks WHERE rowid = 2 AND docs_chunks MATCH '';"
    ));
    assert!(matched.as_array().unwrap().is_empty());
    let match_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN
         SELECT chunk FROM docs_chunks WHERE rowid = 2 AND docs_chunks MATCH 'alpha unique';"
    ));
    assert!(
        match_plan.contains("INDEX 1:"),
        "match-and-rowid plan: {match_plan}"
    );
    assert!(
        !match_plan.contains("INDEX 2:"),
        "match-and-rowid used the rowid plan: {match_plan}"
    );

    let missing = ok(&format!(
        "{setup}
         SELECT chunk FROM docs_chunks WHERE rowid = 99;
         SELECT text FROM docs WHERE rowid = 99;"
    ));
    assert!(missing.as_array().unwrap().is_empty());

    let scan = ok(&format!(
        "{setup}
         SELECT chunk FROM docs_chunks;
         SELECT chunk FROM docs_chunks WHERE rowid > 1;"
    ));
    assert_eq!(cell(&scan, 0, "chunk"), "alpha unique");
    assert_eq!(cell(&scan, 1, "chunk"), "beta other");
    assert_eq!(cell(&scan, 2, "chunk"), "beta other");
    assert_eq!(scan.as_array().unwrap().len(), 3);

    let lookup_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN SELECT chunk FROM docs_chunks WHERE rowid = 2;"
    ));
    assert!(
        lookup_plan.contains("INDEX 2:rowid"),
        "point lookup plan: {lookup_plan}"
    );

    let scan_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN SELECT chunk FROM docs_chunks;"
    ));
    assert!(
        scan_plan.contains("INDEX 0:"),
        "full scan plan: {scan_plan}"
    );
    assert!(
        !scan_plan.contains("INDEX 2:"),
        "full scan used the rowid plan: {scan_plan}"
    );

    let range_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN SELECT chunk FROM docs_chunks WHERE rowid > 1;"
    ));
    assert!(
        range_plan.contains("INDEX 0:"),
        "inequality plan: {range_plan}"
    );
    assert!(
        !range_plan.contains("INDEX 2:"),
        "inequality used the rowid plan: {range_plan}"
    );

    let join = ok(&format!(
        "{setup}
         SELECT c.chunk AS chunk
         FROM docs_vec v
         JOIN docs_chunks c ON c.rowid = v.rowid
         WHERE v.embedding MATCH (SELECT embedding FROM docs_vec WHERE rowid = 2)
           AND k = 1;"
    ));
    assert_eq!(cell(&join, 0, "chunk"), "beta other");

    let join_plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN
         SELECT c.chunk
         FROM docs_vec v
         JOIN docs_chunks c ON c.rowid = v.rowid
         WHERE v.embedding MATCH (SELECT embedding FROM docs_vec WHERE rowid = 2)
           AND k = 1;"
    ));
    assert!(
        join_plan.contains("INDEX 2:rowid"),
        "knn join plan: {join_plan}"
    );
}

/// D from SPEC section 8.1. An exact-query Document fills each fusion slot.
const FUSION_DEPTH: usize = 50;

#[test]
fn tc_272_source_equality_searches_inside_that_source() {
    let query = "filtertoken alpha";
    let kept = "filtertoken alpha sits inside a longer paragraph";
    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO docs(text, source) VALUES ('{query}', 'other');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO docs(text, source) VALUES ('{kept}', 'keep');\n"
    ));
    script.push_str(&format!(
        "SELECT source FROM docs WHERE docs MATCH '{query}';
         SELECT source, score FROM docs WHERE docs MATCH '{query}' AND source = 'keep';
         SELECT source, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}' AND source = 'keep';
         CREATE VIRTUAL TABLE alone USING ragite;
         INSERT INTO alone(text) VALUES ('{kept}');
         SELECT score FROM alone WHERE alone MATCH '{query}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    let unfiltered: Vec<_> = rows
        .iter()
        .filter(|row| row.get("chunk").is_none() && row.get("score").is_none())
        .collect();
    assert_eq!(unfiltered.len(), FUSION_DEPTH);
    assert!(unfiltered.iter().all(|row| row["source"] == "other"));

    let filtered_docs: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.get("source").is_some() && row.get("score").is_some() && row.get("chunk").is_none()
        })
        .collect();
    assert_eq!(filtered_docs.len(), 1);
    assert_eq!(filtered_docs[0]["source"], "keep");

    let filtered_chunks: Vec<_> = rows
        .iter()
        .filter(|row| row.get("chunk").is_some())
        .collect();
    assert_eq!(filtered_chunks.len(), 1);
    assert_eq!(filtered_chunks[0]["source"], "keep");
    assert_eq!(filtered_chunks[0]["chunk"], kept);

    let alone = rows.last().unwrap()["score"].as_f64().expect("alone score");
    assert_score(alone, 2.0 / 61.0);
    assert_score(
        filtered_docs[0]["score"].as_f64().expect("doc score"),
        alone,
    );
    assert_score(
        filtered_chunks[0]["score"].as_f64().expect("chunk score"),
        alone,
    );
}

#[test]
fn tc_273_format_hash_and_text_narrow_the_search() {
    let query = "filtertoken beta";
    let kept = "filtertoken beta sits inside a longer paragraph";
    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO docs(text, source, format) VALUES ('{query}', 'keep', 'text/html');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO docs(text, source, format) VALUES ('{kept}', 'keep', 'text/plain');\n"
    ));
    script.push_str(&format!(
        "SELECT format FROM docs WHERE docs MATCH '{query}' AND source = 'keep';
         SELECT format, score FROM docs
           WHERE docs MATCH '{query}' AND source = 'keep' AND format = 'text/plain';
         SELECT format, score FROM docs
           WHERE docs MATCH '{query}'
             AND hash = (SELECT hash FROM docs WHERE format = 'text/plain');
         SELECT format, score FROM docs
           WHERE docs MATCH '{query}'
             AND text = (SELECT text FROM docs WHERE format = 'text/plain');
         SELECT format FROM docs
           WHERE docs MATCH '{query}' AND format = 'text/html' AND source = 'keep';
         CREATE VIRTUAL TABLE alone USING ragite;
         INSERT INTO alone(text, format) VALUES ('{kept}', 'text/plain');
         SELECT score FROM alone WHERE alone MATCH '{query}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    let source_only: Vec<_> = rows
        .iter()
        .filter(|row| row.get("score").is_none() && row["format"] == "text/html")
        .collect();
    // The first SELECT is the source-only search. Later the html search repeats those formats.
    assert!(source_only.len() >= FUSION_DEPTH);
    assert!(
        rows.iter()
            .take(FUSION_DEPTH)
            .all(|row| row["format"] == "text/html")
    );
    assert!(
        rows.iter()
            .take(FUSION_DEPTH)
            .all(|row| row.get("score").is_none())
    );

    let narrowed: Vec<_> = rows
        .iter()
        .filter(|row| row.get("score").is_some() && row["format"] == "text/plain")
        .collect();
    assert_eq!(narrowed.len(), 3, "plain format, hash, and text searches");
    let alone = rows.last().unwrap()["score"].as_f64().expect("alone score");
    assert_score(alone, 2.0 / 61.0);
    for row in narrowed {
        assert_score(row["score"].as_f64().expect("score"), alone);
    }

    let html: Vec<_> = rows
        .iter()
        .skip(FUSION_DEPTH)
        .filter(|row| row.get("score").is_none() && row["format"] == "text/html")
        .collect();
    assert_eq!(html.len(), FUSION_DEPTH);
    assert!(
        rows.iter()
            .all(|row| row["format"] != "text/plain" || row.get("score").is_some())
    );
}

#[test]
fn tc_274_doc_id_rowid_and_source_in_narrow_the_search() {
    let query = "filtertoken gamma";
    let kept = "filtertoken gamma sits inside a longer paragraph";
    let also = "filtertoken gamma appears among other words";
    let mut script = String::from(
        "CREATE VIRTUAL TABLE docs USING ragite;
         CREATE VIRTUAL TABLE pair USING ragite;\n",
    );
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO docs(text, source) VALUES ('{query}', 'other');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO docs(text, source) VALUES ('{kept}', 'keep');
         INSERT INTO docs(text, source) VALUES ('{also}', 'also');
         INSERT INTO pair(text, source) VALUES ('{kept}', 'keep');
         INSERT INTO pair(text, source) VALUES ('{also}', 'also');
         SELECT 'open' AS q, source FROM docs WHERE docs MATCH '{query}';
         SELECT 'doc' AS q, source, score FROM docs
           WHERE docs MATCH '{query}'
             AND doc_id = (SELECT doc_id FROM docs WHERE source = 'keep');
         SELECT 'row' AS q, source, score FROM docs
           WHERE docs MATCH '{query}'
             AND rowid = (SELECT doc_id FROM docs WHERE source = 'keep');
         SELECT 'inn' AS q, source, score FROM docs
           WHERE docs MATCH '{query}' AND source IN ('keep', 'also');
         SELECT 'pair' AS q, source, score FROM pair WHERE pair MATCH '{query}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    let unfiltered: Vec<_> = rows.iter().filter(|row| row["q"] == "open").collect();
    assert_eq!(unfiltered.len(), FUSION_DEPTH);
    assert!(unfiltered.iter().all(|row| row["source"] == "other"));

    for label in ["doc", "row"] {
        let hit: Vec<_> = rows.iter().filter(|row| row["q"] == label).collect();
        assert_eq!(hit.len(), 1, "{label}");
        assert_eq!(hit[0]["source"], "keep");
        assert_score(hit[0]["score"].as_f64().expect("score"), 2.0 / 61.0);
    }

    let inn: Vec<_> = rows.iter().filter(|row| row["q"] == "inn").collect();
    let pair: Vec<_> = rows.iter().filter(|row| row["q"] == "pair").collect();
    assert_eq!(inn.len(), 2);
    assert_eq!(pair.len(), 2);
    for wanted in ["keep", "also"] {
        let from_pair = pair
            .iter()
            .find(|row| row["source"] == wanted)
            .expect(wanted);
        let from_in = inn
            .iter()
            .find(|row| row["source"] == wanted)
            .expect(wanted);
        assert_score(
            from_in["score"].as_f64().expect("in score"),
            from_pair["score"].as_f64().expect("pair score"),
        );
    }

    let explain = plan_text(&format!(
        "{script}
         EXPLAIN QUERY PLAN SELECT source FROM docs
           WHERE docs MATCH '{query}' AND rowid = (SELECT doc_id FROM docs WHERE source = 'keep');"
    ));
    assert!(
        explain.contains("INDEX 1:"),
        "rowid with MATCH stays on the search plan: {explain}"
    );
}

#[test]
fn tc_275_chunk_columns_and_meta_json_narrow_the_search() {
    let query = "filtertoken delta";
    let body = "filtertoken delta sits inside a longer paragraph";
    let heading = "# Kept";
    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO docs(text, source) VALUES ('{query}', 'other');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO docs(text, source, format) VALUES (
            '{heading}' || char(10) || char(10) || '{body}', 'keep', 'text/markdown');
         SELECT 'open' AS q, chunk FROM docs_chunks WHERE docs_chunks MATCH '{query}';
         SELECT 'chunk' AS q, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND chunk = (SELECT chunk FROM docs_chunks WHERE heading_path = '[\"Kept\"]');
         SELECT 'span' AS q, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND start = (SELECT start FROM docs_chunks WHERE heading_path = '[\"Kept\"]')
             AND \"end\" = (SELECT \"end\" FROM docs_chunks WHERE heading_path = '[\"Kept\"]');
         SELECT 'path' AS q, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}' AND heading_path = '[\"Kept\"]';
         SELECT 'badstart' AS q, chunk FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND start = (SELECT start FROM docs_chunks WHERE heading_path = '[\"Kept\"]') + 1
             AND \"end\" = (SELECT \"end\" FROM docs_chunks WHERE heading_path = '[\"Kept\"]');
         SELECT 'badend' AS q, chunk FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND start = (SELECT start FROM docs_chunks WHERE heading_path = '[\"Kept\"]')
             AND \"end\" = (SELECT \"end\" FROM docs_chunks WHERE heading_path = '[\"Kept\"]') + 1;
         SELECT 'root' AS q, chunk FROM docs_chunks
           WHERE docs_chunks MATCH '{query}' AND heading_path = '[]';
         SELECT 'doc' AS q, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND doc_id = (SELECT doc_id FROM docs WHERE source = 'keep');
         SELECT 'crow' AS q, chunk, score FROM docs_chunks
           WHERE docs_chunks MATCH '{query}'
             AND rowid = (SELECT rowid FROM docs_chunks WHERE heading_path = '[\"Kept\"]');"
    ));
    let docling = format!(
        "{{\"schema_name\":\"DoclingDocument\",\"texts\":[{{\"self_ref\":\"#/texts/0\",\"label\":\"text\",\"text\":\"{body}\"}}],\"body\":{{\"children\":[{{\"$ref\":\"#/texts/0\"}}]}}}}"
    );
    script.push_str("CREATE VIRTUAL TABLE papers USING ragite;\n");
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO papers(text, source) VALUES ('{query}', 'other');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO papers(text, source, format) VALUES (
            '{docling}', 'keep', 'application/vnd.docling.document+json');
         SELECT 'dopen' AS q, chunk FROM papers_chunks WHERE papers_chunks MATCH '{query}';
         SELECT 'meta' AS q, chunk, score FROM papers_chunks
           WHERE papers_chunks MATCH '{query}'
             AND meta_json = (
               SELECT meta_json FROM papers_chunks WHERE chunk = '{body}'
             );
         SELECT 'othermeta' AS q, chunk FROM papers_chunks
           WHERE papers_chunks MATCH '{query}' AND meta_json = '{{\"page\":9}}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    let open: Vec<_> = rows.iter().filter(|row| row["q"] == "open").collect();
    assert_eq!(open.len(), FUSION_DEPTH);
    assert!(
        open.iter()
            .all(|row| row["chunk"] != body && row["chunk"] != heading)
    );
    for label in ["chunk", "span", "path"] {
        let hit: Vec<_> = rows.iter().filter(|row| row["q"] == label).collect();
        assert_eq!(hit.len(), 1, "{label}");
        assert_eq!(hit[0]["chunk"], body);
        assert_score(hit[0]["score"].as_f64().expect("score"), 2.0 / 61.0);
    }
    let doc: Vec<_> = rows.iter().filter(|row| row["q"] == "doc").collect();
    assert!(
        doc.iter()
            .all(|row| row["chunk"] == body || row["chunk"] == heading)
    );
    let doc_body = doc
        .iter()
        .find(|row| row["chunk"] == body)
        .expect("doc body");
    assert_score(doc_body["score"].as_f64().expect("score"), 2.0 / 61.0);
    let crow: Vec<_> = rows.iter().filter(|row| row["q"] == "crow").collect();
    assert_eq!(crow.len(), 1);
    assert_eq!(crow[0]["chunk"], body);
    assert_score(crow[0]["score"].as_f64().expect("score"), 2.0 / 61.0);
    for label in ["badstart", "badend"] {
        let hit: Vec<_> = rows.iter().filter(|row| row["q"] == label).collect();
        assert!(hit.is_empty(), "{label} returned {hit:?}");
    }
    let root: Vec<_> = rows.iter().filter(|row| row["q"] == "root").collect();
    assert!(root.iter().all(|row| row["chunk"] != body));

    let dopen: Vec<_> = rows.iter().filter(|row| row["q"] == "dopen").collect();
    assert_eq!(dopen.len(), FUSION_DEPTH);
    assert!(dopen.iter().all(|row| row["chunk"] != body));
    let meta: Vec<_> = rows.iter().filter(|row| row["q"] == "meta").collect();
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0]["chunk"], body);
    assert_score(meta[0]["score"].as_f64().expect("score"), 2.0 / 61.0);
    let other: Vec<_> = rows.iter().filter(|row| row["q"] == "othermeta").collect();
    assert!(other.iter().all(|row| row["chunk"] != body));
}

fn buried_keep(query: &str, kept: &str) -> String {
    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for _ in 0..FUSION_DEPTH {
        script.push_str(&format!(
            "INSERT INTO docs(text, source) VALUES ('{query}', 'other');\n"
        ));
    }
    script.push_str(&format!(
        "INSERT INTO docs(text, source) VALUES ('{kept}', 'keep');\n"
    ));
    script
}

#[test]
fn tc_281_missing_source_empty_match_and_plain_scan() {
    let query = "filtertoken epsilon";
    let kept = "filtertoken epsilon sits inside a longer paragraph";
    let mut script = buried_keep(query, kept);
    script.push_str(&format!(
        "SELECT 'miss' AS q, source FROM docs WHERE docs MATCH '{query}' AND source = 'missing';
         SELECT 'empty' AS q, source FROM docs WHERE docs MATCH '' AND source = 'keep';
         SELECT 'scan' AS q, source, text FROM docs WHERE source = 'keep';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert!(rows.iter().all(|row| row["q"] != "miss"));
    assert!(rows.iter().all(|row| row["q"] != "empty"));
    let scan: Vec<_> = rows.iter().filter(|row| row["q"] == "scan").collect();
    assert_eq!(scan.len(), 1);
    assert_eq!(scan[0]["source"], "keep");
    assert_eq!(scan[0]["text"], kept);
}

#[test]
fn tc_277_score_comparison_does_not_reach_outside_the_depth() {
    let query = "filtertoken zeta";
    let kept = "filtertoken zeta sits inside a longer paragraph";
    let mut script = buried_keep(query, kept);
    script.push_str(&format!(
        "SELECT source FROM docs WHERE docs MATCH '{query}' AND score > 0;"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row["source"] != "keep"));
}

#[test]
fn tc_278_limit_offset_and_order_by_do_not_change_scores() {
    let query = "filtertoken eta";
    let kept = "filtertoken eta sits inside a longer paragraph";
    let mut script = buried_keep(query, kept);
    script.push_str(&format!(
        "SELECT 'all' AS q, doc_id, source, score FROM docs WHERE docs MATCH '{query}';
         SELECT 'one' AS q, doc_id, score FROM docs WHERE docs MATCH '{query}' LIMIT 1;
         SELECT 'two' AS q, doc_id, score FROM docs WHERE docs MATCH '{query}' LIMIT 1 OFFSET 1;
         SELECT 'ord' AS q, doc_id, source FROM docs WHERE docs MATCH '{query}' ORDER BY source;"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    let all: Vec<_> = rows.iter().filter(|row| row["q"] == "all").collect();
    assert_eq!(all.len(), FUSION_DEPTH);
    assert!(all.iter().all(|row| row["source"] != "keep"));
    let one: Vec<_> = rows.iter().filter(|row| row["q"] == "one").collect();
    let two: Vec<_> = rows.iter().filter(|row| row["q"] == "two").collect();
    assert_eq!(one.len(), 1);
    assert_eq!(two.len(), 1);
    assert_eq!(one[0]["doc_id"], all[0]["doc_id"]);
    assert_score(
        one[0]["score"].as_f64().expect("limit score"),
        all[0]["score"].as_f64().expect("full score"),
    );
    assert_eq!(two[0]["doc_id"], all[1]["doc_id"]);
    assert_score(
        two[0]["score"].as_f64().expect("offset score"),
        all[1]["score"].as_f64().expect("full score"),
    );
    let ordered: Vec<_> = rows.iter().filter(|row| row["q"] == "ord").collect();
    let mut ordered_ids: Vec<_> = ordered
        .iter()
        .map(|row| row["doc_id"].as_i64().unwrap())
        .collect();
    let mut all_ids: Vec<_> = all
        .iter()
        .map(|row| row["doc_id"].as_i64().unwrap())
        .collect();
    ordered_ids.sort();
    all_ids.sort();
    assert_eq!(ordered_ids, all_ids);
}

fn pages_for(query: &str, kept: &str) -> String {
    let mut script = buried_keep(query, kept);
    script.push_str(
        "CREATE TABLE pages(path TEXT, project TEXT);
         INSERT INTO pages(path, project) VALUES ('keep', 'wiki'), ('x1', 'a'), ('x2', 'b');\n",
    );
    script
}

fn assert_keep_at_alone_score(rows: &[serde_json::Value], label: &str) {
    let hit: Vec<_> = rows.iter().filter(|row| row["q"] == label).collect();
    let kept: Vec<_> = hit.iter().filter(|row| row["source"] == "keep").collect();
    assert_eq!(kept.len(), 1, "{label}: {hit:?}");
    assert_score(kept[0]["score"].as_f64().expect("score"), 2.0 / 61.0);
}

#[test]
fn tc_279_a_join_key_narrows_the_search() {
    let query = "filtertoken theta";
    let kept = "filtertoken theta sits inside a longer paragraph";
    let mut script = pages_for(query, kept);
    script.push_str(&format!(
        "SELECT 'inner' AS q, docs.source AS source, docs.score AS score
           FROM docs JOIN pages ON pages.path = docs.source
           WHERE pages.project = 'wiki' AND docs MATCH '{query}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert_keep_at_alone_score(rows, "inner");
}

#[test]
fn tc_276_join_order_does_not_change_a_join_key() {
    let query = "filtertoken kappa";
    let kept = "filtertoken kappa sits inside a longer paragraph";
    let setup = pages_for(query, kept);
    let mut script = setup.clone();
    script.push_str(&format!(
        "SELECT 'cross' AS q, docs.source AS source, docs.score AS score
           FROM pages CROSS JOIN docs ON docs.source = pages.path
           WHERE pages.project = 'wiki' AND docs MATCH '{query}';
         SELECT 'left' AS q, docs.source AS source, docs.score AS score
           FROM pages LEFT JOIN docs
             ON docs.source = pages.path AND docs MATCH '{query}';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert_keep_at_alone_score(rows, "cross");
    assert_keep_at_alone_score(rows, "left");

    let plan = plan_text(&format!(
        "{setup}
         EXPLAIN QUERY PLAN
         SELECT docs.source FROM docs JOIN pages ON pages.path = docs.source
           WHERE pages.project = 'wiki' AND docs MATCH '{query}';"
    ));
    assert!(
        plan.contains("INDEX 1:2"),
        "join key stays on the search plan: {plan}"
    );

    let indexed = plan_text(&format!(
        "{setup}
         CREATE INDEX pages_project ON pages(project);
         EXPLAIN QUERY PLAN
         SELECT docs.source FROM docs JOIN pages ON pages.path = docs.source
           WHERE pages.project = 'wiki' AND docs MATCH '{query}';"
    ));
    assert!(
        indexed.contains("INDEX 1:2"),
        "an index on the other table still pushes the join key: {indexed}"
    );
}

#[test]
fn tc_282_a_predicate_only_on_the_other_table_does_not_narrow() {
    let query = "filtertoken lambda";
    let kept = "filtertoken lambda sits inside a longer paragraph";
    let mut script = pages_for(query, kept);
    script.push_str(&format!(
        "SELECT 'on' AS q, docs.source AS source FROM docs
           JOIN pages ON pages.project = 'wiki'
           WHERE docs MATCH '{query}';
         SELECT 'where' AS q, docs.source AS source FROM docs, pages
           WHERE docs MATCH '{query}' AND pages.project = 'wiki';"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert!(rows.iter().all(|row| row["source"] != "keep"));
    assert!(!rows.is_empty());
}

#[test]
fn tc_280_like_and_inequalities_do_not_reach_outside_the_depth() {
    let query = "filtertoken iota";
    let kept = "filtertoken iota sits inside a longer paragraph";
    let mut script = buried_keep(query, kept);
    script.push_str(&format!(
        "SELECT 'like' AS q, source FROM docs WHERE docs MATCH '{query}' AND source LIKE 'keep%';
         SELECT 'ne' AS q, source FROM docs WHERE docs MATCH '{query}' AND source != 'other';
         SELECT 'gt' AS q, source FROM docs WHERE docs MATCH '{query}' AND rowid > 0;"
    ));
    let rows = ok(&script);
    let rows = rows.as_array().unwrap();
    assert!(rows.iter().all(|row| row["source"] != "keep"));
    let above: Vec<_> = rows.iter().filter(|row| row["q"] == "gt").collect();
    assert_eq!(above.len(), FUSION_DEPTH);
}
