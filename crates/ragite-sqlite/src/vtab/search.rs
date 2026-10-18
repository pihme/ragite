//! MATCH on a Document table or its companion.
//! The virtual-table callbacks pass the query, the usable equalities, and the
//! surface. This module encodes that plan into `idxStr` and decodes it.

use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_int};

use libsqlite3_sys::{
    SQLITE_DONE, SQLITE_NULL, SQLITE_OK, sqlite3, sqlite3_value, sqlite3_value_dup,
    sqlite3_value_free, sqlite3_value_type,
};
use ragite_core::{
    FUSION_DEPTH, FusedChunk, Role, best_document_scores, order_fused_chunks,
    reciprocal_rank_fusion,
};

use super::{
    COL_CHUNK, COL_C_DOC, COL_C_SOURCE, COL_DOC_ID, COL_END, COL_FORMAT, COL_HASH, COL_META,
    COL_PATH, COL_SOURCE, COL_START, COL_TEXT, Row,
};
use crate::sql::Stmt;

unsafe extern "C" {
    fn ragite_vtab_in_first(value: *mut sqlite3_value, out: *mut *mut sqlite3_value) -> c_int;
    fn ragite_vtab_in_next(value: *mut sqlite3_value, out: *mut *mut sqlite3_value) -> c_int;
}

/// Document table, or the read-only companion that lists its Chunks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Surface {
    Document,
    Companion,
}

/// One `=` or `IN` that SQLite marked usable.
#[derive(Clone, Copy)]
pub(super) struct UsableEq {
    pub(super) column: c_int,
    pub(super) in_list: bool,
}

/// Consumed equalities, and the `idxStr` that carries them to `xFilter`.
pub(super) struct MatchPlan {
    /// Indexes into the offered list, in argv order after the MATCH argument.
    pub(super) consumed: Vec<usize>,
    /// Comma-separated column list. `None` when the search is not narrowed.
    pub(super) idx_str: Option<String>,
    pub(super) estimated_cost: f64,
    pub(super) estimated_rows: i64,
}

struct Table<'a> {
    db: *mut sqlite3,
    schema: &'a str,
    doc_name: &'a str,
    surface: Surface,
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

