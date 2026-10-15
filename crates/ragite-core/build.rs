//! Compile a provider Model2Vec directory when `builtin-model` is off.
//!
//! `RAGITE_MODEL_PATH` is a directory of `tokenizer.json`, `model.safetensors`,
//! and `config.json`. `RAGITE_MODEL_NAME` and `RAGITE_MODEL_REVISION` are the
//! catalog row. `RAGITE_QUERY_PREFIX` and `RAGITE_PASSAGE_PREFIX` may be empty.
//! With the default feature on, those variables are ignored.

use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=RAGITE_MODEL_PATH");
    println!("cargo:rerun-if-env-changed=RAGITE_MODEL_NAME");
    println!("cargo:rerun-if-env-changed=RAGITE_MODEL_REVISION");
    println!("cargo:rerun-if-env-changed=RAGITE_QUERY_PREFIX");
    println!("cargo:rerun-if-env-changed=RAGITE_PASSAGE_PREFIX");
    println!("cargo:rustc-check-cfg=cfg(ragite_provider_model)");

    let path = std::env::var("RAGITE_MODEL_PATH").unwrap_or_default();
    if path.is_empty() {
        return;
    }
    if std::env::var_os("CARGO_FEATURE_BUILTIN_MODEL").is_some() {
        println!(
            "cargo:warning=RAGITE_MODEL_PATH is ignored because the builtin-model feature is enabled"
        );
        return;
    }

    let dir = PathBuf::from(&path);
    let tokenizer = dir.join("tokenizer.json");
    let weights = dir.join("model.safetensors");
    let config = dir.join("config.json");
    for file in [&tokenizer, &weights, &config] {
        if !file.is_file() {
            panic!(
                "RAGITE_MODEL_PATH {} is missing {}",
                dir.display(),
                file.display()
            );
        }
        println!("cargo:rerun-if-changed={}", file.display());
    }

    let name = token("RAGITE_MODEL_NAME");
    let revision = token("RAGITE_MODEL_REVISION");
    let query = single_line("RAGITE_QUERY_PREFIX");
    let passage = single_line("RAGITE_PASSAGE_PREFIX");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    copy_file(&tokenizer, &out.join("tokenizer.json"));
    copy_file(&weights, &out.join("model.safetensors"));
    copy_file(&config, &out.join("config.json"));

    println!("cargo:rustc-cfg=ragite_provider_model");
    rustc_env("RAGITE_COMPILED_TOKENIZER", &out.join("tokenizer.json"));
    rustc_env("RAGITE_COMPILED_WEIGHTS", &out.join("model.safetensors"));
    rustc_env("RAGITE_COMPILED_CONFIG", &out.join("config.json"));
    println!("cargo:rustc-env=RAGITE_COMPILED_NAME={name}");
    println!("cargo:rustc-env=RAGITE_COMPILED_REVISION={revision}");
    println!("cargo:rustc-env=RAGITE_COMPILED_QUERY_PREFIX={query}");
    println!("cargo:rustc-env=RAGITE_COMPILED_PASSAGE_PREFIX={passage}");
}

fn token(key: &str) -> String {
    let value = std::env::var(key).unwrap_or_default();
    if value.is_empty()
        || value.contains('@')
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_whitespace)
    {
        panic!("{key} must be a single token without '@' when RAGITE_MODEL_PATH is set");
    }
    value
}

fn single_line(key: &str) -> String {
    let value = std::env::var(key).unwrap_or_default();
    if value
        .chars()
        .any(|ch| ch == '\n' || ch == '\r' || ch == '\0')
    {
        panic!("{key} must be a single line");
    }
    value
}

fn copy_file(from: &std::path::Path, to: &std::path::Path) {
    fs::copy(from, to).unwrap_or_else(|err| {
        panic!(
            "could not copy {} to {}: {err}",
            from.display(),
            to.display()
        );
    });
}

fn rustc_env(key: &str, path: &std::path::Path) {
    let text = path
        .to_str()
        .unwrap_or_else(|| panic!("{key} path is not utf-8"));
    println!("cargo:rustc-env={key}={text}");
}
