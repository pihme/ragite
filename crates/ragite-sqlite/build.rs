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
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let mut build = cc::Build::new();
    build
        .file(sqlite_vec.join("sqlite-vec.c"))
        .file(manifest.join("src/sqlite_in.c"))
        .include(&sqlite)
        .include(&sqlite_vec)
        .define("SQLITE_VEC_OMIT_FS", None)
        .warnings(false);
    if target_os != "windows" {
        build
            .flag("-std=c11")
            // sqlite-vec.c redeclares uint8_t from the BSD names in sys/types.h.
            .flag("-include")
            .flag("sys/types.h");
    }
    build.compile("sqlite_vec");
    // MSVC provides libm in the C runtime.
    if target_os != "windows" {
        println!("cargo:rustc-link-lib=m");
    }
    println!(
        "cargo:rerun-if-changed={}",
        sqlite_vec.join("sqlite-vec.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest.join("src/sqlite_in.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sqlite_vec.join("sqlite-vec.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        sqlite.join("sqlite3ext.h").display()
    );
}
