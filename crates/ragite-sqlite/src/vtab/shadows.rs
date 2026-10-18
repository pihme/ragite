//! Shadow tables of one Document table.
//! A Chunk write or delete touches `chunk_rows`, `vec`, and `fts` together.
//! Scan and load-by-id share one projection. The three tables stay three tables.

use std::collections::HashMap;

use libsqlite3_sys::sqlite3;
use ragite_core::{BuiltinModel, Chunk, Role, builtin_model, heading_path_json};

use super::Row;
use crate::sql::{self, Stmt};

/// One `SELECT` list for a Chunk row joined to its Document.
const CHUNK_PROJECTION: &str =
    "c.chunk_id, c.doc_id, d.source, c.start, c.end, c.heading_path, c.text, c.meta_json";
/// One `SELECT` list for a Document row.
const DOC_PROJECTION: &str = "doc_id, text, source, format, hash";

struct Bare {
    chunks: String,
    documents: String,
    chunk_rows: String,
    vec_table: String,
    fts: String,
    model: String,
}

/// Qualified names. `fts_match` is the quoted bare FTS name used by `MATCH` and `bm25`.
pub(super) struct Tables {
    pub(super) documents: String,
    pub(super) chunk_rows: String,
    pub(super) vec_table: String,
    pub(super) fts: String,
    pub(super) fts_match: String,
    pub(super) model: String,
    /// Bare companion name, `<table>_chunks`.
    pub(super) companion: String,
}

fn bare(table: &str) -> Bare {
    Bare {
        chunks: format!("{table}_chunks"),
        documents: format!("{table}_documents"),
        chunk_rows: format!("{table}_chunk_rows"),
        vec_table: format!("{table}_vec"),
        fts: format!("{table}_fts"),
        model: format!("{table}_model"),
    }
}

pub(super) fn tables(schema: &str, table: &str) -> Tables {
    let names = bare(table);
    Tables {
        documents: sql::qualified(schema, &names.documents),
        chunk_rows: sql::qualified(schema, &names.chunk_rows),
        vec_table: sql::qualified(schema, &names.vec_table),
        fts: sql::qualified(schema, &names.fts),
        fts_match: sql::quote_ident(&names.fts),
        model: sql::qualified(schema, &names.model),
        companion: names.chunks,
    }
}

/// Bare names checked before `CREATE`, in the order `create_shadows` builds them.
pub(super) fn occupied_names(table: &str) -> [String; 6] {
    let names = bare(table);
    [
        names.chunks,
        names.documents,
        names.chunk_rows,
        names.vec_table,
        names.fts,
        names.model,
    ]
}

pub(super) fn create_shadows(
    db: *mut sqlite3,
    schema: &str,
    table: &str,
    model_id: &str,
) -> Result<(), String> {
    let model = builtin_model()?;
    let names = tables(schema, table);
    let companion = sql::qualified(schema, &names.companion);
    sql::exec(
        db,
        &format!(
            "CREATE TABLE {documents} (
                doc_id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                text TEXT NOT NULL,
                format TEXT NOT NULL,
                hash TEXT NOT NULL
            )",
            documents = names.documents,
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
            )",
            chunks = names.chunk_rows,
        ),
    )?;
    sql::exec(
        db,
        &format!("CREATE TABLE {model} (technical_id TEXT NOT NULL)", model = names.model),
    )?;
    let insert = Stmt::prepare(
        db,
        &format!("INSERT INTO {model} (technical_id) VALUES (?1)", model = names.model),
    )?;
    insert.bind_text(1, model_id)?;
    insert.step()?;
    sql::exec(
        db,
        &format!(
            "CREATE VIRTUAL TABLE {vec} USING vec0(embedding float[{}] distance_metric=cosine)",
            model.dimension(),
            vec = names.vec_table,
        ),
    )?;
    sql::exec(
        db,
        &format!(
            "CREATE VIRTUAL TABLE {fts} USING fts5(text, tokenize = 'unicode61', content='', contentless_delete=1)",
            fts = names.fts,
        ),
    )?;
    let parent = super::args_parent_literal(table);
    let key = super::drop_key(schema, &names.companion);
    super::ALLOW_CREATE
        .lock()
        .expect("create set")
        .insert(key.clone());
    let created = sql::exec(db, &super::companion_create_sql(&companion, &parent));
    super::ALLOW_CREATE.lock().expect("create set").remove(&key);
    created?;
    Ok(())
}

