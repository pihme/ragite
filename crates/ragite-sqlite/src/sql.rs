//! Small helpers over the SQLite function pointers. The host owns the database.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;

use libsqlite3_sys::{
    SQLITE_DONE, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TRANSIENT, sqlite3, sqlite3_bind_blob,
    sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text, sqlite3_bind_value,
    sqlite3_column_bytes, sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text,
    sqlite3_column_type, sqlite3_db_handle, sqlite3_errmsg, sqlite3_finalize,
    sqlite3_last_insert_rowid, sqlite3_malloc, sqlite3_prepare_v2, sqlite3_step, sqlite3_stmt,
    sqlite3_value,
};

pub fn alloc_text(msg: &str) -> *mut c_char {
    let bytes = msg.as_bytes();
    unsafe {
        let raw = sqlite3_malloc((bytes.len() + 1) as c_int);
        if raw.is_null() {
            return ptr::null_mut();
        }
        let dest = raw.cast::<u8>();
        ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
        *dest.add(bytes.len()) = 0;
        raw.cast()
    }
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn qualified(schema: &str, name: &str) -> String {
    format!("{}.{}", quote_ident(schema), quote_ident(name))
}

pub fn errmsg(db: *mut sqlite3) -> String {
    unsafe {
        let ptr = sqlite3_errmsg(db);
        if ptr.is_null() {
            return "sqlite error".to_string();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

pub struct Stmt {
    raw: *mut sqlite3_stmt,
}

impl Drop for Stmt {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.raw);
        }
    }
}

impl Stmt {
    pub fn prepare(db: *mut sqlite3, sql: &str) -> Result<Self, String> {
        let c_sql = CString::new(sql).map_err(|_| "sql contains a nul".to_string())?;
        let mut raw = ptr::null_mut();
        let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut raw, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(errmsg(db));
        }
        Ok(Self { raw })
    }

    pub fn bind_null(&self, index: c_int) -> Result<(), String> {
        self.check(unsafe { sqlite3_bind_null(self.raw, index) })
    }

    pub fn bind_i64(&self, index: c_int, value: i64) -> Result<(), String> {
        self.check(unsafe { sqlite3_bind_int64(self.raw, index, value) })
    }

    pub fn bind_text(&self, index: c_int, value: &str) -> Result<(), String> {
        let rc = unsafe {
            sqlite3_bind_text(
                self.raw,
                index,
                value.as_ptr().cast(),
                value.len() as c_int,
                SQLITE_TRANSIENT(),
            )
        };
        self.check(rc)
    }

    pub fn bind_value(&self, index: c_int, value: *mut sqlite3_value) -> Result<(), String> {
        self.check(unsafe { sqlite3_bind_value(self.raw, index, value) })
    }

    pub fn bind_blob(&self, index: c_int, value: &[u8]) -> Result<(), String> {
        let rc = unsafe {
            sqlite3_bind_blob(
                self.raw,
                index,
                value.as_ptr().cast::<c_void>(),
                value.len() as c_int,
                SQLITE_TRANSIENT(),
            )
        };
        self.check(rc)
    }

    pub fn step(&self) -> Result<bool, String> {
        let rc = unsafe { sqlite3_step(self.raw) };
        if rc == SQLITE_ROW {
            Ok(true)
        } else if rc == SQLITE_DONE {
            Ok(false)
        } else {
            Err(errmsg(unsafe { sqlite3_db_handle(self.raw) }))
        }
    }

    pub fn column_i64(&self, index: c_int) -> i64 {
        unsafe { sqlite3_column_int64(self.raw, index) }
    }

    /// Real column, or `None` when the value is NULL.
    /// `sqlite3_column_double` returns 0.0 for NULL, which is not a distance.
    pub fn column_double(&self, index: c_int) -> Option<f64> {
        unsafe {
            if sqlite3_column_type(self.raw, index) == SQLITE_NULL {
                return None;
            }
            Some(sqlite3_column_double(self.raw, index))
        }
    }

    pub fn column_text(&self, index: c_int) -> Option<String> {
        unsafe {
            if sqlite3_column_type(self.raw, index) == SQLITE_NULL {
                return None;
            }
            let ptr = sqlite3_column_text(self.raw, index);
            if ptr.is_null() {
                return None;
            }
            let n = sqlite3_column_bytes(self.raw, index) as usize;
            let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), n);
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
    }

    fn check(&self, rc: c_int) -> Result<(), String> {
        if rc == SQLITE_OK {
            Ok(())
        } else {
            Err(errmsg(unsafe { sqlite3_db_handle(self.raw) }))
        }
    }
}

pub fn exec(db: *mut sqlite3, sql: &str) -> Result<(), String> {
    let stmt = Stmt::prepare(db, sql)?;
    while stmt.step()? {}
    Ok(())
}

pub fn last_insert_rowid(db: *mut sqlite3) -> i64 {
    unsafe { sqlite3_last_insert_rowid(db) }
}

pub fn exists(db: *mut sqlite3, schema: &str, name: &str) -> Result<bool, String> {
    let sql = format!(
        "SELECT 1 FROM {}.sqlite_master WHERE name = ?1 AND type IN ('table', 'view')",
        quote_ident(schema)
    );
    let stmt = Stmt::prepare(db, &sql)?;
    stmt.bind_text(1, name)?;
    stmt.step()
}
