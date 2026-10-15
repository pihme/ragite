//! Parity smoke test (REQ-91): the built-in Model, embedded through
//! ragite's own path, matches the upstream Python reference vectors.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use common::{cell, ok};
use serde_json::Value;

fn decode_f32_le(hex_text: &str) -> Vec<f32> {
    let bytes: Vec<u8> = (0..hex_text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_text[i..i + 2], 16).unwrap())
        .collect();
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn cosine(a: &[f32], b: &[f64]) -> f64 {
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b) {
        let x = f64::from(*x);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[test]
fn tc_246_builtin_model_matches_reference_vectors() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity/reference.json");
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("reference.json")).unwrap();
    let min = reference["min_cosine"].as_f64().unwrap();
    let items = reference["items"].as_array().unwrap();
    assert!(items.len() >= 5, "need a handful of reference sentences");

    let rows = ok("SELECT technical_id FROM ragite_default_model();");
    assert_eq!(
        cell(&rows, 0, "technical_id"),
        reference["model"].as_str().unwrap(),
        "reference.json was generated for another Model; rerun tests/parity/generate_reference.py"
    );

    let selects: Vec<String> = items
        .iter()
        .map(|item| {
            let text = item["text"].as_str().unwrap().replace('\'', "''");
            format!(
                "SELECT hex(ragite_embed_model((SELECT technical_id FROM ragite_default_model()), '{text}', 'passage')) AS v;"
            )
        })
        .collect();
    let rows = ok(&selects.join("\n"));

    let mut failures = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let got = decode_f32_le(&cell(&rows, i, "v"));
        let want: Vec<f64> = item["vector"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(got.len(), want.len(), "dimension for {:?}", item["text"]);
        let c = cosine(&got, &want);
        if !(c >= min) {
            failures.push(format!("{:?}: cosine {c:.5}", item["text"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "below {min}: {failures:#?}");
}
