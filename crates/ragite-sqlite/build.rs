use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();
    let sqlite = root.join("third_party/sqlite");
    let sqlite_vec = root.join("third_party/sqlite-vec");
    cc::Build::new()
        .file(sqlite_vec.join("sqlite-vec.c"))
        .include(&sqlite)
        .include(&sqlite_vec)
        .define("SQLITE_VEC_OMIT_FS", None)
        .flag("-std=c11")
        // sqlite-vec.c redeclares uint8_t from the BSD names in sys/types.h.
        .flag("-include")
        .flag("sys/types.h")
        .warnings(false)
        .compile("sqlite_vec");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rerun-if-changed={}", sqlite_vec.join("sqlite-vec.c").display());
    println!("cargo:rerun-if-changed={}", sqlite_vec.join("sqlite-vec.h").display());
    println!("cargo:rerun-if-changed={}", sqlite.join("sqlite3ext.h").display());
}
