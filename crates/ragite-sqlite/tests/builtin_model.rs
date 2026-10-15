//! Default build: the extension embeds potion-base-8M.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use common::extension_path;

const CONFIG_MARKER: &[u8] = b"baai/bge-base-en-v1.5";

#[test]
fn tc_6_tc_52_default_build_embeds_model2vec_weights() {
    let bytes = std::fs::read(extension_path()).expect("extension");
    assert!(
        bytes
            .windows(CONFIG_MARKER.len())
            .any(|window| window == CONFIG_MARKER),
        "config.json is not embedded in the extension"
    );

    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ragite-core/src/model.rs"),
    )
    .unwrap();
    assert!(src.contains("StaticModel::from_bytes"));
    assert!(src.contains("include_bytes!"));
    assert!(!src.contains("from_pretrained"));

    let manifest = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ragite-core/Cargo.toml"),
    )
    .unwrap();
    assert!(manifest.contains("default = [\"builtin-model\"]"));
}
