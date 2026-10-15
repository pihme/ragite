//! Chunk invariants for the fuzz targets and for TC-250.
//!
//! The length limit does not apply to every chunk. Markdown headings, HTML
//! headings, and HTML comments stay whole (REQ-50, REQ-79).

use crate::html::{self, Piece};
use crate::{CHUNK_LIMIT, Chunk, OVERLAP, chunk_docling, chunk_markdown, chunk_plain};

pub fn check_plain(text: &str) -> Result<(), String> {
    let chunks = chunk_plain(text);
    check_count(text, chunks.len())?;
    check_starts_increase(&chunks)?;
    for chunk in &chunks {
        check_exact_slice(text, chunk)?;
        check_path(chunk)?;
        if chunk.meta_json.is_some() {
            return Err("plain chunk has meta_json".to_string());
        }
        if !chunk.heading_path.is_empty() {
            return Err("plain chunk has a heading path".to_string());
        }
        let n = chunk.text.chars().count();
        if n > CHUNK_LIMIT {
            return Err(format!("plain piece has {n} scalars"));
        }
    }
    check_overlap(&chunks)
}

pub fn check_markdown(text: &str) -> Result<(), String> {
    let chunks = chunk_markdown(text);
    check_count(text, chunks.len())?;
    check_starts_increase(&chunks)?;
    for chunk in &chunks {
        check_exact_slice(text, chunk)?;
        check_path(chunk)?;
        if chunk.meta_json.is_some() {
            return Err("markdown chunk has meta_json".to_string());
        }
        let n = chunk.text.chars().count();
        if n > CHUNK_LIMIT && !is_atx_heading_line(&chunk.text) {
            return Err(format!(
                "markdown piece has {n} scalars and is not one heading"
            ));
        }
    }
    check_overlap(&chunks)
}

pub fn check_html(text: &str) -> Result<(), String> {
    let pieces = html::cut_pieces(text);
    check_count(text, pieces.len())?;
    let chars: Vec<char> = text.chars().collect();
    for pair in pieces.windows(2) {
        if pair[1].chunk.start < pair[0].chunk.start {
            return Err(format!(
                "html chunk start {} precedes {}",
                pair[1].chunk.start, pair[0].chunk.start
            ));
        }
    }
    for piece in &pieces {
        check_html_piece(&chars, piece)?;
    }
    Ok(())
}

pub fn check_docling(text: &str) -> Result<(), String> {
    match chunk_docling(text) {
        Ok(chunks) => check_docling_chunks(text, &chunks),
        Err(err) => docling_error_ok(text, &err),
    }
}

fn check_html_piece(chars: &[char], piece: &Piece) -> Result<(), String> {
    let chunk = &piece.chunk;
    if chunk.meta_json.is_some() {
        return Err("html chunk has meta_json".to_string());
    }
    check_path(chunk)?;
    if chunk.text.is_empty() {
        return Err("html chunk is empty".to_string());
    }
    if chunk.start >= chunk.end || chunk.end > chars.len() {
        return Err(format!(
            "html span {}..{} outside {}",
            chunk.start,
            chunk.end,
            chars.len()
        ));
    }
    let n = chunk.text.chars().count();
    if !piece.whole && n > CHUNK_LIMIT {
        return Err(format!("html paragraph piece has {n} scalars"));
    }
    if chunk.text.contains('\0') {
        let slice: String = chars[chunk.start..chunk.end].iter().collect();
        if !slice.contains('\0') {
            return Err("html chunk introduced U+0000".to_string());
        }
    }
    Ok(())
}

fn check_docling_chunks(text: &str, chunks: &[Chunk]) -> Result<(), String> {
    let allows_null = text.contains('\0') || text.to_ascii_lowercase().contains("\\u0000");
    for chunk in chunks {
        check_path(chunk)?;
        let Some(meta) = chunk.meta_json.as_deref() else {
            return Err("docling chunk lacks meta_json".to_string());
        };
        check_docling_meta(meta)?;
        let n = chunk.text.chars().count();
        if n == 0 || n > CHUNK_LIMIT {
            return Err(format!("docling piece has {n} scalars"));
        }
        let Some(span) = chunk.end.checked_sub(chunk.start) else {
            return Err("docling end is before start".to_string());
        };
        if span != n {
            return Err("docling span does not match the piece".to_string());
        }
        if chunk.text.contains('\0') && !allows_null {
            return Err("docling chunk introduced U+0000".to_string());
        }
    }
    check_overlap(chunks)
}

fn check_docling_meta(raw: &str) -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "meta_json is not JSON".to_string())?;
    let Some(object) = value.as_object() else {
        return Err("meta_json is not an object".to_string());
    };
    for key in ["page", "section", "element"] {
        if !object.contains_key(key) {
            return Err(format!("meta_json lacks {key}"));
        }
    }
    if !object
        .get("element")
        .is_some_and(serde_json::Value::is_string)
    {
        return Err("element is not a string".to_string());
    }
    Ok(())
}

fn docling_error_ok(text: &str, err: &str) -> Result<(), String> {
    let known = err == "the text is not JSON"
        || err == "the JSON value is not an object"
        || err == "schema_name is missing"
        || err.starts_with("schema_name is not DoclingDocument: ");
    if !known {
        return Err(format!("unexpected docling error: {}", preview(err)));
    }
    if text.chars().count() >= 80 && err.contains(text) {
        return Err("docling error repeats the document".to_string());
    }
    Ok(())
}

