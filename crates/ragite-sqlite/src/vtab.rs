//! Virtual tables `ragite` (documents) and the read-only companion `<name>_chunks`.

use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{LazyLock, Mutex, OnceLock};

use libsqlite3_sys::{
    SQLITE_DONE, SQLITE_ERROR, SQLITE_FLOAT, SQLITE_INDEX_CONSTRAINT_EQ,
    SQLITE_INDEX_CONSTRAINT_MATCH, SQLITE_INDEX_SCAN_UNIQUE, SQLITE_INTEGER, SQLITE_NULL,
    SQLITE_OK, SQLITE_TEXT, SQLITE_TRANSIENT, sqlite3, sqlite3_declare_vtab, sqlite3_free,
    sqlite3_index_info, sqlite3_module, sqlite3_value, sqlite3_value_bytes, sqlite3_value_dup,
    sqlite3_value_free, sqlite3_value_int64, sqlite3_value_text, sqlite3_value_type, sqlite3_vtab,
    sqlite3_vtab_cursor,
};
use ragite_core::{
    BuiltinModel, Chunk, FUSION_DEPTH, FusedChunk, Role, TECHNICAL_ID, best_document_scores,
    builtin_model, chunk_docling, chunk_html, chunk_markdown, chunk_plain, heading_path_json,
    order_fused_chunks, reciprocal_rank_fusion, sha256_hex,
};

use crate::sql::{self, Stmt};

const DOC_COLUMNS: &str =
    "doc_id INTEGER, text TEXT, source TEXT, format TEXT, hash TEXT, score REAL";
const CHUNK_COLUMNS: &str = "chunk TEXT, doc_id INTEGER, source TEXT, start INTEGER, end INTEGER, heading_path TEXT, meta_json TEXT, score REAL";

unsafe extern "C" {
    fn ragite_vtab_in(info: *mut sqlite3_index_info, i_cons: c_int, handle: c_int) -> c_int;
    fn ragite_vtab_in_first(value: *mut sqlite3_value, out: *mut *mut sqlite3_value) -> c_int;
    fn ragite_vtab_in_next(value: *mut sqlite3_value, out: *mut *mut sqlite3_value) -> c_int;
}

const IDX_SCAN: c_int = 0;
const IDX_MATCH: c_int = 1;
const IDX_ROWID: c_int = 2;

const COL_DOC_ID: c_int = 0;
const COL_TEXT: c_int = 1;
const COL_SOURCE: c_int = 2;
const COL_FORMAT: c_int = 3;
const COL_HASH: c_int = 4;
const COL_SCORE: c_int = 5;

const COL_CHUNK: c_int = 0;
const COL_C_DOC: c_int = 1;
const COL_C_SOURCE: c_int = 2;
const COL_START: c_int = 3;
const COL_END: c_int = 4;
const COL_PATH: c_int = 5;
const COL_META: c_int = 6;
const COL_C_SCORE: c_int = 7;

static ALLOW_DROP: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
/// Set only around the companion `CREATE` issued by `create_shadows`.
static ALLOW_CREATE: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn module() -> &'static sqlite3_module {
    static MODULE: OnceLock<sqlite3_module> = OnceLock::new();
    MODULE.get_or_init(|| {
        let mut module: sqlite3_module = unsafe { mem::zeroed() };
        module.iVersion = 2;
        module.xCreate = Some(vtab_create);
        module.xConnect = Some(vtab_connect);
        module.xBestIndex = Some(vtab_best_index);
        module.xDisconnect = Some(vtab_disconnect);
        module.xDestroy = Some(vtab_destroy);
        module.xOpen = Some(vtab_open);
        module.xClose = Some(vtab_close);
        module.xFilter = Some(vtab_filter);
        module.xNext = Some(vtab_next);
        module.xEof = Some(vtab_eof);
        module.xColumn = Some(vtab_column);
        module.xRowid = Some(vtab_rowid);
        module.xUpdate = Some(vtab_update);
        // A null xRename lets SQLite rename the schema row and leave the
        // shadow tables under the old name. Refusing keeps both names put.
        module.xRename = Some(vtab_rename);
        module
    })
}

#[repr(C)]
struct Vtab {
    base: sqlite3_vtab,
    db: *mut sqlite3,
    schema: String,
    doc_name: String,
    chunks: bool,
    /// Connect-time result of `read_model_ok`. Writers re-read the shadow row,
    /// because `<name>_model` can change without a reconnect.
    #[allow(dead_code)]
    model_ok: bool,
}

#[repr(C)]
struct Cursor {
    base: sqlite3_vtab_cursor,
    rows: Vec<Row>,
    index: usize,
}

enum Row {
    Doc {
        doc_id: i64,
        text: String,
        source: String,
        format: String,
        hash: String,
        score: Option<f64>,
    },
    Chunk {
        chunk_id: i64,
        doc_id: i64,
        source: String,
        start: i64,
        end: i64,
        heading_path: String,
        text: String,
        meta_json: Option<String>,
        score: Option<f64>,
    },
}

/// `name MATCH ?` resolves only when a column is named `name`. The column is hidden.
fn declared_schema(chunks: bool, vtab_name: &str) -> String {
    let hidden = sql::quote_ident(vtab_name);
    let columns = if chunks { CHUNK_COLUMNS } else { DOC_COLUMNS };
    format!("CREATE TABLE x({columns}, {hidden} HIDDEN)")
}

fn drop_key(schema: &str, name: &str) -> String {
    format!("{schema}.{name}")
}

fn guard(default: c_int, f: impl FnOnce() -> c_int + std::panic::UnwindSafe) -> c_int {
    match catch_unwind(f) {
        Ok(rc) => rc,
        Err(_) => default,
    }
}

unsafe extern "C" fn vtab_create(
    db: *mut sqlite3,
    _aux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    pp_vtab: *mut *mut sqlite3_vtab,
    pz_err: *mut *mut c_char,
) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| unsafe {
            match create(db, argc, argv, pp_vtab) {
                Ok(()) => SQLITE_OK,
                Err(msg) => {
                    if !pz_err.is_null() {
                        *pz_err = sql::alloc_text(&msg);
                    }
                    SQLITE_ERROR
                }
            }
        }),
    )
}

unsafe extern "C" fn vtab_connect(
    db: *mut sqlite3,
    _aux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    pp_vtab: *mut *mut sqlite3_vtab,
    pz_err: *mut *mut c_char,
) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| unsafe {
            match connect(db, argc, argv, pp_vtab, false) {
                Ok(()) => SQLITE_OK,
                Err(msg) => {
                    if !pz_err.is_null() {
                        *pz_err = sql::alloc_text(&msg);
                    }
                    SQLITE_ERROR
                }
            }
        }),
    )
}

