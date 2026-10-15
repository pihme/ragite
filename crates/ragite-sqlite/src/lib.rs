//! Thin SQLite loadable extension. Chunking and embedding live in `ragite-core`.

mod func;
mod sql;
mod vtab;

use std::ffi::{c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};

use libsqlite3_sys::{
    sqlite3, sqlite3_api_routines, sqlite3_create_module_v2, SQLITE_ERROR, SQLITE_OK,
};

unsafe extern "C" {
    fn sqlite3_vec_init(
        db: *mut sqlite3,
        pz_err: *mut *mut c_char,
        p_api: *const sqlite3_api_routines,
    ) -> c_int;
}

/// Entry point for `SELECT load_extension('ragite')`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlite3_ragite_init(
    db: *mut sqlite3,
    pz_err: *mut *mut c_char,
    p_api: *mut sqlite3_api_routines,
) -> c_int {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe { init(db, pz_err, p_api) }));
    match result {
        Ok(rc) => rc,
        Err(_) => {
            set_pz(pz_err, "ragite failed while loading");
            SQLITE_ERROR
        }
    }
}

unsafe fn init(
    db: *mut sqlite3,
    pz_err: *mut *mut c_char,
    p_api: *mut sqlite3_api_routines,
) -> c_int {
    if let Err(err) = unsafe { libsqlite3_sys::rusqlite_extension_init2(p_api) } {
        set_pz(pz_err, &format!("ragite could not bind SQLite: {err}"));
        return SQLITE_ERROR;
    }
    let rc = unsafe { sqlite3_vec_init(db, pz_err, p_api.cast()) };
    if rc != SQLITE_OK {
        return rc;
    }
    let module = vtab::module();
    let rc = unsafe {
        sqlite3_create_module_v2(
            db,
            c"ragite".as_ptr(),
            module,
            std::ptr::null_mut(),
            None,
        )
    };
    if rc != SQLITE_OK {
        set_pz(pz_err, "ragite could not register its module");
        return rc;
    }
    if let Err(err) = func::register(db) {
        set_pz(pz_err, &err);
        return SQLITE_ERROR;
    }
    SQLITE_OK
}

fn set_pz(pz_err: *mut *mut c_char, msg: &str) {
    if pz_err.is_null() {
        return;
    }
    unsafe {
        *pz_err = sql::alloc_text(msg);
    }
}
