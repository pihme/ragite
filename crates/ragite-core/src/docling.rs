//! Cut a DoclingDocument JSON object into Chunks.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::Value;

use crate::chunk::{Chunk, split_range};

/// Cut a DoclingDocument into Chunks. The error names the failed check.
pub fn chunk_docling(text: &str) -> Result<Vec<Chunk>, String> {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => return Err("the text is not JSON".to_string()),
    };
    let Some(object) = value.as_object() else {
        return Err("the JSON value is not an object".to_string());
    };
    let Some(schema_name) = object.get("schema_name") else {
        return Err("schema_name is missing".to_string());
    };
    if schema_name.as_str() != Some("DoclingDocument") {
        return Err(format!(
            "schema_name is not DoclingDocument: {}",
            schema_value(schema_name)
        ));
    }
    Ok(walk(&value))
}

fn schema_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn walk(root: &Value) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut seen = HashSet::new();
    let mut headings = BTreeMap::new();
    // Body is walked before furniture. A ref reached twice is emitted once.
    walk_ref(root, "#/body", false, &mut headings, &mut seen, &mut chunks);
    let mut furniture_headings = BTreeMap::new();
    walk_ref(
        root,
        "#/furniture",
        true,
        &mut furniture_headings,
        &mut seen,
        &mut chunks,
    );
    chunks
}

fn walk_ref(
    root: &Value,
    cref: &str,
    furniture: bool,
    headings: &mut BTreeMap<i64, String>,
    seen: &mut HashSet<String>,
    chunks: &mut Vec<Chunk>,
) {
    if !seen.insert(cref.to_string()) {
        return;
    }
    let Some(node) = resolve(root, cref) else {
        return;
    };
    let path = heading_path_for(cref, node, furniture, headings);
    if let Some(text) = item_text(cref, node) {
        if !text.is_empty() {
            let meta = meta_for(node, cref, &path, furniture);
            push_pieces(chunks, &text, &path, &meta);
        }
    }
    for child in child_refs(node) {
        walk_ref(root, &child, furniture, headings, seen, chunks);
    }
}

/// Titles are level 0. A section header uses its `level`, or 1 when that
/// field is absent. A heading applies until a later heading of the same
/// level or a level above it (a smaller or equal level number).
fn heading_path_for(
    cref: &str,
    node: &Value,
    furniture: bool,
    headings: &mut BTreeMap<i64, String>,
) -> Vec<String> {
    if furniture {
        return Vec::new();
    }
    let Some(level) = heading_level(cref, node) else {
        return headings.values().cloned().collect();
    };
    headings.retain(|key, _| *key < level);
    let path = headings.values().cloned().collect();
    if let Some(title) = node.get("text").and_then(Value::as_str)
        && !title.is_empty()
    {
        headings.insert(level, title.to_string());
    }
    path
}

fn heading_level(cref: &str, node: &Value) -> Option<i64> {
    if collection(cref) != Some("texts") {
        return None;
    }
    match node.get("label").and_then(Value::as_str) {
        Some("title") => Some(0),
        Some("section_header") => Some(node.get("level").and_then(json_i64).unwrap_or(1)),
        _ => None,
    }
}

fn item_text(cref: &str, node: &Value) -> Option<String> {
    match collection(cref) {
        Some("texts") => Some(
            node.get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        ),
        Some("tables") => Some(table_text(node)),
        Some("key_value_items") => Some(kv_text(node)),
        _ => None,
    }
}

fn push_pieces(chunks: &mut Vec<Chunk>, text: &str, path: &[String], meta: &str) {
    let chars: Vec<char> = text.chars().collect();
    for (start, end) in split_range(0, chars.len()) {
        chunks.push(Chunk {
            text: chars[start..end].iter().collect(),
            start,
            end,
            heading_path: path.to_vec(),
            meta_json: Some(meta.to_string()),
        });
    }
}