unsafe fn create(
    db: *mut sqlite3,
    argc: c_int,
    argv: *const *const c_char,
    pp_vtab: *mut *mut sqlite3_vtab,
) -> Result<(), String> {
    let args = read_args(argc, argv)?;
    if args.declared_companion || args.declared_parent {
        let key = drop_key(&args.schema, &args.vtab_name);
        let allowed = args.chunks
            && args.vtab_name == format!("{}_chunks", args.table)
            && ALLOW_CREATE.lock().expect("create set").contains(&key);
        if !allowed {
            let which = if args.declared_companion {
                "companion"
            } else {
                "parent"
            };
            return Err(format!("argument is not accepted: {which}"));
        }
    }
    if args.chunks {
        return unsafe { connect(db, argc, argv, pp_vtab, false) };
    }
    if let Err(err) = builtin_model() {
        return Err(err);
    }
    let model_id = resolve_model(&args.model)?;
    for name in shadow_names(&args.table) {
        if sql::exists(db, &args.schema, &name)? {
            return Err(format!("name already exists: {name}"));
        }
    }
    if let Err(err) = create_shadows(db, &args.schema, &args.table, &model_id) {
        let _ = destroy_shadows(db, &args.schema, &args.table);
        return Err(err);
    }
    if let Err(err) = unsafe { connect(db, argc, argv, pp_vtab, true) } {
        let _ = destroy_shadows(db, &args.schema, &args.table);
        return Err(err);
    }
    Ok(())
}

fn shadow_names(table: &str) -> [String; 6] {
    [
        format!("{table}_chunks"),
        format!("{table}_documents"),
        format!("{table}_chunk_rows"),
        format!("{table}_vec"),
        format!("{table}_fts"),
        format!("{table}_model"),
    ]
}

fn create_shadows(
    db: *mut sqlite3,
    schema: &str,
    table: &str,
    model_id: &str,
) -> Result<(), String> {
    let model = builtin_model()?;
    let documents = sql::qualified(schema, &format!("{table}_documents"));
    let chunks = sql::qualified(schema, &format!("{table}_chunk_rows"));
    let model_table = sql::qualified(schema, &format!("{table}_model"));
    let vec = sql::qualified(schema, &format!("{table}_vec"));
    let fts = sql::qualified(schema, &format!("{table}_fts"));
    let companion = sql::qualified(schema, &format!("{table}_chunks"));
    sql::exec(
        db,
        &format!(
            "CREATE TABLE {documents} (
                doc_id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                text TEXT NOT NULL,
                format TEXT NOT NULL,
                hash TEXT NOT NULL
            )"
        ),
    )?;
    sql::exec(
        db,
        &format!(
            "CREATE TABLE {chunks} (
                chunk_id INTEGER PRIMARY KEY AUTOINCREMENT,
                doc_id INTEGER NOT NULL,
                start INTEGER NOT NULL,
                end INTEGER NOT NULL,
                heading_path TEXT NOT NULL,
                text TEXT NOT NULL,
                meta_json TEXT
            )"
        ),
    )?;
    sql::exec(
        db,
        &format!("CREATE TABLE {model_table} (technical_id TEXT NOT NULL)"),
    )?;
    let insert = Stmt::prepare(
        db,
        &format!("INSERT INTO {model_table} (technical_id) VALUES (?1)"),
    )?;
    insert.bind_text(1, model_id)?;
    insert.step()?;
    sql::exec(
        db,
        &format!(
            "CREATE VIRTUAL TABLE {vec} USING vec0(embedding float[{}] distance_metric=cosine)",
            model.dimension()
        ),
    )?;
    sql::exec(
        db,
        &format!(
            "CREATE VIRTUAL TABLE {fts} USING fts5(text, tokenize = 'unicode61', content='', contentless_delete=1)"
        ),
    )?;
    let parent = args_parent_literal(table);
    let companion_name = format!("{table}_chunks");
    let key = drop_key(schema, &companion_name);
    ALLOW_CREATE.lock().expect("create set").insert(key.clone());
    let created = sql::exec(
        db,
        &format!("CREATE VIRTUAL TABLE {companion} USING ragite(companion=1, parent={parent})"),
    );
    ALLOW_CREATE.lock().expect("create set").remove(&key);
    created?;
    Ok(())
}

fn args_parent_literal(table: &str) -> String {
    format!("'{}'", table.replace('\'', "''"))
}

fn destroy_shadows(db: *mut sqlite3, schema: &str, table: &str) -> Result<(), String> {
    let companion = format!("{table}_chunks");
    let key = drop_key(schema, &companion);
    ALLOW_DROP.lock().expect("drop set").insert(key.clone());
    let drop_companion = sql::exec(
        db,
        &format!(
            "DROP TABLE IF EXISTS {}",
            sql::qualified(schema, &companion)
        ),
    );
    ALLOW_DROP.lock().expect("drop set").remove(&key);
    drop_companion?;
    for suffix in ["_vec", "_fts", "_chunk_rows", "_documents", "_model"] {
        sql::exec(
            db,
            &format!(
                "DROP TABLE IF EXISTS {}",
                sql::qualified(schema, &format!("{table}{suffix}"))
            ),
        )?;
    }
    Ok(())
}

unsafe fn connect(
    db: *mut sqlite3,
    argc: c_int,
    argv: *const *const c_char,
    pp_vtab: *mut *mut sqlite3_vtab,
    just_created: bool,
) -> Result<(), String> {
    let args = read_args(argc, argv)?;
    let model_ok = if args.chunks {
        read_model_ok(db, &args.schema, &args.table).unwrap_or(false)
    } else if just_created {
        true
    } else {
        read_model_ok(db, &args.schema, &args.table)?
    };
    let schema_sql = declared_schema(args.chunks, &args.vtab_name);
    let c_schema =
        std::ffi::CString::new(schema_sql).map_err(|_| "table name contains a nul".to_string())?;
    let rc = unsafe { sqlite3_declare_vtab(db, c_schema.as_ptr()) };
    if rc != SQLITE_OK {
        return Err(sql::errmsg(db));
    }
    let vtab = Box::new(Vtab {
        base: unsafe { mem::zeroed() },
        db,
        schema: args.schema,
        doc_name: args.table,
        chunks: args.chunks,
        model_ok,
    });
    unsafe {
        *pp_vtab = Box::into_raw(vtab).cast();
    }
    Ok(())
}

pub(crate) fn stored_technical_id(
    db: *mut sqlite3,
    schema: &str,
    table: &str,
) -> Result<String, String> {
    let sql = format!(
        "SELECT technical_id FROM {}",
        sql::qualified(schema, &format!("{table}_model"))
    );
    let stmt = Stmt::prepare(db, &sql)?;
    if !stmt.step()? {
        return Err(format!("stored model is missing: {table}"));
    }
    Ok(stmt.column_text(0).unwrap_or_default())
}

fn read_model_ok(db: *mut sqlite3, schema: &str, table: &str) -> Result<bool, String> {
    let stored = stored_technical_id(db, schema, table)?;
    Ok(stored == TECHNICAL_ID && builtin_model().is_ok())
}

pub(crate) fn require_stored_model(
    db: *mut sqlite3,
    schema: &str,
    table: &str,
) -> Result<&'static BuiltinModel, String> {
    let stored = stored_technical_id(db, schema, table)?;
    if stored != TECHNICAL_ID {
        return Err(format!("model cannot be loaded: {stored}"));
    }
    builtin_model()
}