fn check_exact_slice(text: &str, chunk: &Chunk) -> Result<(), String> {
    let chars: Vec<char> = text.chars().collect();
    if chunk.end > chars.len() || chunk.start >= chunk.end {
        return Err(format!(
            "span {}..{} outside {}",
            chunk.start,
            chunk.end,
            chars.len()
        ));
    }
    let slice: String = chars[chunk.start..chunk.end].iter().collect();
    if slice != chunk.text {
        return Err("chunk text is not the source slice".to_string());
    }
    Ok(())
}

fn check_path(chunk: &Chunk) -> Result<(), String> {
    if chunk.heading_path.iter().any(String::is_empty) {
        return Err("heading path has an empty entry".to_string());
    }
    Ok(())
}

fn check_count(text: &str, n: usize) -> Result<(), String> {
    let chars = text.chars().count();
    if n > chars {
        return Err(format!("{n} chunks from {chars} scalars"));
    }
    Ok(())
}

fn check_starts_increase(chunks: &[Chunk]) -> Result<(), String> {
    for pair in chunks.windows(2) {
        if pair[1].start <= pair[0].start {
            return Err(format!(
                "chunk start {} does not follow {}",
                pair[1].start, pair[0].start
            ));
        }
    }
    Ok(())
}

/// Adjacent pieces of one unit overlap by [`OVERLAP`], and the earlier piece is one full limit.
fn check_overlap(chunks: &[Chunk]) -> Result<(), String> {
    for pair in chunks.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.heading_path != b.heading_path || a.meta_json != b.meta_json {
            continue;
        }
        if b.start >= a.end || a.start >= b.end || b.start <= a.start {
            continue;
        }
        if a.end - b.start != OVERLAP {
            return Err(format!("overlap is {} scalars", a.end - b.start));
        }
        if a.text.chars().count() != CHUNK_LIMIT {
            return Err("overlapped piece is shorter than the length limit".to_string());
        }
    }
    Ok(())
}

/// A heading chunk is one line: up to three spaces, one to six `#`, then a space.
fn is_atx_heading_line(text: &str) -> bool {
    if text.contains('\n') {
        return false;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    let mut spaces = 0;
    while index < chars.len() && chars[index] == ' ' && spaces < 3 {
        spaces += 1;
        index += 1;
    }
    if index < chars.len() && chars[index] == ' ' {
        return false;
    }
    let mut level = 0;
    while index < chars.len() && chars[index] == '#' {
        level += 1;
        index += 1;
        if level > 6 {
            return false;
        }
    }
    level >= 1 && index < chars.len() && chars[index] == ' '
}

fn preview(text: &str) -> String {
    let mut out = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i == 40 {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus(kind: &str) -> Vec<(String, String)> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../fuzz/corpus/{kind}"));
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
            .map(|entry| entry.unwrap_or_else(|err| panic!("corpus entry: {err}")))
            .filter(|entry| entry.file_type().unwrap().is_file())
            .collect();
        assert!(!files.is_empty(), "corpus {kind} is empty");
        files.sort_by_key(|entry| entry.file_name());
        files
            .into_iter()
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let bytes =
                    std::fs::read(entry.path()).unwrap_or_else(|err| panic!("read {name}: {err}"));
                let text = String::from_utf8_lossy(&bytes).into_owned();
                (name, text)
            })
            .collect()
    }

    #[test]
    fn tc_250_seed_corpus_obeys_chunk_invariants() {
        for (name, text) in corpus("html") {
            check_html(&text).unwrap_or_else(|err| panic!("html/{name}: {err}"));
        }
        for (name, text) in corpus("markdown") {
            check_markdown(&text).unwrap_or_else(|err| panic!("markdown/{name}: {err}"));
            check_plain(&text).unwrap_or_else(|err| panic!("plain via markdown/{name}: {err}"));
        }
        for (name, text) in corpus("docling") {
            check_docling(&text).unwrap_or_else(|err| panic!("docling/{name}: {err}"));
        }

        let long = "h".repeat(CHUNK_LIMIT + 1);
        let heading = format!("# {long}");
        check_markdown(&heading).expect("long heading");
        assert_eq!(chunk_markdown(&heading).len(), 1);

        let comment = format!("<!--{long}-->");
        check_html(&comment).expect("long comment");

        let paragraph = "p".repeat(CHUNK_LIMIT + OVERLAP);
        check_plain(&paragraph).expect("split plain paragraph");
        check_markdown(&paragraph).expect("split markdown paragraph");

        let html_para = format!("<p>{paragraph}</p>");
        check_html(&html_para).expect("split html paragraph");
        let pieces = html::cut_pieces(&html_para);
        assert!(
            pieces
                .iter()
                .any(|piece| { !piece.whole && piece.chunk.text.chars().count() == CHUNK_LIMIT }),
            "a long html paragraph must produce a split piece"
        );

        let doc = format!(
            r##"{{"schema_name":"DoclingDocument","texts":[{{"self_ref":"#/texts/0","label":"text","text":"{paragraph}"}}],"body":{{"children":[{{"$ref":"#/texts/0"}}]}}}}"##
        );
        check_docling(&doc).expect("split docling item");
    }
}
