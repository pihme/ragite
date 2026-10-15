/// One slice cut from a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub text: String,
    /// Inclusive Unicode-scalar index in the stored document.
    pub start: usize,
    /// Exclusive Unicode-scalar index in the stored document.
    pub end: usize,
    pub heading_path: Vec<String>,
    pub meta_json: Option<String>,
}

/// Cut plain text on blank lines, then split a long paragraph by the length limit.
pub fn chunk_plain(text: &str) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    for (start, end) in paragraph_ranges(&chars) {
        for (piece_start, piece_end) in split_range(start, end) {
            out.push(Chunk {
                text: chars[piece_start..piece_end].iter().collect(),
                start: piece_start,
                end: piece_end,
                heading_path: Vec::new(),
                meta_json: None,
            });
        }
    }
    out
}

/// Markdown rules replace this body. Until then the plain cutter runs.
pub fn chunk_markdown(text: &str) -> Vec<Chunk> {
    chunk_plain(text)
}

/// Cut stored HTML into chunks of visible text. Offsets are scalar indexes in `text`.
pub fn chunk_html(text: &str) -> Vec<Chunk> {
    crate::html::cut(text)
}

/// Docling rules replace this body. Until then the plain cutter runs.
pub fn chunk_docling(text: &str) -> Result<Vec<Chunk>, String> {
    Ok(chunk_plain(text))
}

fn newline_len(chars: &[char], index: usize) -> Option<usize> {
    if index >= chars.len() {
        return None;
    }
    if chars[index] == '\n' {
        return Some(1);
    }
    if chars[index] == '\r' && index + 1 < chars.len() && chars[index + 1] == '\n' {
        return Some(2);
    }
    None
}

fn is_blank_run(chars: &[char], index: usize) -> bool {
    let Some(first) = newline_len(chars, index) else {
        return false;
    };
    let mut next = index + first;
    while next < chars.len() && (chars[next] == ' ' || chars[next] == '\t') {
        next += 1;
    }
    newline_len(chars, next).is_some()
}

/// Consume a run of blank lines starting at `index`. The end is exclusive.
fn consume_blank_run(chars: &[char], index: usize) -> usize {
    let first = newline_len(chars, index).expect("blank run starts on a newline");
    let mut cursor = index + first;
    while cursor < chars.len() && (chars[cursor] == ' ' || chars[cursor] == '\t') {
        cursor += 1;
    }
    let second = newline_len(chars, cursor).expect("blank run has a second newline");
    cursor += second;
    loop {
        let mut look = cursor;
        while look < chars.len() && (chars[look] == ' ' || chars[look] == '\t') {
            look += 1;
        }
        if let Some(extra) = newline_len(chars, look) {
            cursor = look + extra;
        } else {
            break;
        }
    }
    cursor
}

fn paragraph_ranges(chars: &[char]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut paragraph_start = 0;
    let mut index = 0;
    while index < chars.len() {
        if is_blank_run(chars, index) {
            if index > paragraph_start {
                ranges.push((paragraph_start, index));
            }
            index = consume_blank_run(chars, index);
            paragraph_start = index;
            continue;
        }
        index += 1;
    }
    if paragraph_start < chars.len() {
        ranges.push((paragraph_start, chars.len()));
    }
    ranges
}

pub(crate) fn split_range(start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut pieces = Vec::new();
    if end <= start {
        return pieces;
    }
    if end - start <= crate::CHUNK_LIMIT {
        pieces.push((start, end));
        return pieces;
    }
    let mut cursor = start;
    while cursor < end {
        let piece_end = (cursor + crate::CHUNK_LIMIT).min(end);
        pieces.push((cursor, piece_end));
        if piece_end == end {
            break;
        }
        cursor = piece_end - crate::OVERLAP;
    }
    pieces
}
