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
    // "unrelated zebra" is farther than the maximum and is not a full-text hit.
    assert_eq!(docs.len(), 3);
    assert_eq!(cell(&rows, 0, "doc_id"), "2");
    assert_eq!(cell(&rows, 0, "source"), "manual");
    assert_score(score(&rows, 0), 2.0 / 61.0);

    assert_eq!(cell(&rows, 1, "chunk"), "alpha unique");
    assert_score(score(&rows, 1), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    assert_eq!(cell(&rows, 2, "chunk"), "beta other");
    assert_eq!(cell(&rows, 2, "doc_id"), "2");
    assert_score(score(&rows, 2), 1.0 / 62.0);
}

#[test]
fn tc_67_tc_68_fts_folding() {
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
    // café is farther than the maximum from "install", and Install is farther than
    // the maximum from "cafe". Each query keeps the full-text hit only.
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(cell(&rows, 0, "doc_id"), "1");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    assert_eq!(cell(&rows, 1, "doc_id"), "2");
    assert_score(score(&rows, 1), 2.0 / 61.0);
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
         INSERT INTO docs(text) VALUES ('target');
         INSERT INTO docs(text) VALUES ('phrase');
         INSERT INTO docs(text) VALUES ('target phrase');
         SELECT doc_id, score FROM docs WHERE docs MATCH 'target phrase';
         SELECT doc_id, score FROM docs WHERE docs MATCH 'target phrase' LIMIT 2;
         SELECT doc_id, score FROM docs_chunks WHERE docs_chunks MATCH 'target phrase' LIMIT 1;");
    assert_eq!(cell(&rows, 0, "doc_id"), "3");
    assert_score(score(&rows, 0), 2.0 / 61.0);
    let mut rest = vec![cell(&rows, 1, "doc_id"), cell(&rows, 2, "doc_id")];
    rest.sort();
    assert_eq!(rest, vec!["1".to_string(), "2".to_string()]);
    assert_eq!(rows.as_array().unwrap().len(), 6);
    assert_eq!(cell(&rows, 3, "doc_id"), "3");
    assert_score(score(&rows, 3), 2.0 / 61.0);
    let second = cell(&rows, 4, "doc_id");
    let second_score = score(&rows, 4);
    let unlimited = (1..=2)
        .find(|index| cell(&rows, *index, "doc_id") == second)
        .expect("LIMIT row is one of the fused rows");
    assert_score(second_score, score(&rows, unlimited));
    assert_eq!(cell(&rows, 5, "doc_id"), "3");
    assert_score(score(&rows, 5), 2.0 / 61.0);
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
}

/// `M` and `D` from SPEC section 8.1. This test does not hard-code a guessed distance.
fn spec_section_81() -> &'static str {
    let spec = include_str!("../../../SPEC.md");
    let start = spec
        .find("### 8.1 Version 1 constants")
        .expect("SPEC section 8.1");
    let rest = &spec[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

fn spec_leading_number(label: &str) -> f64 {
    let section = spec_section_81();
    let marker = format!("**{label}.** ");
    let at = section
        .find(&marker)
        .unwrap_or_else(|| panic!("section 8.1 has no {label}"));
    let token = section[at + marker.len()..]
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("section 8.1 {label} has no number"));
    token
        .parse()
        .unwrap_or_else(|_| panic!("section 8.1 {label} is not a number: {token}"))
}

fn sql_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn json_i64(row: &serde_json::Value, key: &str) -> i64 {
    row[key]
        .as_i64()
        .unwrap_or_else(|| panic!("{key} in {row}"))
}

fn json_f64(row: &serde_json::Value, key: &str) -> f64 {
    row[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} in {row}"))
}

fn rrf(vector: &[i64], full_text: &[i64]) -> std::collections::HashMap<i64, f64> {
    let mut scores = std::collections::HashMap::new();
    for list in [vector, full_text] {
        let mut seen = std::collections::HashSet::new();
        for (index, id) in list.iter().enumerate() {
            if !seen.insert(*id) {
                continue;
            }
            *scores.entry(*id).or_insert(0.0) += 1.0 / (60.0 + index as f64 + 1.0);
        }
    }
    scores
}

