//! Extension built with `--no-default-features` and no provider model.
//! `make test` builds that extension and runs the ignored test against it.

#[allow(dead_code)]
mod common;

use common::{extension_path, sql};

const CONFIG_MARKER: &[u8] = b"baai/bge-base-en-v1.5";

#[test]
#[ignore = "needs the --no-default-features extension, without a provider model"]
fn tc_53_no_default_features_omits_the_embedded_file() {
    let bytes = std::fs::read(extension_path()).expect("extension");
    assert!(
        !bytes
            .windows(CONFIG_MARKER.len())
            .any(|window| window == CONFIG_MARKER),
        "config.json from potion-retrieval-32M is still embedded"
    );
    let out = sql("SELECT ragite_embed_model(
            'potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7',
            'hello',
            'passage'
         );");
    assert!(
        !out.status,
        "embed succeeded without embedded weights\n{}",
        out.stdout
    );
    assert!(
        out.stderr.contains("no built-in model") || out.stderr.contains("cannot be loaded"),
        "{}",
        out.stderr
    );
}