struct Args {
    schema: String,
    /// Document table name. On the companion this is the parent, not `<name>_chunks`.
    table: String,
    /// Name SQLite created this virtual table under. `name MATCH` binds to it.
    vtab_name: String,
    chunks: bool,
    /// `companion` was present in the module arguments.
    declared_companion: bool,
    /// `parent` was present in the module arguments.
    declared_parent: bool,
    model: Option<String>,
}

fn read_args(argc: c_int, argv: *const *const c_char) -> Result<Args, String> {
    if argc < 3 {
        return Err("create is missing a table name".to_string());
    }
    let mut raw = Vec::new();
    for i in 0..argc {
        raw.push(unsafe { cstr_at(argv, i) });
    }
    let schema = raw[1].clone();
    let vtab_name = raw[2].clone();
    let mut table = vtab_name.clone();
    let mut chunks = false;
    let mut declared_companion = false;
    let mut declared_parent = false;
    let mut model = None;
    for arg in raw.iter().skip(3) {
        let Some((key, raw)) = split_arg(arg) else {
            return Err(format!("argument is not accepted: {arg}"));
        };
        match key {
            "companion" => {
                declared_companion = true;
                if unquote(raw) != "1" {
                    return Err(format!("argument is not accepted: {arg}"));
                }
                chunks = true;
            }
            "parent" => {
                declared_parent = true;
                let value = unquote(raw);
                if value.is_empty() {
                    return Err("parent table is empty".to_string());
                }
                table = value;
            }
            "model" => {
                if is_unquoted_null(raw) {
                    model = None;
                } else {
                    model = Some(unquote(raw));
                }
            }
            other => return Err(format!("argument is not accepted: {other}")),
        }
    }
    if schema.is_empty() || table.is_empty() {
        return Err("table name is empty".to_string());
    }
    Ok(Args {
        schema,
        table,
        vtab_name,
        chunks,
        declared_companion,
        declared_parent,
        model,
    })
}

fn resolve_model(model: &Option<String>) -> Result<String, String> {
    let Some(value) = model else {
        return Ok(TECHNICAL_ID.to_string());
    };
    accept_model_id(value)?;
    Ok(value.clone())
}

pub(crate) fn accept_model_id(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("model is empty".to_string());
    }
    if value == "default" {
        return Err("model is not accepted: default".to_string());
    }
    if value.contains("://") || value.contains('/') || value.contains('\\') {
        return Err(format!("model is not accepted: {value}"));
    }
    if value != TECHNICAL_ID {
        return Err(format!("model cannot be loaded: {value}"));
    }
    builtin_model()?;
    Ok(())
}

fn split_arg(arg: &str) -> Option<(&str, &str)> {
    let (key, value) = arg.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let quote = bytes[0];
        if (quote == b'\'' || quote == b'"') && bytes[bytes.len() - 1] == quote {
            let inner = &value[1..value.len() - 1];
            return if quote == b'\'' {
                inner.replace("''", "'")
            } else {
                inner.replace("\"\"", "\"")
            };
        }
    }
    value.to_string()
}

fn is_unquoted_null(value: &str) -> bool {
    !value.starts_with('\'') && !value.starts_with('"') && value.eq_ignore_ascii_case("null")
}

unsafe fn cstr_at(argv: *const *const c_char, index: c_int) -> String {
    let ptr = unsafe { *argv.offset(index as isize) };
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

unsafe extern "C" fn vtab_best_index(
    vtab: *mut sqlite3_vtab,
    info: *mut sqlite3_index_info,
) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| unsafe { best_index(vtab, info) }),
    )
}

/// Columns whose `=` or `IN` can select Chunks before the depth is counted.
/// `score` is omitted on both tables. The companion does not expose `format` or `hash`.
fn match_eq_column(chunks: bool, column: c_int) -> bool {
    if chunks {
        column == -1
            || column == COL_CHUNK
            || column == COL_C_DOC
            || column == COL_C_SOURCE
            || column == COL_START
            || column == COL_END
            || column == COL_PATH
            || column == COL_META
    } else {
        column == -1
            || column == COL_DOC_ID
            || column == COL_TEXT
            || column == COL_SOURCE
            || column == COL_FORMAT
            || column == COL_HASH
    }
}