/// Columns whose `=` or `IN` can select Chunks before the depth is counted.
/// `score` is omitted on both tables. The companion does not expose `format` or `hash`.
pub(super) fn narrows(surface: Surface, column: c_int) -> bool {
    if surface == Surface::Companion {
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

pub(super) fn plan(surface: Surface, offered: &[UsableEq]) -> MatchPlan {
    let mut consumed = Vec::new();
    let mut columns = Vec::new();
    for (index, eq) in offered.iter().enumerate() {
        if !narrows(surface, eq.column) {
            continue;
        }
        consumed.push(index);
        let token = if eq.in_list {
            format!("i{}", eq.column)
        } else {
            eq.column.to_string()
        };
        columns.push(token);
    }
    // The unconstrained plan must cost more than one that also consumes a
    // column constraint. When the costs are equal, SQLite keeps only the
    // plan with fewer tables, and that plan is the unconstrained MATCH.
    // An inner join then misses rows that the same join returns as a
    // CROSS JOIN (REQ-102).
    let estimated_cost = if columns.is_empty() {
        1_000_000.0
    } else {
        10.0
    };
    let idx_str = if columns.is_empty() {
        None
    } else {
        Some(columns.join(","))
    };
    MatchPlan {
        consumed,
        idx_str,
        estimated_cost,
        estimated_rows: FUSION_DEPTH as i64,
    }
}

/// Ordered rows for one MATCH. `idx_str` is null when the search is not narrowed.
/// `argv[0]` is the query value; the following arguments are the consumed equalities.
pub(super) fn rows(
    db: *mut sqlite3,
    schema: &str,
    doc_name: &str,
    surface: Surface,
    query: &str,
    idx_str: *const c_char,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> Result<Vec<Row>, String> {
    let eqs = match pushed_eqs(idx_str, argc, argv)? {
        Some(eqs) => eqs,
        None => return Ok(Vec::new()),
    };
    let table = Table {
        db,
        schema,
        doc_name,
        surface,
    };
    search_rows(&table, query, &eqs)
}

fn search_rows(table: &Table<'_>, query: &str, eqs: &[PushedTerm]) -> Result<Vec<Row>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let model = super::require_stored_model(table.db, table.schema, table.doc_name)?;
    let blob = model.embed_blob(query, Role::Query);
    let depth = FUSION_DEPTH as i64;
    let vector = vector_ids(table, &blob, depth, eqs)?;
    let full_text = fts_ids(table, query, depth, eqs)?;
    let fused = reciprocal_rank_fusion(&vector, &full_text);
    let chunk_ids: Vec<i64> = fused.iter().map(|(id, _)| *id).collect();
    let doc_of = super::shadows::chunk_doc_ids(table.db, table.schema, table.doc_name, &chunk_ids)?;
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
    if table.surface == Surface::Companion {
        order_fused_chunks(&mut chunks);
        load_scored_chunks(table, &chunks)
    } else {
        load_scored_docs(table, &best_document_scores(&chunks))
    }
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

fn vector_ids(
    table: &Table<'_>,
    blob: &[u8],
    depth: i64,
    eqs: &[PushedTerm],
) -> Result<Vec<i64>, String> {
    let vec_name = super::shadows::tables(table.schema, table.doc_name).vec_table;
    let restrict = chunk_id_restriction(table, eqs)?;
    let stmt = Stmt::prepare(
        table.db,
        &format!(
            "SELECT rowid FROM {vec_name} WHERE embedding MATCH ?1 AND k = ?2{restrict} ORDER BY distance"
        ),
    )?;
    stmt.bind_blob(1, blob)?;
    stmt.bind_i64(2, depth)?;
    bind_eqs(&stmt, eqs)?;
    read_ids(&stmt)
}

fn fts_ids(
    table: &Table<'_>,
    query: &str,
    depth: i64,
    eqs: &[PushedTerm],
) -> Result<Vec<i64>, String> {
    let names = super::shadows::tables(table.schema, table.doc_name);
    let from = names.fts;
    let name = names.fts_match;
    let restrict = chunk_id_restriction(table, eqs)?;
    let stmt = Stmt::prepare(
        table.db,
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

fn column_expr(surface: Surface, column: c_int) -> Result<&'static str, String> {
    let chunks = surface == Surface::Companion;
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
fn chunk_id_restriction(table: &Table<'_>, eqs: &[PushedTerm]) -> Result<String, String> {
    if eqs.is_empty() {
        return Ok(String::new());
    }
    let mut param = 3usize;
    let mut terms = Vec::with_capacity(eqs.len());
    for eq in eqs {
        let expr = column_expr(table.surface, eq.column)?;
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
    let names = super::shadows::tables(table.schema, table.doc_name);
    let chunks = names.chunk_rows;
    let docs = names.documents;
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

fn load_scored_chunks(table: &Table<'_>, chunks: &[FusedChunk]) -> Result<Vec<Row>, String> {
    let ids: Vec<i64> = chunks.iter().map(|chunk| chunk.chunk_id).collect();
    let rows = super::shadows::chunks_for_ids(table.db, table.schema, table.doc_name, &ids)?;
    let mut by_id = HashMap::new();
    for row in rows {
        if let Row::Chunk { chunk_id, .. } = &row {
            by_id.insert(*chunk_id, row);
        }
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

fn load_scored_docs(table: &Table<'_>, docs: &[(i64, f64)]) -> Result<Vec<Row>, String> {
    let ids: Vec<i64> = docs.iter().map(|(id, _)| *id).collect();
    let rows = super::shadows::documents_for_ids(table.db, table.schema, table.doc_name, &ids)?;
    let mut by_id = HashMap::new();
    for row in rows {
        if let Row::Doc { doc_id, .. } = &row {
            by_id.insert(*doc_id, row);
        }
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
