//! SQL functions for the built-in model. Table-valued functions return the
//! catalog row. Scalar functions exist so a generated column or an index can
//! call the same names: a generated column cannot query a table-valued function.

use std::ffi::{CString, c_char, c_int, c_void};
use std::mem;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::OnceLock;

use libsqlite3_sys::{
    SQLITE_DETERMINISTIC, SQLITE_ERROR, SQLITE_INNOCUOUS, SQLITE_OK, SQLITE_TRANSIENT, SQLITE_UTF8,
    sqlite3, sqlite3_context, sqlite3_context_db_handle, sqlite3_create_function_v2,
    sqlite3_create_module_v2, sqlite3_declare_vtab, sqlite3_module, sqlite3_result_blob,
    sqlite3_result_error, sqlite3_result_text, sqlite3_value, sqlite3_vtab, sqlite3_vtab_cursor,
};
use ragite_core::{MODEL_NAME, TECHNICAL_ID, builtin_model};

use crate::sql::{self, Stmt};
use crate::vtab::{self, accept_model_id, require_stored_model};

const FUNC_FLAGS: c_int = SQLITE_UTF8 | SQLITE_DETERMINISTIC | SQLITE_INNOCUOUS;

pub fn register(db: *mut sqlite3) -> Result<(), String> {
    register_catalog(db)?;
    scalar(db, "ragite_list_models", scalar_technical_id)?;
    scalar(db, "ragite_default_model", scalar_technical_id)?;
    scalar(db, "ragite_table_model", scalar_table_model)?;
    scalar(db, "ragite_embed", scalar_embed)?;
    scalar(db, "ragite_embed_model", scalar_embed_model)?;
    Ok(())
}

fn scalar(
    db: *mut sqlite3,
    name: &str,
    func: unsafe extern "C" fn(*mut sqlite3_context, c_int, *mut *mut sqlite3_value),
) -> Result<(), String> {
    let c_name = CString::new(name).map_err(|_| "function name contains a nul".to_string())?;
    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c_name.as_ptr(),
            -1,
            FUNC_FLAGS,
            ptr::null_mut(),
            Some(func),
            None,
            None,
            None,
        )
    };
    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(format!("ragite could not register {name}"))
    }
}

fn register_catalog(db: *mut sqlite3) -> Result<(), String> {
    let module = catalog_module();
    for name in ["ragite_list_models", "ragite_default_model"] {
        let c_name = CString::new(name).unwrap();
        let rc =
            unsafe { sqlite3_create_module_v2(db, c_name.as_ptr(), module, ptr::null_mut(), None) };
        if rc != SQLITE_OK {
            return Err(format!("ragite could not register {name}"));
        }
    }
    Ok(())
}

fn catalog_module() -> &'static sqlite3_module {
    static MODULE: OnceLock<sqlite3_module> = OnceLock::new();
    MODULE.get_or_init(|| {
        let mut module: sqlite3_module = unsafe { mem::zeroed() };
        module.iVersion = 2;
        module.xConnect = Some(catalog_connect);
        module.xBestIndex = Some(catalog_best_index);
        module.xDisconnect = Some(catalog_disconnect);
        module.xDestroy = Some(catalog_disconnect);
        module.xOpen = Some(catalog_open);
        module.xClose = Some(catalog_close);
        module.xFilter = Some(catalog_filter);
        module.xNext = Some(catalog_next);
        module.xEof = Some(catalog_eof);
        module.xColumn = Some(catalog_column);
        module.xRowid = Some(catalog_rowid);
        module
    })
}

#[repr(C)]
struct CatalogVtab {
    base: sqlite3_vtab,
}

#[repr(C)]
struct CatalogCursor {
    base: sqlite3_vtab_cursor,
    row: i32,
}

unsafe extern "C" fn catalog_connect(
    db: *mut sqlite3,
    _aux: *mut c_void,
    _argc: c_int,
    _argv: *const *const c_char,
    pp_vtab: *mut *mut sqlite3_vtab,
    pz_err: *mut *mut c_char,
) -> c_int {
    let result = catch_unwind(AssertUnwindSafe(|| unsafe { connect_catalog(db, pp_vtab) }));
    match result {
        Ok(Ok(())) => SQLITE_OK,
        Ok(Err(msg)) => {
            set_pz(pz_err, &msg);
            SQLITE_ERROR
        }
        Err(_) => {
            set_pz(pz_err, "ragite failed while opening the model catalog");
            SQLITE_ERROR
        }
    }
}