#[test]
fn tc_193_tc_286_vector_neighbors_past_the_maximum_are_dropped() {
    let max_distance = spec_leading_number("Vector distance");
    let depth = spec_leading_number("Fusion depth") as usize;
    assert_eq!(
        max_distance.to_bits(),
        ragite_core::MAX_COSINE_DISTANCE.to_bits()
    );
    assert_eq!(depth, ragite_core::FUSION_DEPTH);

    let query = "purple quantum teapot";
    let filler = format!("zzzzzzzz purple quantum teapot {}", "zebra ".repeat(40));
    let zeros = "A".repeat(400);
    let dots = ".".repeat(80);
    let texts = [
        query,
        "a purple quantum teapot sits on the table",
        "teapot purple",
        "purple quantum kettle",
        filler.as_str(),
        "the mitochondria is the powerhouse of the cell",
        "unrelated zebra quilt factory manual",
        dots.as_str(),
        zeros.as_str(),
    ];
    let quoted = sql_quote(query);
    // The same AND of quoted tokens that `fts_and` binds. This ASCII query's
    // unicode61 tokens are its words. A phrase would miss a different order.
    let fts_query = sql_quote(
        &query
            .split_whitespace()
            .map(|token| format!("\"{token}\""))
            .collect::<Vec<_>>()
            .join(" AND "),
    );
    let mut script = String::from("CREATE VIRTUAL TABLE docs USING ragite;\n");
    for text in texts {
        script.push_str(&format!(
            "INSERT INTO docs(text) VALUES ({});\n",
            sql_quote(text)
        ));
    }
    script.push_str(&format!(
        "SELECT 'dist' AS kind, c.rowid AS id, c.chunk AS chunk,
                vec_distance_cosine(v.embedding, ragite_embed('docs', {quoted}, 'query')) AS distance
         FROM docs_chunks c JOIN docs_vec v ON v.rowid = c.rowid;
         SELECT 'fts' AS kind, rowid AS id FROM docs_fts
         WHERE docs_fts MATCH {fts_query} ORDER BY bm25(docs_fts);
         SELECT 'hit' AS kind, rowid AS id, score FROM docs_chunks
         WHERE docs_chunks MATCH {quoted};
         SELECT 'again' AS kind, rowid AS id, score FROM docs_chunks
         WHERE docs_chunks MATCH {quoted};"
    ));
    let rows = ok(&script);
    let all = rows.as_array().unwrap();
    let of_kind = |name: &str| {
        all.iter()
            .filter(|row| row["kind"].as_str() == Some(name))
            .collect::<Vec<_>>()
    };

    struct Stored {
        id: i64,
        chunk: String,
        distance: Option<f64>,
    }
    let stored: Vec<Stored> = of_kind("dist")
        .into_iter()
        .map(|row| Stored {
            id: json_i64(row, "id"),
            chunk: row["chunk"].as_str().expect("chunk").to_string(),
            distance: if row["distance"].is_null() {
                None
            } else {
                Some(json_f64(row, "distance"))
            },
        })
        .collect();
    let fts: Vec<i64> = of_kind("fts")
        .into_iter()
        .map(|row| json_i64(row, "id"))
        .collect();
    let hits: Vec<(i64, f64)> = of_kind("hit")
        .into_iter()
        .map(|row| (json_i64(row, "id"), json_f64(row, "score")))
        .collect();
    let again: Vec<(i64, f64)> = of_kind("again")
        .into_iter()
        .map(|row| (json_i64(row, "id"), json_f64(row, "score")))
        .collect();
    assert_eq!(hits, again);

    let mut kept: Vec<(i64, f64)> = stored
        .iter()
        .filter_map(|chunk| {
            chunk
                .distance
                .filter(|distance| *distance <= max_distance)
                .map(|distance| (chunk.id, distance))
        })
        .collect();
    kept.sort_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap()
            .then(left.0.cmp(&right.0))
    });
    let vector: Vec<i64> = kept.into_iter().take(depth).map(|(id, _)| id).collect();
    let full_text: Vec<i64> = fts.iter().copied().take(depth).collect();
    let expected = rrf(&vector, &full_text);
    assert_eq!(hits.len(), expected.len(), "{hits:?} vs {expected:?}");
    for (id, score) in &hits {
        let want = expected[id];
        assert!(
            (score - want).abs() < 1e-9,
            "chunk {id} score {score} != {want}"
        );
    }
    assert!(
        (expected[vector.first().expect("a neighbor within M")]
            - hits
                .iter()
                .find(|(id, _)| *id == *vector.first().unwrap())
                .unwrap()
                .1)
            .abs()
            < 1e-9
    );
    assert!(expected[&vector[0]] + 1e-12 >= 1.0 / 61.0);

    let fts_set: std::collections::HashSet<i64> = fts.iter().copied().collect();
    let hit_set: std::collections::HashSet<i64> = hits.iter().map(|(id, _)| *id).collect();
    let exact = stored.iter().find(|chunk| chunk.chunk == query).unwrap();
    assert_eq!(exact.distance, Some(0.0));
    assert!(hit_set.contains(&exact.id));
    assert_eq!(vector.first().copied(), Some(exact.id));

    for chunk in &stored {
        let over = match chunk.distance {
            Some(distance) => distance > max_distance,
            None => true,
        };
        if hit_set.contains(&chunk.id) && !fts_set.contains(&chunk.id) {
            let distance = chunk.distance.expect("vector-only hit");
            assert!(distance <= max_distance, "{distance} > {max_distance}");
        }
        if over {
            assert_eq!(
                hit_set.contains(&chunk.id),
                fts_set.contains(&chunk.id),
                "chunk {} distance {:?}",
                chunk.id,
                chunk.distance
            );
        }
    }
    assert!(stored.iter().any(|chunk| {
        chunk
            .distance
            .is_some_and(|distance| distance > max_distance)
            && fts_set.contains(&chunk.id)
            && hit_set.contains(&chunk.id)
    }));
    assert!(stored.iter().any(|chunk| {
        chunk
            .distance
            .is_some_and(|distance| distance > max_distance)
            && !fts_set.contains(&chunk.id)
            && !hit_set.contains(&chunk.id)
    }));
    assert!(stored.iter().any(|chunk| {
        chunk
            .distance
            .is_some_and(|distance| distance <= max_distance && distance > 0.0)
            && !fts_set.contains(&chunk.id)
            && hit_set.contains(&chunk.id)
    }));

    let zero = stored.iter().find(|chunk| chunk.chunk == zeros).unwrap();
    assert!(zero.distance.is_none(), "{:?}", zero.distance);
    assert!(!hit_set.contains(&zero.id));

    let mut vector_only: Vec<(f64, f64)> = stored
        .iter()
        .filter(|chunk| hit_set.contains(&chunk.id) && !fts_set.contains(&chunk.id))
        .map(|chunk| {
            let score = hits.iter().find(|(id, _)| *id == chunk.id).unwrap().1;
            (chunk.distance.unwrap(), score)
        })
        .collect();
    vector_only.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap());
    assert!(vector_only.len() >= 2, "{vector_only:?}");
    for pair in vector_only.windows(2) {
        assert!(
            pair[0].1 + 1e-12 >= pair[1].1,
            "closer {:?} should outrank {:?}",
            pair[0],
            pair[1]
        );
    }

    let far_texts: Vec<&str> = stored
        .iter()
        .filter(|chunk| {
            let over = match chunk.distance {
                Some(distance) => distance > max_distance,
                None => true,
            };
            over && !fts_set.contains(&chunk.id)
        })
        .map(|chunk| chunk.chunk.as_str())
        .collect();
    assert!(!far_texts.is_empty());
    let mut far_script = String::from("CREATE VIRTUAL TABLE far USING ragite;\n");
    for text in &far_texts {
        far_script.push_str(&format!(
            "INSERT INTO far(text) VALUES ({});\n",
            sql_quote(text)
        ));
    }
    far_script.push_str(&format!(
        "SELECT 'dist' AS kind, c.rowid AS id,
                vec_distance_cosine(v.embedding, ragite_embed('far', {quoted}, 'query')) AS distance
         FROM far_chunks c JOIN far_vec v ON v.rowid = c.rowid;
         SELECT 'fts' AS kind, count(*) AS n FROM far_fts WHERE far_fts MATCH {fts_query};
         SELECT 'hit' AS kind, count(*) AS n FROM far_chunks WHERE far_chunks MATCH {quoted};
         SELECT 'doc' AS kind, count(*) AS n FROM far WHERE far MATCH {quoted};"
    ));
    let far_rows = ok(&far_script);
    let far_all = far_rows.as_array().unwrap();
    let far_kind = |name: &str| {
        far_all
            .iter()
            .filter(|row| row["kind"].as_str() == Some(name))
            .collect::<Vec<_>>()
    };
    assert!(!far_kind("dist").is_empty());
    for row in far_kind("dist") {
        if row["distance"].is_null() {
            continue;
        }
        let distance = json_f64(row, "distance");
        assert!(distance > max_distance, "{distance} <= {max_distance}");
    }
    assert_eq!(json_i64(far_kind("fts")[0], "n"), 0);
    assert_eq!(json_i64(far_kind("hit")[0], "n"), 0);
    assert_eq!(json_i64(far_kind("doc")[0], "n"), 0);

    let message = err(&format!(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ({quoted});
         SELECT score FROM docs WHERE docs MATCH {quoted} AND distance = 0.1;"
    ));
    assert!(message.contains("no such column"), "{message}");
}