unsafe fn best_index(vtab: *mut sqlite3_vtab, info_ptr: *mut sqlite3_index_info) -> c_int {
    if info_ptr.is_null() {
        return SQLITE_ERROR;
    }
    let chunks = !vtab.is_null() && unsafe { (*vtab.cast::<Vtab>()).chunks };
    let info = unsafe { &mut *info_ptr };
    info.idxNum = IDX_SCAN;
    info.estimatedCost = 1_000_000.0;
    if info.nConstraint <= 0 || info.aConstraint.is_null() || info.aConstraintUsage.is_null() {
        return SQLITE_OK;
    }
    let n = info.nConstraint as usize;
    let constraints = unsafe { std::slice::from_raw_parts(info.aConstraint, n) };
    let usage = unsafe { std::slice::from_raw_parts_mut(info.aConstraintUsage, n) };
    let mut match_at = None;
    let mut rowid_at = None;
    for (index, constraint) in constraints.iter().enumerate() {
        if constraint.usable == 0 {
            continue;
        }
        if match_at.is_none() && constraint.op == SQLITE_INDEX_CONSTRAINT_MATCH as u8 {
            match_at = Some(index);
        }
        if rowid_at.is_none()
            && constraint.iColumn == -1
            && constraint.op == SQLITE_INDEX_CONSTRAINT_EQ as u8
        {
            rowid_at = Some(index);
        }
    }
    // SQLite cannot recheck MATCH. When it is usable, that search is the plan.
    if let Some(index) = match_at {
        usage[index].argvIndex = 1;
        usage[index].omit = 1;
        info.idxNum = IDX_MATCH;
        // The unconstrained plan must cost more than one that also consumes a
        // column constraint. When the costs are equal, SQLite keeps only the
        // plan with fewer tables, and that plan is the unconstrained MATCH.
        // An inner join then misses rows that the same join returns as a
        // CROSS JOIN (REQ-102).
        info.estimatedCost = 1_000_000.0;
        info.estimatedRows = FUSION_DEPTH as i64;
        let mut argv_index = 2;
        let mut columns = Vec::new();
        for (index, constraint) in constraints.iter().enumerate() {
            if constraint.usable == 0
                || constraint.op != SQLITE_INDEX_CONSTRAINT_EQ as u8
                || !match_eq_column(chunks, constraint.iColumn)
            {
                continue;
            }
            let in_list = unsafe { ragite_vtab_in(info_ptr, index as c_int, -1) } != 0;
            if in_list {
                unsafe { ragite_vtab_in(info_ptr, index as c_int, 1) };
            }
            usage[index].argvIndex = argv_index;
            usage[index].omit = 1;
            argv_index += 1;
            let token = if in_list {
                format!("i{}", constraint.iColumn)
            } else {
                constraint.iColumn.to_string()
            };
            columns.push(token);
        }
        if !columns.is_empty() {
            info.estimatedCost = 10.0;
            let encoded = columns.join(",");
            let ptr = sql::alloc_text(&encoded);
            if ptr.is_null() {
                return SQLITE_ERROR;
            }
            info.idxStr = ptr;
            info.needToFreeIdxStr = 1;
        }
        return SQLITE_OK;
    }
    if let Some(index) = rowid_at {
        usage[index].argvIndex = 1;
        // sqlite3_value_int64 truncates a non-integral value. Leaving the
        // constraint in place lets SQLite reject 2.5 and '2.5' the same way
        // a rowid table does, while 2.0 and '2' still match (REQ-96).
        usage[index].omit = 0;
        info.idxNum = IDX_ROWID;
        info.idxStr = c"rowid".as_ptr().cast_mut();
        info.needToFreeIdxStr = 0;
        info.estimatedCost = 1.0;
        info.estimatedRows = 1;
        info.idxFlags = SQLITE_INDEX_SCAN_UNIQUE;
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_disconnect(vtab: *mut sqlite3_vtab) -> c_int {
    if !vtab.is_null() {
        unsafe {
            drop(Box::from_raw(vtab.cast::<Vtab>()));
        }
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_destroy(vtab: *mut sqlite3_vtab) -> c_int {
    guard(SQLITE_ERROR, AssertUnwindSafe(|| unsafe { destroy(vtab) }))
}

unsafe fn destroy(vtab: *mut sqlite3_vtab) -> c_int {
    let table = unsafe { &*vtab.cast::<Vtab>() };
    if table.chunks {
        let key = drop_key(&table.schema, &format!("{}_chunks", table.doc_name));
        let allowed = ALLOW_DROP.lock().expect("drop set").remove(&key);
        if !allowed {
            // DROP reports SQLite's generic error. xDestroy's zErrMsg is not copied.
            set_vtab_error(
                vtab,
                &format!("cannot drop companion table: {}_chunks", table.doc_name),
            );
            return SQLITE_ERROR;
        }
        unsafe {
            drop(Box::from_raw(vtab.cast::<Vtab>()));
        }
        return SQLITE_OK;
    }
    let schema = table.schema.clone();
    let name = table.doc_name.clone();
    let db = table.db;
    if let Err(err) = destroy_shadows(db, &schema, &name) {
        set_vtab_error(vtab, &format!("drop failed: {err}"));
        return SQLITE_ERROR;
    }
    unsafe {
        drop(Box::from_raw(vtab.cast::<Vtab>()));
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_rename(vtab: *mut sqlite3_vtab, _new_name: *const c_char) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| {
            set_vtab_error(vtab, "rename is not accepted");
            SQLITE_ERROR
        }),
    )
}

fn set_vtab_error(vtab: *mut sqlite3_vtab, msg: &str) {
    unsafe {
        if !(*vtab).zErrMsg.is_null() {
            sqlite3_free((*vtab).zErrMsg.cast());
        }
        (*vtab).zErrMsg = sql::alloc_text(msg);
    }
}

unsafe extern "C" fn vtab_open(
    vtab: *mut sqlite3_vtab,
    pp_cursor: *mut *mut sqlite3_vtab_cursor,
) -> c_int {
    let cursor = Box::new(Cursor {
        base: unsafe { mem::zeroed() },
        rows: Vec::new(),
        index: 0,
    });
    unsafe {
        let raw = Box::into_raw(cursor);
        (*raw).base.pVtab = vtab;
        *pp_cursor = raw.cast();
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_close(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    if !cursor.is_null() {
        unsafe {
            drop(Box::from_raw(cursor.cast::<Cursor>()));
        }
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_filter(
    cursor: *mut sqlite3_vtab_cursor,
    idx_num: c_int,
    idx_str: *const c_char,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| unsafe { filter(cursor, idx_num, idx_str, argc, argv) }),
    )
}

unsafe fn filter(
    cursor: *mut sqlite3_vtab_cursor,
    idx_num: c_int,
    idx_str: *const c_char,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> c_int {
    let cursor = unsafe { &mut *cursor.cast::<Cursor>() };
    let vtab = unsafe { &*cursor.base.pVtab.cast::<Vtab>() };
    let loaded = if idx_num == IDX_MATCH {
        let query = if argc > 0 && !argv.is_null() {
            value_text(unsafe { *argv }).unwrap_or_default()
        } else {
            String::new()
        };
        match pushed_eqs(idx_str, argc, argv) {
            Ok(Some(eqs)) => search_rows(vtab, &query, &eqs),
            Ok(None) => Ok(Vec::new()),
            Err(err) => Err(err),
        }
    } else if idx_num == IDX_ROWID {
        match unsafe { rowid_argument(argc, argv) } {
            Some(rowid) => load_row(vtab, rowid),
            None => Ok(Vec::new()),
        }
    } else {
        load_rows(vtab)
    };
    match loaded {
        Ok(rows) => {
            cursor.rows = rows;
            cursor.index = 0;
            SQLITE_OK
        }
        Err(err) => {
            set_vtab_error(cursor.base.pVtab, &err);
            SQLITE_ERROR
        }
    }
}

fn load_rows(vtab: &Vtab) -> Result<Vec<Row>, String> {
    if vtab.chunks {
        let sql = format!(
            "SELECT c.chunk_id, c.doc_id, d.source, c.start, c.end, c.heading_path, c.text, c.meta_json
             FROM {chunks} c
             JOIN {docs} d ON d.doc_id = c.doc_id
             ORDER BY c.doc_id, c.chunk_id",
            chunks = sql::qualified(&vtab.schema, &format!("{}_chunk_rows", vtab.doc_name)),
            docs = sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name)),
        );
        let stmt = Stmt::prepare(vtab.db, &sql)?;
        let mut rows = Vec::new();
        while stmt.step()? {
            rows.push(Row::Chunk {
                chunk_id: stmt.column_i64(0),
                doc_id: stmt.column_i64(1),
                source: stmt.column_text(2).unwrap_or_default(),
                start: stmt.column_i64(3),
                end: stmt.column_i64(4),
                heading_path: stmt.column_text(5).unwrap_or_else(|| "[]".to_string()),
                text: stmt.column_text(6).unwrap_or_default(),
                meta_json: stmt.column_text(7),
                score: None,
            });
        }
        Ok(rows)
    } else {
        let sql = format!(
            "SELECT doc_id, text, source, format, hash FROM {} ORDER BY doc_id",
            sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name))
        );
        let stmt = Stmt::prepare(vtab.db, &sql)?;
        let mut rows = Vec::new();
        while stmt.step()? {
            rows.push(Row::Doc {
                doc_id: stmt.column_i64(0),
                text: stmt.column_text(1).unwrap_or_default(),
                source: stmt.column_text(2).unwrap_or_default(),
                format: stmt.column_text(3).unwrap_or_default(),
                hash: stmt.column_text(4).unwrap_or_default(),
                score: None,
            });
        }
        Ok(rows)
    }
}

unsafe fn rowid_argument(argc: c_int, argv: *mut *mut sqlite3_value) -> Option<i64> {
    if argc < 1 || argv.is_null() {
        return None;
    }
    let value = unsafe { *argv };
    if value.is_null() || unsafe { sqlite3_value_type(value) } == SQLITE_NULL {
        return None;
    }
    Some(unsafe { sqlite3_value_int64(value) })
}

fn load_row(vtab: &Vtab, rowid: i64) -> Result<Vec<Row>, String> {
    if vtab.chunks {
        let sql = format!(
            "SELECT c.chunk_id, c.doc_id, d.source, c.start, c.end, c.heading_path, c.text, c.meta_json
             FROM {chunks} c
             JOIN {docs} d ON d.doc_id = c.doc_id
             WHERE c.chunk_id = ?1",
            chunks = sql::qualified(&vtab.schema, &format!("{}_chunk_rows", vtab.doc_name)),
            docs = sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name)),
        );
        let stmt = Stmt::prepare(vtab.db, &sql)?;
        stmt.bind_i64(1, rowid)?;
        if !stmt.step()? {
            return Ok(Vec::new());
        }
        Ok(vec![Row::Chunk {
            chunk_id: stmt.column_i64(0),
            doc_id: stmt.column_i64(1),
            source: stmt.column_text(2).unwrap_or_default(),
            start: stmt.column_i64(3),
            end: stmt.column_i64(4),
            heading_path: stmt.column_text(5).unwrap_or_else(|| "[]".to_string()),
            text: stmt.column_text(6).unwrap_or_default(),
            meta_json: stmt.column_text(7),
            score: None,
        }])
    } else {
        let sql = format!(
            "SELECT doc_id, text, source, format, hash FROM {} WHERE doc_id = ?1",
            sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name))
        );
        let stmt = Stmt::prepare(vtab.db, &sql)?;
        stmt.bind_i64(1, rowid)?;
        if !stmt.step()? {
            return Ok(Vec::new());
        }
        Ok(vec![Row::Doc {
            doc_id: stmt.column_i64(0),
            text: stmt.column_text(1).unwrap_or_default(),
            source: stmt.column_text(2).unwrap_or_default(),
            format: stmt.column_text(3).unwrap_or_default(),
            hash: stmt.column_text(4).unwrap_or_default(),
            score: None,
        }])
    }
}

