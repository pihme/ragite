//! sqlite-vec names registered when the extension loads (TC-283).

mod common;

use common::{cell, err, ok};

const FUNCTIONS: &[&str] = &[
    "vec_add",
    "vec_bit",
    "vec_debug",
    "vec_distance_cosine",
    "vec_distance_hamming",
    "vec_distance_l1",
    "vec_distance_l2",
    "vec_f32",
    "vec_int8",
    "vec_length",
    "vec_normalize",
    "vec_quantize_binary",
    "vec_quantize_int8",
    "vec_slice",
    "vec_sub",
    "vec_to_json",
    "vec_type",
    "vec_version",
];

const MODULES: &[&str] = &["vec0", "vec_each"];

fn column(rows: &serde_json::Value, key: &str) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, _)| cell(rows, index, key))
        .collect()
}

#[test]
fn tc_283_load_registers_sqlite_vec_0_1_9() {
    let version = ok("SELECT vec_version() AS version;");
    assert_eq!(cell(&version, 0, "version"), "v0.1.9");

    let functions =
        ok("SELECT name FROM pragma_function_list WHERE name GLOB 'vec_*' ORDER BY name;");
    assert_eq!(column(&functions, "name"), FUNCTIONS);

    let modules = ok("SELECT name FROM pragma_module_list WHERE name GLOB 'vec*' ORDER BY name;");
    assert_eq!(column(&modules, "name"), MODULES);

    for name in ["vec_npy_file", "vec_static_blob_from_raw"] {
        let message = err(&format!("SELECT {name}(1);"));
        assert!(message.contains("no such function"), "{name}: {message}");
    }
    for name in [
        "vec_npy_each",
        "vec_static_blobs",
        "vec_static_blob_entries",
    ] {
        let message = err(&format!("CREATE VIRTUAL TABLE t USING {name};"));
        assert!(message.contains("no such module"), "{name}: {message}");
    }
}
