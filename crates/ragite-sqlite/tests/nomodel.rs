//! Extension built with `--no-default-features` and no provider model.
//! `make test` builds that extension and runs the ignored test against it.

#[allow(dead_code)]
mod common;

use common::{extension_path, sql};

// Neither the built-in (512) nor the fixture (256) config may be embedded.
const CONFIG_MARKER: &[u8] = b"\"hidden_dim\": 512";
const FIXTURE_MARKER: &[u8] = b"\"hidden_dim\": 256";

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
    assert!(
        !bytes
            .windows(FIXTURE_MARKER.len())
            .any(|window| window == FIXTURE_MARKER),
        "config.json from potion-base-8M is embedded"
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