struct HeldValue {
    value: *mut sqlite3_value,
    owned: bool,
}

impl Drop for HeldValue {
    fn drop(&mut self) {
        if self.owned && !self.value.is_null() {
            unsafe { sqlite3_value_free(self.value) };
        }
    }
}

struct PushedTerm {
    column: c_int,
    values: Vec<HeldValue>,
}

/// `idxStr` lists consumed constraints, in argv order after the MATCH argument.
/// A token `iN` is an `IN` list on column N. Any other token is `=` on that column.
/// `Ok(None)` means the constraints match no stored value.
fn pushed_eqs(
    idx_str: *const c_char,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> Result<Option<Vec<PushedTerm>>, String> {
    if idx_str.is_null() {
        return Ok(Some(Vec::new()));
    }
    let text = unsafe { CStr::from_ptr(idx_str) }.to_string_lossy();
    if text.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let mut eqs = Vec::new();
    for (offset, part) in text.split(',').enumerate() {
        let in_list = part.starts_with('i');
        let number = if in_list { &part[1..] } else { part };
        let column = number
            .parse::<c_int>()
            .map_err(|_| format!("search plan is invalid: {text}"))?;
        let argv_index = 1 + offset as isize;
        if argv.is_null() || argv_index >= argc as isize {
            return Err(format!("search plan is invalid: {text}"));
        }
        let value = unsafe { *argv.offset(argv_index) };
        if value.is_null() {
            return Err(format!("search plan is invalid: {text}"));
        }
        let values = if in_list {
            in_values(value)?
        } else if unsafe { sqlite3_value_type(value) } == SQLITE_NULL {
            return Ok(None);
        } else {
            vec![HeldValue {
                value,
                owned: false,
            }]
        };
        if values.is_empty() {
            return Ok(None);
        }
        eqs.push(PushedTerm { column, values });
    }
    Ok(Some(eqs))
}

fn in_values(handle: *mut sqlite3_value) -> Result<Vec<HeldValue>, String> {
    let mut values = Vec::new();
    let mut item = std::ptr::null_mut();
    let mut rc = unsafe { ragite_vtab_in_first(handle, &mut item) };
    while rc == SQLITE_OK && !item.is_null() {
        if unsafe { sqlite3_value_type(item) } != SQLITE_NULL {
            let dup = unsafe { sqlite3_value_dup(item) };
            if dup.is_null() {
                return Err("out of memory".to_string());
            }
            values.push(HeldValue {
                value: dup,
                owned: true,
            });
        }
        rc = unsafe { ragite_vtab_in_next(handle, &mut item) };
    }
    if rc != SQLITE_DONE && rc != SQLITE_OK {
        return Err("search constraint list failed".to_string());
    }
    Ok(values)
}

fn search_rows(vtab: &Vtab, query: &str, eqs: &[PushedTerm]) -> Result<Vec<Row>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let model = require_model(vtab)?;
    let blob = model.embed_blob(query, Role::Query);
    let depth = FUSION_DEPTH as i64;
    let vector = vector_ids(vtab, &blob, depth, eqs)?;
    let full_text = fts_ids(vtab, query, depth, eqs)?;
    let fused = reciprocal_rank_fusion(&vector, &full_text);
    let doc_of = chunk_documents(vtab, &fused)?;
    let mut chunks = Vec::new();
    for (chunk_id, score) in fused {
        let Some(doc_id) = doc_of.get(&chunk_id).copied() else {
            continue;
        };
        chunks.push(FusedChunk {
            chunk_id,
            doc_id,
            score,
        });
    }
    if vtab.chunks {
        order_fused_chunks(&mut chunks);
        load_scored_chunks(vtab, &chunks)
    } else {
        load_scored_docs(vtab, &best_document_scores(&chunks))
    }
}

fn vector_ids(
    vtab: &Vtab,
    blob: &[u8],
    depth: i64,
    eqs: &[PushedTerm],
) -> Result<Vec<i64>, String> {
    let vec_name = sql::qualified(&vtab.schema, &format!("{}_vec", vtab.doc_name));
    let restrict = chunk_id_restriction(vtab, eqs)?;
    let stmt = Stmt::prepare(
        vtab.db,
        &format!(
            "SELECT rowid FROM {vec_name} WHERE embedding MATCH ?1 AND k = ?2{restrict} ORDER BY distance"
        ),
    )?;
    stmt.bind_blob(1, blob)?;
    stmt.bind_i64(2, depth)?;
    bind_eqs(&stmt, eqs)?;
    read_ids(&stmt)
}

fn fts_ids(vtab: &Vtab, query: &str, depth: i64, eqs: &[PushedTerm]) -> Result<Vec<i64>, String> {
    let bare = format!("{}_fts", vtab.doc_name);
    let from = sql::qualified(&vtab.schema, &bare);
    let name = sql::quote_ident(&bare);
    let restrict = chunk_id_restriction(vtab, eqs)?;
    let stmt = Stmt::prepare(
        vtab.db,
        &format!(
            "SELECT rowid FROM {from} WHERE {name} MATCH ?1{restrict} ORDER BY bm25({name}) LIMIT ?2"
        ),
    )?;
    stmt.bind_text(1, &fts_phrase(query))?;
    stmt.bind_i64(2, depth)?;
    bind_eqs(&stmt, eqs)?;
    read_ids(&stmt)
}