pub(super) fn destroy_shadows(db: *mut sqlite3, schema: &str, table: &str) -> Result<(), String> {
    let names = bare(table);
    let key = super::drop_key(schema, &names.chunks);
    super::ALLOW_DROP
        .lock()
        .expect("drop set")
        .insert(key.clone());
    let drop_companion = sql::exec(
        db,
        &format!(
            "DROP TABLE IF EXISTS {}",
            sql::qualified(schema, &names.chunks)
        ),
    );
    super::ALLOW_DROP.lock().expect("drop set").remove(&key);
    drop_companion?;
    for name in [
        &names.vec_table,
        &names.fts,
        &names.chunk_rows,
        &names.documents,
        &names.model,
    ] {
        sql::exec(
            db,
            &format!("DROP TABLE IF EXISTS {}", sql::qualified(schema, name)),
        )?;
    }
    Ok(())
}

pub(super) fn insert_chunk(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    doc_id: i64,
    chunk: &Chunk,
    model: &BuiltinModel,
) -> Result<(), String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(
        db,
        &format!(
            "INSERT INTO {rows} (doc_id, start, end, heading_path, text, meta_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rows = names.chunk_rows,
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
    let chunk_id = sql::last_insert_rowid(db);
    let blob = model.embed_blob(&chunk.text, Role::Passage);
    let stmt = Stmt::prepare(
        db,
        &format!("INSERT INTO {vec} (rowid, embedding) VALUES (?1, ?2)", vec = names.vec_table),
    )?;
    stmt.bind_i64(1, chunk_id)?;
    stmt.bind_blob(2, &blob)?;
    stmt.step()?;
    let stmt = Stmt::prepare(
        db,
        &format!("INSERT INTO {fts} (rowid, text) VALUES (?1, ?2)", fts = names.fts),
    )?;
    stmt.bind_i64(1, chunk_id)?;
    stmt.bind_text(2, &chunk.text)?;
    stmt.step()?;
    Ok(())
}

pub(super) fn delete_chunks(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    doc_id: i64,
) -> Result<(), String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(
        db,
        &format!("SELECT chunk_id FROM {rows} WHERE doc_id = ?1", rows = names.chunk_rows),
    )?;
    stmt.bind_i64(1, doc_id)?;
    let mut ids = Vec::new();
    while stmt.step()? {
        ids.push(stmt.column_i64(0));
    }
    for id in ids {
        let stmt = Stmt::prepare(
            db,
            &format!("DELETE FROM {vec} WHERE rowid = ?1", vec = names.vec_table),
        )?;
        stmt.bind_i64(1, id)?;
        stmt.step()?;
        let stmt = Stmt::prepare(
            db,
            &format!("DELETE FROM {fts} WHERE rowid = ?1", fts = names.fts),
        )?;
        stmt.bind_i64(1, id)?;
        stmt.step()?;
    }
    let stmt = Stmt::prepare(
        db,
        &format!("DELETE FROM {rows} WHERE doc_id = ?1", rows = names.chunk_rows),
    )?;
    stmt.bind_i64(1, doc_id)?;
    stmt.step()?;
    Ok(())
}

pub(super) fn scan_chunks(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
) -> Result<Vec<Row>, String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(db, &chunk_sql(&names, "ORDER BY c.doc_id, c.chunk_id"))?;
    read_all(&stmt, read_chunk)
}

pub(super) fn chunk_by_id(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    chunk_id: i64,
) -> Result<Vec<Row>, String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(db, &chunk_sql(&names, "WHERE c.chunk_id = ?1"))?;
    stmt.bind_i64(1, chunk_id)?;
    read_one(&stmt, read_chunk)
}

/// Rows in `chunk_ids` order. A missing id is skipped. Score stays unset.
pub(super) fn chunks_for_ids(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    chunk_ids: &[i64],
) -> Result<Vec<Row>, String> {
    if chunk_ids.is_empty() {
        return Ok(Vec::new());
    }
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(
        db,
        &chunk_sql(&names, &format!("WHERE c.chunk_id IN ({})", id_list(chunk_ids))),
    )?;
    rows_in_order(&stmt, chunk_ids, read_chunk, chunk_id_of)
}