#[test]
fn tc_210_tc_211_tc_212_match_text_is_not_fts_syntax() {
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
fn tc_285_every_token_any_order() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('cats AND dogs');
         CREATE TABLE keeper(x INTEGER PRIMARY KEY);
         INSERT INTO keeper(x) VALUES (7);
         SELECT last_insert_rowid() AS id;
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'dogs AND cats';
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'cats dogs';
         SELECT chunk, score FROM docs_chunks WHERE docs_chunks MATCH 'cats dogs birds';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '***';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '\"';
         SELECT last_insert_rowid() AS id;");
    assert_eq!(cell(&rows, 0, "id"), "7");
    assert_eq!(cell(&rows, 1, "chunk"), "cats AND dogs");
    assert_score(score(&rows, 1), 2.0 / 61.0);
    assert_eq!(cell(&rows, 2, "chunk"), "cats AND dogs");
    assert_score(score(&rows, 2), 2.0 / 61.0);
    assert_eq!(cell(&rows, 3, "chunk"), "cats AND dogs");
    assert_score(score(&rows, 3), 1.0 / 61.0);
    assert_eq!(cell(&rows, 4, "n"), "0");
    assert_eq!(cell(&rows, 5, "n"), "0");
    assert_eq!(cell(&rows, 6, "id"), "7");
    assert_eq!(rows.as_array().unwrap().len(), 7);
}

#[test]
fn tc_115_tc_116_empty_skips_the_model_and_whitespace_is_embedded() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('hello world');
         SELECT count(*) AS n FROM docs WHERE docs MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH '   ';");
    assert_eq!(cell(&rows, 0, "n"), "0");
    assert_eq!(cell(&rows, 1, "n"), "0");
    assert_eq!(cell(&rows, 2, "n"), "0");

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
    // "unrelated zebra" is farther than the maximum and is not a full-text hit.
    assert_eq!(filtered.as_array().unwrap().len(), 1);
    assert_eq!(cell(&filtered, 0, "source"), "keep");
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