fn bind_eqs(stmt: &Stmt, eqs: &[PushedTerm]) -> Result<(), String> {
    let mut param = 3;
    for eq in eqs {
        for value in &eq.values {
            stmt.bind_value(param, value.value)?;
            param += 1;
        }
    }
    Ok(())
}

fn column_expr(chunks: bool, column: c_int) -> Result<&'static str, String> {
    Ok(match (chunks, column) {
        (true, COL_C_SOURCE) | (false, COL_SOURCE) => "d.source",
        (true, COL_CHUNK) => "c.text",
        (true, -1) => "c.chunk_id",
        (true, COL_C_DOC) => "c.doc_id",
        (true, COL_START) => "c.start",
        (true, COL_END) => "c.\"end\"",
        (true, COL_PATH) => "c.heading_path",
        (true, COL_META) => "c.meta_json",
        (false, -1) | (false, COL_DOC_ID) => "d.doc_id",
        (false, COL_TEXT) => "d.text",
        (false, COL_FORMAT) => "d.format",
        (false, COL_HASH) => "d.hash",
        _ => return Err("search plan is invalid".to_string()),
    })
}

/// Chunk ids that satisfy the pushed equalities. Empty when the search is not narrowed.
fn chunk_id_restriction(vtab: &Vtab, eqs: &[PushedTerm]) -> Result<String, String> {
    if eqs.is_empty() {
        return Ok(String::new());
    }
    let mut param = 3usize;
    let mut terms = Vec::with_capacity(eqs.len());
    for eq in eqs {
        let expr = column_expr(vtab.chunks, eq.column)?;
        let params: Vec<usize> = (0..eq.values.len())
            .map(|_| {
                let current = param;
                param += 1;
                current
            })
            .collect();
        if params.len() == 1 {
            terms.push(format!("{expr} = ?{}", params[0]));
        } else {
            let list = params
                .iter()
                .map(|p| format!("?{p}"))
                .collect::<Vec<_>>()
                .join(", ");
            terms.push(format!("{expr} IN ({list})"));
        }
    }
    let chunks = sql::qualified(&vtab.schema, &format!("{}_chunk_rows", vtab.doc_name));
    let docs = sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name));
    Ok(format!(
        " AND rowid IN (SELECT c.chunk_id FROM {chunks} c JOIN {docs} d ON d.doc_id = c.doc_id WHERE {})",
        terms.join(" AND ")
    ))
}

/// One FTS5 phrase. Quotes inside the text are doubled so they stay inside it.
fn fts_phrase(query: &str) -> String {
    let mut phrase = String::with_capacity(query.len() + 2);
    phrase.push('"');
    for ch in query.chars() {
        if ch == '"' {
            phrase.push('"');
        }
        phrase.push(ch);
    }
    phrase.push('"');
    phrase
}

fn read_ids(stmt: &Stmt) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    while stmt.step()? {
        ids.push(stmt.column_i64(0));
    }
    Ok(ids)
}

fn chunk_documents(vtab: &Vtab, fused: &[(i64, f64)]) -> Result<HashMap<i64, i64>, String> {
    if fused.is_empty() {
        return Ok(HashMap::new());
    }
    let list = fused
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rows = sql::qualified(&vtab.schema, &format!("{}_chunk_rows", vtab.doc_name));
    let stmt = Stmt::prepare(
        vtab.db,
        &format!("SELECT chunk_id, doc_id FROM {rows} WHERE chunk_id IN ({list})"),
    )?;
    let mut map = HashMap::new();
    while stmt.step()? {
        map.insert(stmt.column_i64(0), stmt.column_i64(1));
    }
    Ok(map)
}

fn load_scored_chunks(vtab: &Vtab, chunks: &[FusedChunk]) -> Result<Vec<Row>, String> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }
    let list = chunks
        .iter()
        .map(|chunk| chunk.chunk_id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rows = sql::qualified(&vtab.schema, &format!("{}_chunk_rows", vtab.doc_name));
    let docs = sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name));
    let stmt = Stmt::prepare(
        vtab.db,
        &format!(
            "SELECT c.chunk_id, c.doc_id, d.source, c.start, c.end, c.heading_path, c.text, c.meta_json
             FROM {rows} c
             JOIN {docs} d ON d.doc_id = c.doc_id
             WHERE c.chunk_id IN ({list})"
        ),
    )?;
    let mut by_id = HashMap::new();
    while stmt.step()? {
        let chunk_id = stmt.column_i64(0);
        by_id.insert(
            chunk_id,
            Row::Chunk {
                chunk_id,
                doc_id: stmt.column_i64(1),
                source: stmt.column_text(2).unwrap_or_default(),
                start: stmt.column_i64(3),
                end: stmt.column_i64(4),
                heading_path: stmt.column_text(5).unwrap_or_else(|| "[]".to_string()),
                text: stmt.column_text(6).unwrap_or_default(),
                meta_json: stmt.column_text(7),
                score: None,
            },
        );
    }
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let Some(mut row) = by_id.remove(&chunk.chunk_id) else {
            continue;
        };
        if let Row::Chunk { score, .. } = &mut row {
            *score = Some(chunk.score);
        }
        out.push(row);
    }
    Ok(out)
}

fn load_scored_docs(vtab: &Vtab, docs: &[(i64, f64)]) -> Result<Vec<Row>, String> {
    if docs.is_empty() {
        return Ok(Vec::new());
    }
    let list = docs
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let documents = sql::qualified(&vtab.schema, &format!("{}_documents", vtab.doc_name));
    let stmt = Stmt::prepare(
        vtab.db,
        &format!(
            "SELECT doc_id, text, source, format, hash FROM {documents} WHERE doc_id IN ({list})"
        ),
    )?;
    let mut by_id = HashMap::new();
    while stmt.step()? {
        let doc_id = stmt.column_i64(0);
        by_id.insert(
            doc_id,
            Row::Doc {
                doc_id,
                text: stmt.column_text(1).unwrap_or_default(),
                source: stmt.column_text(2).unwrap_or_default(),
                format: stmt.column_text(3).unwrap_or_default(),
                hash: stmt.column_text(4).unwrap_or_default(),
                score: None,
            },
        );
    }
    let mut out = Vec::with_capacity(docs.len());
    for (doc_id, score) in docs {
        let Some(mut row) = by_id.remove(doc_id) else {
            continue;
        };
        if let Row::Doc { score: slot, .. } = &mut row {
            *slot = Some(*score);
        }
        out.push(row);
    }
    Ok(out)
}

unsafe extern "C" fn vtab_next(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    let cursor = unsafe { &mut *cursor.cast::<Cursor>() };
    cursor.index += 1;
    SQLITE_OK
}

unsafe extern "C" fn vtab_eof(cursor: *mut sqlite3_vtab_cursor) -> c_int {
    let cursor = unsafe { &*cursor.cast::<Cursor>() };
    i32::from(cursor.index >= cursor.rows.len())
}