fn meta_for(node: &Value, cref: &str, path: &[String], furniture: bool) -> String {
    let section = if furniture {
        None
    } else {
        path.last().map(String::as_str)
    };
    meta_json(page_no(node), section, element_ref(node, cref))
}

fn meta_json(page: Option<i64>, section: Option<&str>, element: &str) -> String {
    let page = match page {
        Some(page) => page.to_string(),
        None => "null".to_string(),
    };
    let section = match section {
        Some(section) => Value::from(section).to_string(),
        None => "null".to_string(),
    };
    let element = Value::from(element).to_string();
    format!("{{\"page\":{page},\"section\":{section},\"element\":{element}}}")
}

fn page_no(node: &Value) -> Option<i64> {
    let prov = node.get("prov").and_then(Value::as_array)?;
    prov.first()?.get("page_no").and_then(json_i64)
}

fn element_ref<'a>(node: &'a Value, cref: &'a str) -> &'a str {
    node.get("self_ref")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .unwrap_or(cref)
}

fn table_text(node: &Value) -> String {
    let Some(data) = node.get("data") else {
        return String::new();
    };
    let cells = data
        .get("table_cells")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let declared_rows = data.get("num_rows").and_then(json_usize).unwrap_or(0);
    let declared_cols = data.get("num_cols").and_then(json_usize).unwrap_or(0);
    let positioned = cells.iter().any(|cell| cell_pos(cell).is_some());
    let seq_cols = if declared_cols == 0 {
        cells.len().max(1)
    } else {
        declared_cols
    };
    let mut placed = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let text = cell.get("text").and_then(Value::as_str).unwrap_or("");
        let Some((row, col)) = (if positioned {
            cell_pos(cell)
        } else {
            Some((index / seq_cols, index % seq_cols))
        }) else {
            continue;
        };
        placed.push((row, col, text));
    }
    let mut rows = declared_rows;
    let mut cols = declared_cols;
    for (row, col, _) in &placed {
        rows = rows.max(row + 1);
        cols = cols.max(col + 1);
    }
    if rows == 0 || cols == 0 {
        return String::new();
    }
    let mut grid = vec![vec![""; cols]; rows];
    for (row, col, text) in placed {
        if row < rows && col < cols {
            grid[row][col] = text;
        }
    }
    let mut lines = Vec::with_capacity(rows);
    for row in grid {
        lines.push(row.join("\t"));
    }
    lines.join("\n")
}

fn cell_pos(cell: &Value) -> Option<(usize, usize)> {
    Some((
        cell.get("start_row_offset_idx").and_then(json_usize)?,
        cell.get("start_col_offset_idx").and_then(json_usize)?,
    ))
}

fn kv_text(node: &Value) -> String {
    let Some(graph) = node.get("graph") else {
        return String::new();
    };
    let cells = graph
        .get("cells")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut by_id: HashMap<i64, String> = HashMap::new();
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for cell in cells {
        let text = cell
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(id) = cell.get("cell_id").and_then(json_i64) {
            by_id.insert(id, text.clone());
        }
        match cell.get("label").and_then(Value::as_str) {
            Some("key") => keys.push(text),
            Some("value") => values.push(text),
            _ => {}
        }
    }
    let mut lines = Vec::new();
    if let Some(links) = graph.get("links").and_then(Value::as_array) {
        for link in links {
            let Some(label) = link.get("label").and_then(Value::as_str) else {
                continue;
            };
            let source = link.get("source_cell_id").and_then(json_i64);
            let target = link.get("target_cell_id").and_then(json_i64);
            let (key_id, value_id) = match label {
                "to_value" => (source, target),
                "to_key" => (target, source),
                _ => continue,
            };
            let key = lookup(&by_id, key_id);
            let value = lookup(&by_id, value_id);
            lines.push(format!("{key}\t{value}"));
        }
    }
    if lines.is_empty() {
        for (key, value) in keys.iter().zip(values.iter()) {
            lines.push(format!("{key}\t{value}"));
        }
    }
    lines.join("\n")
}

