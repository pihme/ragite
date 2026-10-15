//! Default build: the extension embeds potion-retrieval-32M.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use common::extension_path;

// `hidden_dim` tells the potion-retrieval-32M config (512) from the potion-base-8M
// fixture (256); both name the same tokenizer.
const CONFIG_MARKER: &[u8] = b"\"hidden_dim\": 512";

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