unsafe extern "C" fn vtab_column(
    cursor: *mut sqlite3_vtab_cursor,
    ctx: *mut libsqlite3_sys::sqlite3_context,
    col: c_int,
) -> c_int {
    let cursor = unsafe { &*cursor.cast::<Cursor>() };
    let Some(row) = cursor.rows.get(cursor.index) else {
        return SQLITE_ERROR;
    };
    match row {
        Row::Doc {
            doc_id,
            text,
            source,
            format,
            hash,
            score,
        } => match col {
            COL_DOC_ID => result_i64(ctx, *doc_id),
            COL_TEXT => result_text(ctx, text),
            COL_SOURCE => result_text(ctx, source),
            COL_FORMAT => result_text(ctx, format),
            COL_HASH => result_text(ctx, hash),
            COL_SCORE => result_score(ctx, *score),
            _ => result_null(ctx),
        },
        Row::Chunk {
            doc_id,
            source,
            start,
            end,
            heading_path,
            text,
            meta_json,
            score,
            ..
        } => match col {
            COL_CHUNK => result_text(ctx, text),
            COL_C_DOC => result_i64(ctx, *doc_id),
            COL_C_SOURCE => result_text(ctx, source),
            COL_START => result_i64(ctx, *start),
            COL_END => result_i64(ctx, *end),
            COL_PATH => result_text(ctx, heading_path),
            COL_META => {
                if let Some(meta) = meta_json {
                    result_text(ctx, meta)
                } else {
                    result_null(ctx)
                }
            }
            COL_C_SCORE => result_score(ctx, *score),
            _ => result_null(ctx),
        },
    }
}

unsafe extern "C" fn vtab_rowid(cursor: *mut sqlite3_vtab_cursor, p_rowid: *mut i64) -> c_int {
    let cursor = unsafe { &*cursor.cast::<Cursor>() };
    let Some(row) = cursor.rows.get(cursor.index) else {
        return SQLITE_ERROR;
    };
    unsafe {
        *p_rowid = match row {
            Row::Doc { doc_id, .. } => *doc_id,
            Row::Chunk { chunk_id, .. } => *chunk_id,
        };
    }
    SQLITE_OK
}

fn result_null(ctx: *mut libsqlite3_sys::sqlite3_context) -> c_int {
    unsafe { libsqlite3_sys::sqlite3_result_null(ctx) };
    SQLITE_OK
}

fn result_i64(ctx: *mut libsqlite3_sys::sqlite3_context, value: i64) -> c_int {
    unsafe { libsqlite3_sys::sqlite3_result_int64(ctx, value) };
    SQLITE_OK
}

fn result_f64(ctx: *mut libsqlite3_sys::sqlite3_context, value: f64) -> c_int {
    unsafe { libsqlite3_sys::sqlite3_result_double(ctx, value) };
    SQLITE_OK
}

fn result_score(ctx: *mut libsqlite3_sys::sqlite3_context, score: Option<f64>) -> c_int {
    match score {
        Some(value) => result_f64(ctx, value),
        None => result_null(ctx),
    }
}

fn result_text(ctx: *mut libsqlite3_sys::sqlite3_context, value: &str) -> c_int {
    unsafe {
        libsqlite3_sys::sqlite3_result_text(
            ctx,
            value.as_ptr().cast(),
            value.len() as c_int,
            SQLITE_TRANSIENT(),
        );
    }
    SQLITE_OK
}

unsafe extern "C" fn vtab_update(
    vtab: *mut sqlite3_vtab,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
    p_rowid: *mut i64,
) -> c_int {
    guard(
        SQLITE_ERROR,
        AssertUnwindSafe(|| unsafe {
            match update(vtab, argc, argv, p_rowid) {
                Ok(()) => SQLITE_OK,
                Err(err) => {
                    set_vtab_error(vtab, &err);
                    SQLITE_ERROR
                }
            }
        }),
    )
}

unsafe fn update(
    vtab: *mut sqlite3_vtab,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
    p_rowid: *mut i64,
) -> Result<(), String> {
    let table = unsafe { &*vtab.cast::<Vtab>() };
    if table.chunks {
        return Err(format!(
            "cannot write companion table: {}_chunks",
            table.doc_name
        ));
    }
    if argc == 1 {
        let rowid =
            value_i64(unsafe { *argv }).ok_or_else(|| "delete is missing a rowid".to_string())?;
        delete_document(table, rowid)?;
        return Ok(());
    }
    // Rowid pair plus every declared column, including the hidden MATCH name.
    if argc != 9 {
        return Err("write has the wrong number of columns".to_string());
    }
    let old_rowid = value_i64(unsafe { *argv });
    let new_rowid = value_i64(unsafe { *argv.add(1) });
    let doc_id_col = value_i64(unsafe { *argv.add(2) });
    let text = value_text(unsafe { *argv.add(3) });
    let source = value_text(unsafe { *argv.add(4) });
    let format = value_text(unsafe { *argv.add(5) });
    let hash = value_text(unsafe { *argv.add(6) });
    let score_set = !value_is_null(unsafe { *argv.add(7) });
    if score_set {
        return Err("score cannot be written".to_string());
    }
    if old_rowid.is_none() {
        if new_rowid.is_some() || doc_id_col.is_some() {
            return Err("doc_id is assigned by ragite".to_string());
        }
        let text = require_text(text)?;
        let source = source.unwrap_or_default();
        let format = canonical_format(format.as_deref())?;
        let hash = hash.unwrap_or_else(|| sha256_hex(&text));
        require_model(table)?;
        let doc_id = insert_document(table, &text, &source, &format, &hash)?;
        if !p_rowid.is_null() {
            unsafe { *p_rowid = doc_id };
        }
        return Ok(());
    }
    let old_rowid = old_rowid.unwrap();
    if new_rowid.is_some_and(|id| id != old_rowid) || doc_id_col.is_some_and(|id| id != old_rowid) {
        return Err("doc_id is assigned by ragite".to_string());
    }
    let text = require_text(text)?;
    let source = source.unwrap_or_default();
    let format = canonical_format(format.as_deref())?;
    update_document(table, old_rowid, &text, &source, &format, hash)?;
    if !p_rowid.is_null() {
        unsafe { *p_rowid = old_rowid };
    }
    Ok(())
}

fn require_text(text: Option<String>) -> Result<String, String> {
    match text {
        Some(text) if !text.is_empty() => Ok(text),
        Some(_) => Err("text is empty".to_string()),
        None => Err("text is required".to_string()),
    }
}

fn require_model(table: &Vtab) -> Result<&'static BuiltinModel, String> {
    require_stored_model(table.db, &table.schema, &table.doc_name)
}

fn canonical_format(format: Option<&str>) -> Result<String, String> {
    let Some(format) = format else {
        return Ok("text/plain".to_string());
    };
    match format.to_ascii_lowercase().as_str() {
        "text/plain" => Ok("text/plain".to_string()),
        "text/markdown" => Ok("text/markdown".to_string()),
        "text/html" => Ok("text/html".to_string()),
        "application/vnd.docling.document+json" => {
            Ok("application/vnd.docling.document+json".to_string())
        }
        other => Err(format!("format is not accepted: {other}")),
    }
}

