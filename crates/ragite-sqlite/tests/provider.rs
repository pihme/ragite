//! Extension built with `--no-default-features` and a provider model.
//! `make test` passes the provider variables and runs the ignored test.

#[allow(dead_code)]
mod common;

use common::{cell, extension_path, ok, sql};

const CONFIG_MARKER: &[u8] = b"baai/bge-base-en-v1.5";
const BUILTIN_ID: &[u8] = b"potion-base-8M@bf8b056651a2c21b8d2565580b8569da283cab23";

#[test]
#[ignore = "needs the provider-model extension from make test"]
fn tc_54_tc_55_tc_126_provider_catalog_and_prefixes() {
    let bytes = std::fs::read(extension_path()).expect("extension");
    assert!(
        bytes
            .windows(CONFIG_MARKER.len())
            .any(|window| window == CONFIG_MARKER),
        "provider weights were not compiled in"
    );
    assert!(
        !bytes
            .windows(BUILTIN_ID.len())
            .any(|window| window == BUILTIN_ID),
        "built-in technical id is still in the extension"
    );

    let rows = ok("SELECT name, technical_id FROM ragite_list_models();
         SELECT name, technical_id FROM ragite_default_model();
         SELECT hex(ragite_embed_model('provider-demo@rev-supplied', 'hello', 'query')) AS q,
                hex(ragite_embed_model('provider-demo@rev-supplied', 'hello', 'passage')) AS p;");
    assert_eq!(cell(&rows, 0, "name"), "provider-demo");
    assert_eq!(cell(&rows, 0, "technical_id"), "provider-demo@rev-supplied");
    assert_eq!(cell(&rows, 1, "name"), "provider-demo");
    assert_eq!(cell(&rows, 1, "technical_id"), "provider-demo@rev-supplied");
    let revision = cell(&rows, 0, "technical_id")
        .split_once('@')
        .unwrap()
        .1
        .to_string();
    assert_eq!(revision, "rev-supplied");
    assert_ne!(revision.len(), 40);
    assert_ne!(cell(&rows, 2, "q"), cell(&rows, 2, "p"));
    assert!(!cell(&rows, 2, "q").is_empty());

    let rejected = sql("CREATE VIRTUAL TABLE docs USING ragite(revision='rev-supplied');");
    assert!(!rejected.status, "{}", rejected.stdout);
    assert!(
        rejected
            .stderr
            .contains("argument is not accepted: revision"),
        "{}",
        rejected.stderr
    );
}