unsafe fn connect_catalog(db: *mut sqlite3, pp_vtab: *mut *mut sqlite3_vtab) -> Result<(), String> {
    let schema = CString::new("CREATE TABLE x(name TEXT, technical_id TEXT)").unwrap();
    let rc = unsafe { sqlite3_declare_vtab(db, schema.as_ptr()) };
    if rc != SQLITE_OK {
        return Err(sql::errmsg(db));
    }
    let vtab = Box::new(CatalogVtab {
        base: unsafe { mem::zeroed() },
    });
    unsafe {
        *pp_vtab = Box::into_raw(vtab).cast();
    }
    Ok(())
}

unsafe extern "C" fn catalog_best_index(
    _vtab: *mut sqlite3_vtab,
    info: *mut libsqlite3_sys::sqlite3_index_info,
) -> c_int {
    if !info.is_null() {
        unsafe {
            (*info).estimatedCost = 1.0;
        }
    }
    SQLITE_OK
}

unsafe extern "C" fn catalog_disconnect(vtab: *mut sqlite3_vtab) -> c_int {
    if !vtab.is_null() {
        unsafe {
            drop(Box::from_raw(vtab.cast::<CatalogVtab>()));
        }
    }
    SQLITE_OK
}

unsafe extern "C" fn catalog_open(
    vtab: *mut sqlite3_vtab,
    pp_cursor: *mut *mut sqlite3_vtab_cursor,
) -> c_int {
    let cursor = Box::new(CatalogCursor {
        base: unsafe { mem::zeroed() },
        row: 0,
    });
    unsafe {
        let raw = Box::into_raw(cursor);
        (*raw).base.pVtab = vtab;
        *pp_cursor = raw.cast();
    }
    SQLITE_OK
}

unsafe extern "C" fn catalog_close(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    if !cursor.is_null() {
        unsafe {
            drop(Box::from_raw(cursor.cast::<CatalogCursor>()));
        }
    }
    SQLITE_OK
}

unsafe extern "C" fn catalog_filter(
    cursor: *mut sqlite3_vtab_cursor,
    _idx_num: c_int,
    _idx_str: *const c_char,
    _argc: c_int,
    _argv: *mut *mut sqlite3_value,
) -> c_int {
    let cursor = unsafe { &mut *cursor.cast::<CatalogCursor>() };
    cursor.row = 0;
    SQLITE_OK
}

unsafe extern "C" fn catalog_next(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    let cursor = unsafe { &mut *cursor.cast::<CatalogCursor>() };
    cursor.row += 1;
    SQLITE_OK
}

unsafe extern "C" fn catalog_eof(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    let cursor = unsafe { &*cursor.cast::<CatalogCursor>() };
    i32::from(cursor.row >= 1)
}

unsafe extern "C" fn catalog_column(
    cursor: *mut sqlite3_vtab_cursor,
    ctx: *mut sqlite3_context,
    col: c_int,
) -> c_int {
    let cursor = unsafe { &*cursor.cast::<CatalogCursor>() };
    if cursor.row != 0 {
        return SQLITE_ERROR;
    }
    match col {
        0 => result_text(ctx, MODEL_NAME),
        1 => result_text(ctx, TECHNICAL_ID),
        _ => SQLITE_ERROR,
    }
}

unsafe extern "C" fn catalog_rowid(cursor: *mut sqlite3_vtab_cursor, p_rowid: *mut i64) -> c_int {
    let cursor = unsafe { &*cursor.cast::<CatalogCursor>() };
    if cursor.row != 0 {
        return SQLITE_ERROR;
    }
    unsafe {
        *p_rowid = 1;
    }
    SQLITE_OK
}