fn insert_document(
    table: &Vtab,
    text: &str,
    source: &str,
    format: &str,
    hash: &str,
) -> Result<i64, String> {
    let documents = sql::qualified(&table.schema, &format!("{}_documents", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("INSERT INTO {documents} (source, text, format, hash) VALUES (?1, ?2, ?3, ?4)"),
    )?;
    stmt.bind_text(1, source)?;
    stmt.bind_text(2, text)?;
    stmt.bind_text(3, format)?;
    stmt.bind_text(4, hash)?;
    stmt.step()?;
    let doc_id = sql::last_insert_rowid(table.db);
    write_chunks(table, doc_id, text, format)?;
    Ok(doc_id)
}

fn write_chunks(table: &Vtab, doc_id: i64, text: &str, format: &str) -> Result<(), String> {
    let model = builtin_model()?;
    let chunks = match format {
        "text/markdown" => chunk_markdown(text),
        "text/html" => chunk_html(text),
        "application/vnd.docling.document+json" => chunk_docling(text)?,
        _ => chunk_plain(text),
    };
    for chunk in chunks {
        insert_chunk(table, doc_id, &chunk, model)?;
    }
    Ok(())
}

fn insert_chunk(
    table: &Vtab,
    doc_id: i64,
    chunk: &Chunk,
    model: &BuiltinModel,
) -> Result<(), String> {
    let rows = sql::qualified(&table.schema, &format!("{}_chunk_rows", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!(
            "INSERT INTO {rows} (doc_id, start, end, heading_path, text, meta_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ),
    )?;
    stmt.bind_i64(1, doc_id)?;
    stmt.bind_i64(2, chunk.start as i64)?;
    stmt.bind_i64(3, chunk.end as i64)?;
    stmt.bind_text(4, &heading_path_json(&chunk.heading_path))?;
    stmt.bind_text(5, &chunk.text)?;
    if let Some(meta) = &chunk.meta_json {
        stmt.bind_text(6, meta)?;
    } else {
        stmt.bind_null(6)?;
    }
    stmt.step()?;
    let chunk_id = sql::last_insert_rowid(table.db);
    let blob = model.embed_blob(&chunk.text, Role::Passage);
    let vec = sql::qualified(&table.schema, &format!("{}_vec", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("INSERT INTO {vec} (rowid, embedding) VALUES (?1, ?2)"),
    )?;
    stmt.bind_i64(1, chunk_id)?;
    stmt.bind_blob(2, &blob)?;
    stmt.step()?;
    let fts = sql::qualified(&table.schema, &format!("{}_fts", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("INSERT INTO {fts} (rowid, text) VALUES (?1, ?2)"),
    )?;
    stmt.bind_i64(1, chunk_id)?;
    stmt.bind_text(2, &chunk.text)?;
    stmt.step()?;
    Ok(())
}

fn delete_document(table: &Vtab, doc_id: i64) -> Result<(), String> {
    delete_chunks(table, doc_id)?;
    let documents = sql::qualified(&table.schema, &format!("{}_documents", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("DELETE FROM {documents} WHERE doc_id = ?1"),
    )?;
    stmt.bind_i64(1, doc_id)?;
    stmt.step()?;
    Ok(())
}

fn delete_chunks(table: &Vtab, doc_id: i64) -> Result<(), String> {
    let rows = sql::qualified(&table.schema, &format!("{}_chunk_rows", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("SELECT chunk_id FROM {rows} WHERE doc_id = ?1"),
    )?;
    stmt.bind_i64(1, doc_id)?;
    let mut ids = Vec::new();
    while stmt.step()? {
        ids.push(stmt.column_i64(0));
    }
    let vec = sql::qualified(&table.schema, &format!("{}_vec", table.doc_name));
    let fts = sql::qualified(&table.schema, &format!("{}_fts", table.doc_name));
    for id in ids {
        let stmt = Stmt::prepare(table.db, &format!("DELETE FROM {vec} WHERE rowid = ?1"))?;
        stmt.bind_i64(1, id)?;
        stmt.step()?;
        let stmt = Stmt::prepare(table.db, &format!("DELETE FROM {fts} WHERE rowid = ?1"))?;
        stmt.bind_i64(1, id)?;
        stmt.step()?;
    }
    let stmt = Stmt::prepare(table.db, &format!("DELETE FROM {rows} WHERE doc_id = ?1"))?;
    stmt.bind_i64(1, doc_id)?;
    stmt.step()?;
    Ok(())
}

fn update_document(
    table: &Vtab,
    doc_id: i64,
    text: &str,
    source: &str,
    format: &str,
    hash: Option<String>,
) -> Result<(), String> {
    let documents = sql::qualified(&table.schema, &format!("{}_documents", table.doc_name));
    let stmt = Stmt::prepare(
        table.db,
        &format!("SELECT text, format, hash FROM {documents} WHERE doc_id = ?1"),
    )?;
    stmt.bind_i64(1, doc_id)?;
    if !stmt.step()? {
        return Err(format!("document was not found: {doc_id}"));
    }
    let old_text = stmt.column_text(0).unwrap_or_default();
    let old_format = stmt.column_text(1).unwrap_or_default();
    let old_hash = stmt.column_text(2).unwrap_or_default();
    let hash = hash.unwrap_or_else(|| sha256_hex(text));
    let rechunk = old_text != text || old_format != format;
    if rechunk {
        require_model(table)?;
    }
    let stmt = Stmt::prepare(
        table.db,
        &format!(
            "UPDATE {documents} SET source = ?1, text = ?2, format = ?3, hash = ?4 WHERE doc_id = ?5"
        ),
    )?;
    stmt.bind_text(1, source)?;
    stmt.bind_text(2, text)?;
    stmt.bind_text(3, format)?;
    stmt.bind_text(4, &hash)?;
    stmt.bind_i64(5, doc_id)?;
    stmt.step()?;
    if rechunk {
        delete_chunks(table, doc_id)?;
        write_chunks(table, doc_id, text, format)?;
    }
    let _ = old_hash;
    Ok(())
}

fn value_is_null(value: *mut sqlite3_value) -> bool {
    value.is_null() || unsafe { sqlite3_value_type(value) } == SQLITE_NULL
}

fn value_i64(value: *mut sqlite3_value) -> Option<i64> {
    if value_is_null(value) {
        return None;
    }
    let kind = unsafe { sqlite3_value_type(value) };
    if kind == SQLITE_INTEGER || kind == SQLITE_FLOAT {
        Some(unsafe { sqlite3_value_int64(value) })
    } else {
        None
    }
}

pub(crate) fn value_text(value: *mut sqlite3_value) -> Option<String> {
    if value_is_null(value) {
        return None;
    }
    unsafe {
        let kind = sqlite3_value_type(value);
        if kind != SQLITE_TEXT
            && kind != libsqlite3_sys::SQLITE_BLOB
            && kind != SQLITE_INTEGER
            && kind != SQLITE_FLOAT
        {
            return None;
        }
        // sqlite3_value_text applies SQLite's own numeric-to-text conversion.
        // sqlite3_value_int64 would drop the fraction of a REAL (3.7 -> 3).
        let ptr = sqlite3_value_text(value);
        if ptr.is_null() {
            return None;
        }
        let n = sqlite3_value_bytes(value) as usize;
        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), n);
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}