pub(super) fn scan_documents(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
) -> Result<Vec<Row>, String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(db, &doc_sql(&names, "ORDER BY doc_id"))?;
    read_all(&stmt, read_doc)
}

pub(super) fn document_by_id(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    doc_id: i64,
) -> Result<Vec<Row>, String> {
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(db, &doc_sql(&names, "WHERE doc_id = ?1"))?;
    stmt.bind_i64(1, doc_id)?;
    read_one(&stmt, read_doc)
}

/// Rows in `doc_ids` order. A missing id is skipped. Score stays unset.
pub(super) fn documents_for_ids(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    doc_ids: &[i64],
) -> Result<Vec<Row>, String> {
    if doc_ids.is_empty() {
        return Ok(Vec::new());
    }
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(
        db,
        &doc_sql(&names, &format!("WHERE doc_id IN ({})", id_list(doc_ids))),
    )?;
    rows_in_order(&stmt, doc_ids, read_doc, doc_id_of)
}

pub(super) fn chunk_doc_ids(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    chunk_ids: &[i64],
) -> Result<HashMap<i64, i64>, String> {
    if chunk_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let names = tables(schema, doc_name);
    let stmt = Stmt::prepare(
        db,
        &format!(
            "SELECT chunk_id, doc_id FROM {rows} WHERE chunk_id IN ({list})",
            rows = names.chunk_rows,
            list = id_list(chunk_ids),
        ),
    )?;
    let mut map = HashMap::new();
    while stmt.step()? {
        map.insert(stmt.column_i64(0), stmt.column_i64(1));
    }
    Ok(map)
}

fn chunk_sql(names: &Tables, tail: &str) -> String {
    format!(
        "SELECT {CHUNK_PROJECTION} FROM {chunks} c JOIN {docs} d ON d.doc_id = c.doc_id {tail}",
        chunks = names.chunk_rows,
        docs = names.documents,
    )
}

fn doc_sql(names: &Tables, tail: &str) -> String {
    format!(
        "SELECT {DOC_PROJECTION} FROM {documents} {tail}",
        documents = names.documents,
    )
}

fn id_list(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn read_chunk(stmt: &Stmt) -> Row {
    Row::Chunk {
        chunk_id: stmt.column_i64(0),
        doc_id: stmt.column_i64(1),
        source: stmt.column_text(2).unwrap_or_default(),
        start: stmt.column_i64(3),
        end: stmt.column_i64(4),
        heading_path: stmt.column_text(5).unwrap_or_else(|| "[]".to_string()),
        text: stmt.column_text(6).unwrap_or_default(),
        meta_json: stmt.column_text(7),
        score: None,
    }
}

fn read_doc(stmt: &Stmt) -> Row {
    Row::Doc {
        doc_id: stmt.column_i64(0),
        text: stmt.column_text(1).unwrap_or_default(),
        source: stmt.column_text(2).unwrap_or_default(),
        format: stmt.column_text(3).unwrap_or_default(),
        hash: stmt.column_text(4).unwrap_or_default(),
        score: None,
    }
}

fn read_all(stmt: &Stmt, read: fn(&Stmt) -> Row) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    while stmt.step()? {
        rows.push(read(stmt));
    }
    Ok(rows)
}

fn read_one(stmt: &Stmt, read: fn(&Stmt) -> Row) -> Result<Vec<Row>, String> {
    if !stmt.step()? {
        return Ok(Vec::new());
    }
    Ok(vec![read(stmt)])
}

fn chunk_id_of(row: &Row) -> i64 {
    match row {
        Row::Chunk { chunk_id, .. } => *chunk_id,
        Row::Doc { .. } => 0,
    }
}

fn doc_id_of(row: &Row) -> i64 {
    match row {
        Row::Doc { doc_id, .. } => *doc_id,
        Row::Chunk { .. } => 0,
    }
}

fn rows_in_order(
    stmt: &Stmt,
    ids: &[i64],
    read: fn(&Stmt) -> Row,
    id_of: fn(&Row) -> i64,
) -> Result<Vec<Row>, String> {
    let mut by_id = HashMap::new();
    while stmt.step()? {
        let row = read(stmt);
        by_id.insert(id_of(&row), row);
    }
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(row) = by_id.remove(id) else {
            continue;
        };
        out.push(row);
    }
    Ok(out)
}