fn lookup(by_id: &HashMap<i64, String>, id: Option<i64>) -> String {
    id.and_then(|id| by_id.get(&id).cloned())
        .unwrap_or_default()
}

fn collection(cref: &str) -> Option<&str> {
    let path = cref.strip_prefix("#/")?;
    let first = path.split('/').next().unwrap_or("");
    if first.is_empty() { None } else { Some(first) }
}

fn resolve<'a>(root: &'a Value, cref: &str) -> Option<&'a Value> {
    let path = cref.strip_prefix("#/")?;
    if path.is_empty() {
        return None;
    }
    let mut current = root;
    for segment in path.split('/') {
        let segment = decode_pointer(segment);
        current = match current {
            Value::Object(map) => map.get(&segment)?,
            Value::Array(items) => {
                let index: usize = segment.parse().ok()?;
                items.get(index)?
            }
            _ => return None,
        };
    }
    Some(current)
}

fn decode_pointer(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut chars = segment.chars();
    while let Some(ch) = chars.next() {
        if ch == '~' {
            match chars.next() {
                Some('0') => out.push('~'),
                Some('1') => out.push('/'),
                Some(other) => {
                    out.push('~');
                    out.push(other);
                }
                None => out.push('~'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn child_refs(node: &Value) -> Vec<String> {
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return Vec::new();
    };
    children.iter().filter_map(child_ref).collect()
}

fn child_ref(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    let object = value.as_object()?;
    object
        .get("$ref")
        .or_else(|| object.get("cref"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn json_i64(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    value.as_u64().and_then(|number| i64::try_from(number).ok())
}

fn json_usize(value: &Value) -> Option<usize> {
    json_i64(value).and_then(|number| usize::try_from(number).ok())
}

#[cfg(test)]
mod tests {
    use super::chunk_docling;
    use crate::Chunk;

    fn cut(text: &str) -> Vec<Chunk> {
        chunk_docling(text).unwrap_or_else(|err| panic!("chunk failed: {err}"))
    }

    fn texts(chunks: &[Chunk]) -> Vec<&str> {
        chunks.iter().map(|chunk| chunk.text.as_str()).collect()
    }

    #[test]
    fn tc_151_to_154_docling_rejects_bad_json() {
        let text = "not json";
        let err = chunk_docling(text).unwrap_err();
        assert!(err.contains("the text is not JSON"), "{err}");
        assert!(!err.contains(text), "{err}");

        let array = "[]";
        let err = chunk_docling(array).unwrap_err();
        assert!(err.contains("the JSON value is not an object"), "{err}");
        assert!(!err.contains(array), "{err}");

        let quoted = "\"DoclingDocument\"";
        let err = chunk_docling(quoted).unwrap_err();
        assert!(err.contains("the JSON value is not an object"), "{err}");
        assert!(!err.contains("DoclingDocument"), "{err}");

        let missing = "{}";
        let err = chunk_docling(missing).unwrap_err();
        assert!(err.contains("schema_name is missing"), "{err}");
        assert!(!err.contains(missing), "{err}");

        let other = r#"{"schema_name":"Other"}"#;
        let err = chunk_docling(other).unwrap_err();
        assert!(err.contains("schema_name is not DoclingDocument"), "{err}");
        assert!(err.contains("Other"), "{err}");
        assert!(!err.contains(other), "{err}");

        let err = chunk_docling(r#"{"schema_name":"doclingdocument"}"#).unwrap_err();
        assert!(err.contains("doclingdocument"), "{err}");

        let err = chunk_docling(r#"{"schema_name":null}"#).unwrap_err();
        assert!(err.contains("schema_name is not DoclingDocument"), "{err}");
        assert!(err.contains("null"), "{err}");
        assert!(!err.contains("schema_name is missing"), "{err}");

        let err = chunk_docling(r#"{"schema_name":1}"#).unwrap_err();
        assert!(
            err.contains("schema_name is not DoclingDocument: 1"),
            "{err}"
        );
        assert!(!err.contains(r#"{"schema_name":1}"#), "{err}");
    }

    #[test]
    fn tc_155_tc_156_missing_trees_and_extra_fields_store_no_chunks() {
        assert!(cut(r#"{"schema_name":"DoclingDocument"}"#).is_empty());
        assert!(
            cut(r#"{"schema_name":"DoclingDocument","version":"9.9.9","extra":true}"#).is_empty()
        );
        assert!(
            cut(r#"{"schema_name":"DoclingDocument","body":null,"furniture":null}"#).is_empty()
        );
    }

    #[test]
    fn tc_132_tc_133_tc_134_tc_137_empty_text_is_skipped_and_whitespace_is_kept() {
        let only_empty = r##"{"schema_name":"DoclingDocument","texts":[{"self_ref":"#/texts/0","label":"text","text":""}],"body":{"children":[{"$ref":"#/texts/0"}]}}"##;
        assert!(cut(only_empty).is_empty());

        let doc = r##"{"schema_name":"DoclingDocument","texts":[
            {"self_ref":"#/texts/0","label":"text","text":""},
            {"self_ref":"#/texts/1","label":"text","text":"Hello","orig":"ORIG"},
            {"self_ref":"#/texts/2","label":"text","text":" "},
            {"self_ref":"#/texts/3","label":"text","text":""}
        ],"body":{"children":[{"$ref":"#/texts/0"},{"$ref":"#/texts/1"},{"$ref":"#/texts/2"}]},
        "furniture":{"children":[{"$ref":"#/texts/3"}]}}"##;
        let chunks = cut(doc);
        assert_eq!(texts(&chunks), vec!["Hello", " "]);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 5);
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/texts/1"}"##)
        );
        assert_eq!(chunks[1].text, " ");
        assert_eq!((chunks[1].start, chunks[1].end), (0, 1));
    }

    #[test]
    fn tc_48_tc_142_text_field_offsets_count_scalars_in_the_item() {
        let doc = r##"{"schema_name":"DoclingDocument","texts":[{"self_ref":"#/texts/3","label":"text","text":"a😀b","orig":"NO"}],"body":{"children":[{"$ref":"#/texts/0"}]}}"##;
        let chunks = cut(doc);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "a😀b");
        assert_eq!((chunks[0].start, chunks[0].end), (0, 3));
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/texts/3"}"##)
        );
    }

    #[test]
    fn tc_143_table_cells_join_in_reading_order_and_caption_stays_separate() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [
                {"self_ref": "#/texts/0", "label": "caption", "text": "Caption"},
                {"self_ref": "#/texts/1", "label": "caption", "text": "NotInTree"}
            ],
            "tables": [{
                "self_ref": "#/tables/0",
                "label": "table",
                "prov": [{"page_no": 2}, {"page_no": 8}],
                "captions": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}],
                "children": [{"$ref": "#/texts/0"}],
                "data": {
                    "num_rows": 2,
                    "num_cols": 2,
                    "table_cells": [
                        {"text": "D", "start_row_offset_idx": 1, "start_col_offset_idx": 1},
                        {"text": "A", "start_row_offset_idx": 0, "start_col_offset_idx": 0},
                        {"text": "", "start_row_offset_idx": 0, "start_col_offset_idx": 1},
                        {"text": "C", "start_row_offset_idx": 1, "start_col_offset_idx": 0}
                    ]
                }
            }],
            "body": {"children": [{"$ref": "#/tables/0"}]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(texts(&chunks), vec!["A\t\nC\tD", "Caption"]);
        assert_eq!((chunks[0].start, chunks[0].end), (0, 6));
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":2,"section":null,"element":"#/tables/0"}"##)
        );
        assert_eq!(
            chunks[1].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/texts/0"}"##)
        );
    }

    #[test]
    fn tc_144_key_value_item_is_one_line_per_pair() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "key_value_items": [{
                "self_ref": "#/key_value_items/0",
                "label": "key_value_region",
                "graph": {
                    "cells": [
                        {"cell_id": 0, "label": "key", "text": "role"},
                        {"cell_id": 1, "label": "value", "text": "admin"},
                        {"cell_id": 2, "label": "key", "text": "name"},
                        {"cell_id": 3, "label": "value", "text": "ada"}
                    ],
                    "links": [
                        {"label": "to_value", "source_cell_id": 2, "target_cell_id": 3},
                        {"label": "to_value", "source_cell_id": 0, "target_cell_id": 1}
                    ]
                }
            }],
            "body": {"children": [{"$ref": "#/key_value_items/0"}]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "name\tada\nrole\tadmin");
        assert_eq!((chunks[0].start, chunks[0].end), (0, 19));
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/key_value_items/0"}"##)
        );

        let reversed = serde_json::json!({
            "schema_name": "DoclingDocument",
            "key_value_items": [{
                "self_ref": "#/key_value_items/0",
                "graph": {
                    "cells": [
                        {"cell_id": 1, "label": "value", "text": "ada"},
                        {"cell_id": 2, "label": "key", "text": "name"}
                    ],
                    "links": [{"label": "to_key", "source_cell_id": 1, "target_cell_id": 2}]
                }
            }],
            "body": {"children": [{"$ref": "#/key_value_items/0"}]}
        });
        assert_eq!(texts(&cut(&reversed.to_string())), vec!["name\tada"]);
    }

    #[test]
    fn tc_145_picture_contributes_no_image_uri_or_placeholder() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [{"self_ref": "#/texts/0", "label": "caption", "text": "Caption"}],
            "pictures": [{
                "self_ref": "#/pictures/0",
                "label": "picture",
                "image": {"uri": "file:///sentinel.png", "placeholder": "[image]"},
                "children": [{"$ref": "#/texts/0"}]
            }],
            "body": {"children": [{"$ref": "#/pictures/0"}]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(texts(&chunks), vec!["Caption"]);
        assert!(!chunks[0].text.contains("sentinel"));
        assert!(!chunks[0].text.contains("[image]"));
        assert!(!chunks[0].text.contains("file://"));
    }

    #[test]
    fn tc_135_tc_136_body_and_furniture_keep_every_content_kind() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [
                {"self_ref": "#/texts/0", "label": "title", "text": "Sec", "children": [
                    {"$ref": "#/texts/1"},
                    {"$ref": "#/tables/0"},
                    {"$ref": "#/pictures/0"},
                    {"$ref": "#/key_value_items/0"}
                ]},
                {"self_ref": "#/texts/1", "label": "text", "text": "BodyText", "orig": "ORIG"},
                {"self_ref": "#/texts/2", "label": "caption", "text": "Cap"},
                {"self_ref": "#/texts/3", "label": "caption", "text": "PicCap"},
                {"self_ref": "#/texts/4", "label": "page_header", "text": "KeepHeader"},
                {"self_ref": "#/texts/5", "label": "text", "text": "FurnText"},
                {"self_ref": "#/texts/6", "label": "caption", "text": "FurnCap"},
                {"self_ref": "#/texts/7", "label": "caption", "text": "FurnPic"},
                {"self_ref": "#/texts/8", "label": "section_header", "text": "FurnHead"}
            ],
            "tables": [
                {"self_ref": "#/tables/0", "label": "table", "children": [{"$ref": "#/texts/2"}],
                    "data": {"num_rows": 1, "num_cols": 2, "table_cells": [{"text": "A"}, {"text": "B"}]}},
                {"self_ref": "#/tables/1", "label": "table", "children": [{"$ref": "#/texts/6"}],
                    "data": {"num_rows": 1, "num_cols": 2, "table_cells": [{"text": "F"}, {"text": "G"}]}}
            ],
            "pictures": [
                {"self_ref": "#/pictures/0", "label": "picture", "image": {"uri": "file:///sentinel.png"},
                    "children": [{"$ref": "#/texts/3"}]},
                {"self_ref": "#/pictures/1", "label": "picture", "image": {"uri": "file:///furn.png"},
                    "children": [{"$ref": "#/texts/7"}]}
            ],
            "key_value_items": [
                {"self_ref": "#/key_value_items/0", "graph": {"cells": [
                    {"cell_id": 0, "label": "key", "text": "n"},
                    {"cell_id": 1, "label": "value", "text": "v"}
                ], "links": [{"label": "to_value", "source_cell_id": 0, "target_cell_id": 1}]}},
                {"self_ref": "#/key_value_items/1", "graph": {"cells": [
                    {"cell_id": 0, "label": "key", "text": "k"},
                    {"cell_id": 1, "label": "value", "text": "v"}
                ], "links": [{"label": "to_value", "source_cell_id": 0, "target_cell_id": 1}]}}
            ],
            "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/4"}]},
            "furniture": {"children": [
                {"$ref": "#/texts/5"},
                {"$ref": "#/tables/1"},
                {"$ref": "#/pictures/1"},
                {"$ref": "#/key_value_items/1"},
                {"$ref": "#/texts/8"}
            ]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(
            texts(&chunks),
            vec![
                "Sec",
                "BodyText",
                "A\tB",
                "Cap",
                "PicCap",
                "n\tv",
                "KeepHeader",
                "FurnText",
                "F\tG",
                "FurnCap",
                "FurnPic",
                "k\tv",
                "FurnHead",
            ]
        );
        assert_eq!(chunks[0].heading_path, Vec::<String>::new());
        assert_eq!(chunks[1].heading_path, vec!["Sec".to_string()]);
        assert_eq!(chunks[2].heading_path, vec!["Sec".to_string()]);
        assert_eq!(chunks[5].heading_path, vec!["Sec".to_string()]);
        assert_eq!(
            chunks[6].heading_path,
            vec!["Sec".to_string()],
            "a page header follows the headings still open"
        );
        assert!(
            chunks[6]
                .meta_json
                .as_deref()
                .unwrap()
                .contains(r#""section":"Sec""#)
        );
        for chunk in &chunks[7..] {
            assert!(chunk.heading_path.is_empty(), "{}", chunk.text);
            let meta = chunk.meta_json.as_deref().unwrap();
            assert!(meta.contains(r#""section":null"#), "{meta}");
        }
        assert!(!texts(&chunks).join("\n").contains("sentinel"));
        assert!(!texts(&chunks).join("\n").contains("ORIG"));
    }

    #[test]
    fn tc_47_tc_49_tc_171_meta_section_and_path_use_stored_titles() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [
                {
                    "self_ref": "#/texts/0",
                    "label": "title",
                    "text": "  A # ",
                    "orig": "DIFFERENT",
                    "prov": [{"page_no": 1}],
                    "children": [{"$ref": "#/texts/1"}]
                },
                {
                    "self_ref": "#/texts/1",
                    "label": "section_header",
                    "text": "Details",
                    "children": [{"$ref": "#/texts/2"}]
                },
                {
                    "self_ref": "#/texts/2",
                    "label": "text",
                    "text": "a😀b",
                    "prov": [{"page_no": 3}, {"page_no": 9}]
                },
                {
                    "self_ref": "#/texts/3",
                    "label": "page_header",
                    "text": "Head",
                    "prov": [{"page_no": 4}]
                },
                {
                    "self_ref": "#/texts/4",
                    "label": "section_header",
                    "text": "FurnHead",
                    "children": [{"$ref": "#/texts/5"}]
                },
                {"self_ref": "#/texts/5", "label": "text", "text": "FurnBody"},
                {
                    "self_ref": "#/texts/6",
                    "label": "title",
                    "text": "",
                    "children": [{"$ref": "#/texts/7"}]
                },
                {"self_ref": "#/texts/7", "label": "text", "text": "AfterEmpty"}
            ],
            "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/6"}]},
            "furniture": {"children": [
                {"$ref": "#/texts/2"},
                {"$ref": "#/texts/3"},
                {"$ref": "#/texts/4"}
            ]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(
            texts(&chunks),
            vec![
                "  A # ",
                "Details",
                "a😀b",
                "AfterEmpty",
                "Head",
                "FurnHead",
                "FurnBody"
            ]
        );
        assert_eq!(chunks[0].heading_path, Vec::<String>::new());
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":1,"section":null,"element":"#/texts/0"}"##)
        );
        assert_eq!(chunks[1].heading_path, vec!["  A # ".to_string()]);
        assert_eq!(
            chunks[1].meta_json.as_deref(),
            Some(r##"{"page":null,"section":"  A # ","element":"#/texts/1"}"##)
        );
        assert_eq!(
            chunks[2].heading_path,
            vec!["  A # ".to_string(), "Details".to_string()]
        );
        assert_eq!((chunks[2].start, chunks[2].end), (0, 3));
        assert_eq!(chunks[2].text, "a😀b");
        assert_eq!(
            chunks[2].meta_json.as_deref(),
            Some(r##"{"page":3,"section":"Details","element":"#/texts/2"}"##)
        );
        assert!(chunks[3].heading_path.is_empty());
        assert_eq!(chunks[3].text, "AfterEmpty");
        for chunk in &chunks[4..] {
            assert!(chunk.heading_path.is_empty(), "{}", chunk.text);
            let meta = chunk.meta_json.as_deref().unwrap();
            assert!(meta.contains(r#""section":null"#), "{meta}");
        }
        assert_eq!(
            chunks[4].meta_json.as_deref(),
            Some(r##"{"page":4,"section":null,"element":"#/texts/3"}"##)
        );
        assert_eq!(
            chunks.iter().filter(|chunk| chunk.text == "a😀b").count(),
            1
        );
    }

    #[test]
    fn tc_150_long_item_pieces_stay_together_before_children() {
        let item = format!("{}{}c", "a".repeat(448), "b".repeat(64));
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [
                {
                    "self_ref": "#/texts/0",
                    "label": "text",
                    "text": item,
                    "children": [{"$ref": "#/texts/1"}]
                },
                {"self_ref": "#/texts/1", "label": "text", "text": "child"},
                {"self_ref": "#/texts/2", "label": "text", "text": "next"},
                {"self_ref": "#/texts/3", "label": "text", "text": "foot"}
            ],
            "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/2"}]},
            "furniture": {"children": [{"$ref": "#/texts/3"}]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(chunks.len(), 5);
        assert_eq!(
            chunks[0].text,
            format!("{}{}", "a".repeat(448), "b".repeat(64))
        );
        assert_eq!((chunks[0].start, chunks[0].end), (0, 512));
        assert_eq!(chunks[1].text, format!("{}c", "b".repeat(64)));
        assert_eq!((chunks[1].start, chunks[1].end), (448, 513));
        assert_eq!(chunks[0].meta_json, chunks[1].meta_json);
        assert_eq!(
            chunks[0].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/texts/0"}"##)
        );
        assert_eq!(chunks[2].text, "child");
        assert_eq!(chunks[3].text, "next");
        assert_eq!(chunks[4].text, "foot");
        assert!(chunks[4].heading_path.is_empty());
    }

    #[test]
    fn tc_239_tc_240_groups_are_containers_and_refs_are_emitted_once() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "groups": [{
                "self_ref": "#/groups/0",
                "label": "list",
                "name": "NotAHeading",
                "children": [
                    {"$ref": "#/texts/2"},
                    {"$ref": "#/texts/3"},
                    {"$ref": "#/texts/2"}
                ]
            }],
            "texts": [
                {
                    "self_ref": "#/texts/0",
                    "label": "text",
                    "text": "parent",
                    "children": [{"$ref": "#/texts/1"}]
                },
                {"self_ref": "#/texts/1", "label": "text", "text": "child"},
                {"self_ref": "#/texts/2", "label": "text", "text": "a"},
                {"self_ref": "#/texts/3", "label": "text", "text": "b"},
                {"self_ref": "#/texts/4", "label": "text", "text": "foot"},
                {"self_ref": "#/texts/5", "label": "text", "text": "orphan"}
            ],
            "body": {"children": [
                {"$ref": "#/texts/0"},
                {"$ref": "#/groups/0"},
                {"$ref": "#/texts/0"}
            ]},
            "furniture": {"children": [{"$ref": "#/texts/4"}]}
        });
        let chunks = cut(&doc.to_string());
        assert_eq!(texts(&chunks), vec!["parent", "child", "a", "b", "foot"]);
        assert!(chunks.iter().all(|chunk| chunk.heading_path.is_empty()));
        assert_eq!(
            chunks[2].meta_json.as_deref(),
            Some(r##"{"page":null,"section":null,"element":"#/texts/2"}"##)
        );
        assert!(
            chunks[4]
                .meta_json
                .as_deref()
                .unwrap()
                .contains(r#""section":null"#)
        );
    }

    #[test]
    fn tc_241_sibling_headers_fill_the_items_that_follow() {
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [
                {"self_ref": "#/texts/0", "label": "title", "text": "Paper", "children": []},
                {"self_ref": "#/texts/1", "label": "section_header", "text": "ABSTRACT", "level": 1, "children": []},
                {"self_ref": "#/texts/2", "label": "text", "text": "Summary", "children": []},
                {"self_ref": "#/texts/3", "label": "section_header", "text": "Intro", "level": 1, "children": [
                    {"$ref": "#/texts/4"}
                ]},
                {"self_ref": "#/texts/4", "label": "text", "text": "Nested", "children": []},
                {"self_ref": "#/texts/5", "label": "text", "text": "After", "children": []},
                {"self_ref": "#/texts/6", "label": "section_header", "text": "Background", "level": 2, "children": []},
                {"self_ref": "#/texts/7", "label": "text", "text": "Earlier", "children": []},
                {"self_ref": "#/texts/8", "label": "section_header", "text": "Methods", "level": 1, "children": []},
                {"self_ref": "#/texts/9", "label": "text", "text": "Done", "children": []},
                {"self_ref": "#/texts/10", "label": "section_header", "text": "Plain", "children": []},
                {"self_ref": "#/texts/11", "label": "text", "text": "UnderPlain", "children": []}
            ],
            "body": {"children": [
                {"$ref": "#/texts/0"},
                {"$ref": "#/texts/1"},
                {"$ref": "#/texts/2"},
                {"$ref": "#/texts/3"},
                {"$ref": "#/texts/5"},
                {"$ref": "#/texts/6"},
                {"$ref": "#/texts/7"},
                {"$ref": "#/texts/8"},
                {"$ref": "#/texts/9"},
                {"$ref": "#/texts/10"},
                {"$ref": "#/texts/11"}
            ]}
        });
        let chunks = cut(&doc.to_string());
        let paths: Vec<(&str, Vec<&str>)> = chunks
            .iter()
            .map(|chunk| {
                (
                    chunk.text.as_str(),
                    chunk.heading_path.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            paths,
            vec![
                ("Paper", vec![]),
                ("ABSTRACT", vec!["Paper"]),
                ("Summary", vec!["Paper", "ABSTRACT"]),
                ("Intro", vec!["Paper"]),
                ("Nested", vec!["Paper", "Intro"]),
                ("After", vec!["Paper", "Intro"]),
                ("Background", vec!["Paper", "Intro"]),
                ("Earlier", vec!["Paper", "Intro", "Background"]),
                ("Methods", vec!["Paper"]),
                ("Done", vec!["Paper", "Methods"]),
                ("Plain", vec!["Paper"]),
                ("UnderPlain", vec!["Paper", "Plain"]),
            ]
        );
        assert_eq!(
            chunks[2].meta_json.as_deref(),
            Some(r##"{"page":null,"section":"ABSTRACT","element":"#/texts/2"}"##)
        );
    }
}