unsafe extern "C" fn scalar_technical_id(
    ctx: *mut sqlite3_context,
    argc: c_int,
    _argv: *mut *mut sqlite3_value,
) {
    guard_ctx(ctx, || {
        if argc != 0 {
            result_error(ctx, "model catalog expects no arguments");
            return;
        }
        result_text(ctx, TECHNICAL_ID);
    });
}

unsafe extern "C" fn scalar_table_model(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    guard_ctx(ctx, || match table_model(ctx, argc, argv) {
        Ok(id) => {
            result_text(ctx, &id);
        }
        Err(err) => result_error(ctx, &err),
    });
}

fn table_model(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> Result<String, String> {
    if argc != 1 {
        return Err("ragite_table_model expects a table name".to_string());
    }
    let name = arg_text(argv, 0).unwrap_or_default();
    let db = db_handle(ctx)?;
    let doc = find_document_table(db, &name)?;
    vtab::stored_technical_id(db, &doc.schema, &doc.name)
}

unsafe extern "C" fn scalar_embed(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    guard_ctx(ctx, || match embed_table(ctx, argc, argv) {
        Ok(blob) => result_blob(ctx, &blob),
        Err(err) => result_error(ctx, &err),
    });
}

unsafe extern "C" fn scalar_embed_model(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    guard_ctx(ctx, || match embed_model(argc, argv) {
        Ok(blob) => result_blob(ctx, &blob),
        Err(err) => result_error(ctx, &err),
    });
}

fn embed_table(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> Result<Vec<u8>, String> {
    expect_argc(argc, "ragite_embed expects table, text, and role")?;
    let table = match arg_text(argv, 0) {
        Some(table) if !table.is_empty() => table,
        _ => return Err("table name is empty".to_string()),
    };
    let db = db_handle(ctx)?;
    let doc = find_document_table(db, &table)?;
    let role = parse_role(arg_text(argv, 2))?;
    let text = require_embed_text(arg_text(argv, 1))?;
    let model = require_stored_model(db, &doc.schema, &doc.name)?;
    Ok(model.embed_blob(&text, role))
}

fn embed_model(argc: c_int, argv: *mut *mut sqlite3_value) -> Result<Vec<u8>, String> {
    expect_argc(argc, "ragite_embed_model expects model, text, and role")?;
    let identifier = match arg_text(argv, 0) {
        Some(identifier) => identifier,
        None => return Err("model is required".to_string()),
    };
    accept_model_id(&identifier)?;
    let role = parse_role(arg_text(argv, 2))?;
    let text = require_embed_text(arg_text(argv, 1))?;
    let model = builtin_model()?;
    Ok(model.embed_blob(&text, role))
}

fn expect_argc(argc: c_int, message: &str) -> Result<(), String> {
    if argc == 2 {
        return Err("role is required".to_string());
    }
    if argc != 3 {
        return Err(message.to_string());
    }
    Ok(())
}

fn require_embed_text(text: Option<String>) -> Result<String, String> {
    match text {
        Some(text) if !text.is_empty() => Ok(text),
        Some(_) => Err("text is empty".to_string()),
        None => Err("text is required".to_string()),
    }
}

fn parse_role(role: Option<String>) -> Result<ragite_core::Role, String> {
    match role.as_deref() {
        None => Err("role is required".to_string()),
        Some("query") => Ok(ragite_core::Role::Query),
        Some("passage") => Ok(ragite_core::Role::Passage),
        Some("") => Err("role is not accepted: ''".to_string()),
        Some(other) => Err(format!("role is not accepted: {other}")),
    }
}

struct DocTable {
    schema: String,
    name: String,
}

fn find_document_table(db: *mut sqlite3, table_arg: &str) -> Result<DocTable, String> {
    let table_arg = table_arg.trim();
    if table_arg.is_empty() {
        return Err("table name is empty".to_string());
    }
    let (schema, name) = match table_arg.split_once('.') {
        Some((schema, name)) => {
            if schema.is_empty() || name.is_empty() || name.contains('.') {
                return Err(format!("table name is not accepted: {table_arg}"));
            }
            (Some(schema.to_string()), name.to_string())
        }
        None => (None, table_arg.to_string()),
    };
    let schemas = if let Some(schema) = schema {
        let Some(canonical) = canonical_schema(db, &schema)? else {
            return Err(format!("schema was not found: {schema}"));
        };
        vec![canonical]
    } else {
        search_schemas(db)?
    };
    let mut found_other = false;
    for schema in schemas {
        match table_sql(db, &schema, &name)? {
            Some((stored, sql)) if is_document_vtab(&sql) => {
                return Ok(DocTable {
                    schema,
                    name: stored,
                });
            }
            Some(_) => found_other = true,
            None => {}
        }
    }
    if found_other {
        Err(format!("table is not a document table: {table_arg}"))
    } else {
        Err(format!("table was not found: {table_arg}"))
    }
}

fn canonical_schema(db: *mut sqlite3, schema: &str) -> Result<Option<String>, String> {
    Ok(database_names(db)?
        .into_iter()
        .find(|name| name.eq_ignore_ascii_case(schema)))
}

fn search_schemas(db: *mut sqlite3) -> Result<Vec<String>, String> {
    let names = database_names(db)?;
    let mut ordered = Vec::new();
    if names.iter().any(|name| name == "temp") {
        ordered.push("temp".to_string());
    }
    for name in names {
        if name != "temp" {
            ordered.push(name);
        }
    }
    Ok(ordered)
}

fn database_names(db: *mut sqlite3) -> Result<Vec<String>, String> {
    let stmt = Stmt::prepare(db, "PRAGMA database_list")?;
    let mut names = Vec::new();
    while stmt.step()? {
        if let Some(name) = stmt.column_text(1) {
            names.push(name);
        }
    }
    Ok(names)
}

fn table_sql(
    db: *mut sqlite3,
    schema: &str,
    name: &str,
) -> Result<Option<(String, String)>, String> {
    let sql = format!(
        "SELECT name, ifnull(sql, '') FROM {}.sqlite_master \
         WHERE name = ?1 COLLATE NOCASE AND type = 'table'",
        sql::quote_ident(schema)
    );
    let stmt = Stmt::prepare(db, &sql)?;
    stmt.bind_text(1, name)?;
    if !stmt.step()? {
        return Ok(None);
    }
    Ok(Some((
        stmt.column_text(0).unwrap_or_default(),
        stmt.column_text(1).unwrap_or_default(),
    )))
}

fn is_document_vtab(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    let ragite = lower.contains("using ragite") || lower.contains("using \"ragite\"");
    let companion = lower.contains("companion=1") || lower.contains("companion = 1");
    ragite && !companion
}

fn db_handle(ctx: *mut sqlite3_context) -> Result<*mut sqlite3, String> {
    let db = unsafe { sqlite3_context_db_handle(ctx) };
    if db.is_null() {
        Err("ragite could not read the database".to_string())
    } else {
        Ok(db)
    }
}

fn arg_text(argv: *mut *mut sqlite3_value, index: c_int) -> Option<String> {
    if argv.is_null() {
        return None;
    }
    unsafe { vtab::value_text(*argv.offset(index as isize)) }
}

fn guard_ctx(ctx: *mut sqlite3_context, f: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(f)).is_err() {
        result_error(ctx, "ragite failed");
    }
}

fn result_error(ctx: *mut sqlite3_context, msg: &str) {
    let msg = msg.replace('\0', "");
    unsafe {
        sqlite3_result_error(ctx, msg.as_ptr().cast(), msg.len() as c_int);
    }
}

fn result_text(ctx: *mut sqlite3_context, value: &str) -> c_int {
    unsafe {
        sqlite3_result_text(
            ctx,
            value.as_ptr().cast(),
            value.len() as c_int,
            SQLITE_TRANSIENT(),
        );
    }
    SQLITE_OK
}

fn result_blob(ctx: *mut sqlite3_context, value: &[u8]) {
    unsafe {
        sqlite3_result_blob(
            ctx,
            value.as_ptr().cast(),
            value.len() as c_int,
            SQLITE_TRANSIENT(),
        );
    }
}

fn set_pz(pz_err: *mut *mut c_char, msg: &str) {
    if pz_err.is_null() {
        return;
    }
    unsafe {
        *pz_err = sql::alloc_text(msg);
    }
}
