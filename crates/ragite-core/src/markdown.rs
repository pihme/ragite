//! Markdown subset: ATX headings, blank-line paragraphs, fences, indented code.

use crate::chunk::{Chunk, split_range};

struct Line {
    content_end: usize,
    next: usize,
}

struct HeadingLine {
    level: usize,
    title: String,
    content_end: usize,
    next: usize,
}

pub(crate) fn cut(text: &str) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if is_blank_line(&chars, index) {
            index = line_at(&chars, index).next;
            continue;
        }
        if let Some(heading) = heading_at(&chars, index) {
            while stack
                .last()
                .is_some_and(|(level, _)| *level >= heading.level)
            {
                stack.pop();
            }
            // An empty title does not become a Chunk and does not enter the path.
            // The length limit does not split a heading.
            if !heading.title.is_empty() {
                push_whole(&mut out, &chars, index, heading.content_end, titles(&stack));
                stack.push((heading.level, heading.title));
            }
            index = heading.next;
            continue;
        }
        if let Some(open) = fence_open_at(&chars, index) {
            let (end, next) = fence_end(&chars, &open);
            push_split(&mut out, &chars, open.start, end, titles(&stack));
            index = next;
            continue;
        }
        if is_indented_code_line(&chars, index) {
            index = push_indented(&chars, index, &titles(&stack), &mut out);
            continue;
        }
        index = push_paragraph(&chars, index, &titles(&stack), &mut out);
    }
    out
}

fn titles(stack: &[(usize, String)]) -> Vec<String> {
    stack.iter().map(|(_, title)| title.clone()).collect()
}

fn push_whole(out: &mut Vec<Chunk>, chars: &[char], start: usize, end: usize, path: Vec<String>) {
    if end <= start {
        return;
    }
    out.push(Chunk {
        text: chars[start..end].iter().collect(),
        start,
        end,
        heading_path: path,
        meta_json: None,
    });
}

fn push_split(out: &mut Vec<Chunk>, chars: &[char], start: usize, end: usize, path: Vec<String>) {
    for (piece_start, piece_end) in split_range(chars, start, end) {
        out.push(Chunk {
            text: chars[piece_start..piece_end].iter().collect(),
            start: piece_start,
            end: piece_end,
            heading_path: path.clone(),
            meta_json: None,
        });
    }
}

fn line_at(chars: &[char], start: usize) -> Line {
    let mut index = start;
    while index < chars.len() {
        if chars[index] == '\n' {
            return Line {
                content_end: index,
                next: index + 1,
            };
        }
        if chars[index] == '\r' && index + 1 < chars.len() && chars[index + 1] == '\n' {
            return Line {
                content_end: index,
                next: index + 2,
            };
        }
        index += 1;
    }
    Line {
        content_end: chars.len(),
        next: chars.len(),
    }
}

fn is_blank_line(chars: &[char], start: usize) -> bool {
    let line = line_at(chars, start);
    chars[start..line.content_end]
        .iter()
        .all(|c| *c == ' ' || *c == '\t')
}

fn heading_at(chars: &[char], start: usize) -> Option<HeadingLine> {
    let line = line_at(chars, start);
    let content = &chars[start..line.content_end];
    let mut index = 0;
    let mut spaces = 0;
    while index < content.len() && content[index] == ' ' && spaces < 3 {
        spaces += 1;
        index += 1;
    }
    if index < content.len() && content[index] == ' ' {
        return None;
    }
    let mut level = 0;
    while index < content.len() && content[index] == '#' {
        level += 1;
        index += 1;
        if level > 6 {
            return None;
        }
    }
    if level == 0 || index >= content.len() || content[index] != ' ' {
        return None;
    }
    index += 1;
    Some(HeadingLine {
        level,
        title: heading_title(&content[index..]),
        content_end: line.content_end,
        next: line.next,
    })
}

/// Title with the opening space, closing `#` run, and surrounding spaces removed.
fn heading_title(content: &[char]) -> String {
    let mut end = content.len();
    while end > 0 && content[end - 1] == ' ' {
        end -= 1;
    }
    let mut hash_at = end;
    while hash_at > 0 && content[hash_at - 1] == '#' {
        hash_at -= 1;
    }
    if hash_at < end && (hash_at == 0 || content[hash_at - 1] == ' ') {
        end = hash_at;
        while end > 0 && content[end - 1] == ' ' {
            end -= 1;
        }
    }
    let mut begin = 0;
    while begin < end && content[begin] == ' ' {
        begin += 1;
    }
    content[begin..end].iter().collect()
}

struct FenceOpen {
    marker: char,
    len: usize,
    start: usize,
    next: usize,
}

fn fence_open_at(chars: &[char], start: usize) -> Option<FenceOpen> {
    let line = line_at(chars, start);
    let content = &chars[start..line.content_end];
    let mut index = 0;
    let mut spaces = 0;
    while index < content.len() && content[index] == ' ' && spaces < 3 {
        spaces += 1;
        index += 1;
    }
    if index < content.len() && content[index] == ' ' {
        return None;
    }
    if index >= content.len() {
        return None;
    }
    let marker = content[index];
    if marker != '`' && marker != '~' {
        return None;
    }
    let mark_at = index;
    while index < content.len() && content[index] == marker {
        index += 1;
    }
    let len = index - mark_at;
    if len < 3 {
        return None;
    }
    Some(FenceOpen {
        marker,
        len,
        start: start + mark_at,
        next: line.next,
    })
}

fn fence_end(chars: &[char], open: &FenceOpen) -> (usize, usize) {
    let mut scan = open.next;
    while scan < chars.len() {
        if let Some(end) = fence_close_end(chars, scan, open.marker, open.len) {
            return (end, line_at(chars, scan).next);
        }
        scan = line_at(chars, scan).next;
    }
    (chars.len(), chars.len())
}

/// Exclusive index of the last closing marker, when this line closes the fence.
fn fence_close_end(chars: &[char], start: usize, marker: char, min_len: usize) -> Option<usize> {
    let line = line_at(chars, start);
    let content = &chars[start..line.content_end];
    let mut index = 0;
    while index < content.len() && content[index] == ' ' {
        index += 1;
    }
    if index >= content.len() || content[index] != marker {
        return None;
    }
    let mut count = 0;
    while index < content.len() && content[index] == marker {
        count += 1;
        index += 1;
    }
    if count < min_len {
        return None;
    }
    let marker_end = index;
    while index < content.len() && content[index] == ' ' {
        index += 1;
    }
    if index != content.len() {
        return None;
    }
    Some(start + marker_end)
}

fn is_indented_code_line(chars: &[char], start: usize) -> bool {
    let line = line_at(chars, start);
    let content = &chars[start..line.content_end];
    content.len() >= 4
        && content[..4].iter().all(|c| *c == ' ')
        && content.iter().any(|c| *c != ' ' && *c != '\t')
}

fn push_indented(chars: &[char], start: usize, path: &[String], out: &mut Vec<Chunk>) -> usize {
    let first = line_at(chars, start);
    let mut last_content_end = first.content_end;
    let mut index = first.next;
    while index < chars.len() {
        if is_indented_code_line(chars, index) {
            let line = line_at(chars, index);
            last_content_end = line.content_end;
            index = line.next;
            continue;
        }
        if is_blank_line(chars, index) && indented_follows(chars, line_at(chars, index).next) {
            index = line_at(chars, index).next;
            continue;
        }
        break;
    }
    push_split(out, chars, start, last_content_end, path.to_vec());
    index
}

fn indented_follows(chars: &[char], mut index: usize) -> bool {
    while index < chars.len() && is_blank_line(chars, index) {
        index = line_at(chars, index).next;
    }
    index < chars.len() && is_indented_code_line(chars, index)
}

fn push_paragraph(chars: &[char], start: usize, path: &[String], out: &mut Vec<Chunk>) -> usize {
    let first = line_at(chars, start);
    let mut last_content_end = first.content_end;
    let mut index = first.next;
    while index < chars.len() {
        if is_blank_line(chars, index)
            || heading_at(chars, index).is_some()
            || fence_open_at(chars, index).is_some()
        {
            break;
        }
        let line = line_at(chars, index);
        last_content_end = line.content_end;
        index = line.next;
    }
    push_split(out, chars, start, last_content_end, path.to_vec());
    index
}

#[cfg(test)]
mod tests {
    use crate::{CHUNK_LIMIT, Chunk, OVERLAP, chunk_markdown};

    fn assert_chunk(chunk: &Chunk, text: &str, start: usize, end: usize, path: &[&str]) {
        assert_eq!(chunk.text, text);
        assert_eq!((chunk.start, chunk.end), (start, end));
        assert_eq!(
            chunk
                .heading_path
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            path
        );
        assert!(chunk.meta_json.is_none());
        assert_eq!(text.chars().count(), end - start);
    }

    #[test]
    fn tc_97_tc_138_tc_46_heading_only_document() {
        let chunks = chunk_markdown("# Only");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "# Only", 0, 6, &[]);
    }

    #[test]
    fn tc_141_empty_heading_is_not_a_chunk() {
        assert!(chunk_markdown("# ").is_empty());
        assert!(chunk_markdown("# \n").is_empty());
        let chunks = chunk_markdown("# A\n\n# \n\nBody");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "# A", 0, 3, &[]);
        assert_chunk(&chunks[1], "Body", 9, 13, &[]);
        let chunks = chunk_markdown("# A\n\n## \n\nBody");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[1], "Body", 10, 14, &["A"]);
    }

    #[test]
    fn tc_139_tc_168_tc_169_tc_201_headings_and_paths() {
        let chunks = chunk_markdown("## Title\n\nBody");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "## Title", 0, 8, &[]);
        assert_chunk(&chunks[1], "Body", 10, 14, &["Title"]);

        let chunks = chunk_markdown("## Title\r\n\r\nBody");
        assert_chunk(&chunks[0], "## Title", 0, 8, &[]);
        assert_chunk(&chunks[1], "Body", 12, 16, &["Title"]);

        let chunks = chunk_markdown("# A😀\n\nB");
        assert_chunk(&chunks[0], "# A😀", 0, 4, &[]);
        assert_chunk(&chunks[1], "B", 6, 7, &["A😀"]);

        let cases = [
            ("# T\n\nB", "# T", "T"),
            (" # T\n\nB", " # T", "T"),
            ("  # T\n\nB", "  # T", "T"),
            ("   # T\n\nB", "   # T", "T"),
            ("###### T\n\nB", "###### T", "T"),
            ("# T #\n\nB", "# T #", "T"),
            ("## T ##\n\nB", "## T ##", "T"),
        ];
        for (doc, heading, title) in cases {
            let chunks = chunk_markdown(doc);
            assert_eq!(chunks.len(), 2, "{doc}");
            assert_eq!(chunks[0].text, heading);
            assert_eq!(chunks[1].text, "B");
            assert_eq!(chunks[1].heading_path, vec![title.to_string()]);
        }

        let chunks = chunk_markdown("  ## Title ##\n\nnext");
        assert_eq!(chunks[0].text, "  ## Title ##");
        assert_eq!(chunks[0].heading_path, Vec::<String>::new());
        let entry = chunks[1].heading_path[0].as_str();
        assert!(!entry.contains('#'));
        assert!(!entry.contains('\n'));
        assert!(!entry.contains('\r'));
        assert_eq!(entry, "Title");

        let chunks = chunk_markdown("# A\n## B\nbody");
        assert_chunk(&chunks[0], "# A", 0, 3, &[]);
        assert_chunk(&chunks[1], "## B", 4, 8, &["A"]);
        assert_chunk(&chunks[2], "body", 9, 13, &["A", "B"]);

        let chunks = chunk_markdown("# Above\n\n## Title\n\nBody");
        assert_chunk(&chunks[0], "# Above", 0, 7, &[]);
        assert_chunk(&chunks[1], "## Title", 9, 17, &["Above"]);
        assert_chunk(&chunks[2], "Body", 19, 23, &["Above", "Title"]);
    }

    #[test]
    fn tc_202_marks_without_a_heading_shape_stay_text() {
        let chunks = chunk_markdown("    # Title\n\nBody");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "    # Title", 0, 11, &[]);
        assert_chunk(&chunks[1], "Body", 13, 17, &[]);

        let chunks = chunk_markdown("####### Title\n\nBody");
        assert_chunk(&chunks[0], "####### Title", 0, 13, &[]);
        assert_chunk(&chunks[1], "Body", 15, 19, &[]);

        let chunks = chunk_markdown("#NoSpace\n\nBody");
        assert_chunk(&chunks[0], "#NoSpace", 0, 8, &[]);
        assert_chunk(&chunks[1], "Body", 10, 14, &[]);

        let chunks = chunk_markdown("#NoSpace\nstays");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "#NoSpace\nstays", 0, 14, &[]);
    }

    #[test]
    fn tc_162_blank_lines_between_headings() {
        let chunks = chunk_markdown("# H\n\na\nb\n\nc");
        assert_chunk(&chunks[0], "# H", 0, 3, &[]);
        assert_chunk(&chunks[1], "a\nb", 5, 8, &["H"]);
        assert_chunk(&chunks[2], "c", 10, 11, &["H"]);

        let chunks = chunk_markdown("a\n\n\nb");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "a", 0, 1, &[]);
        assert_chunk(&chunks[1], "b", 4, 5, &[]);

        let chunks = chunk_markdown("a\r\n\r\nb");
        assert_chunk(&chunks[0], "a", 0, 1, &[]);
        assert_chunk(&chunks[1], "b", 5, 6, &[]);

        let chunks = chunk_markdown("a\n \nb");
        assert_chunk(&chunks[0], "a", 0, 1, &[]);
        assert_chunk(&chunks[1], "b", 4, 5, &[]);

        let chunks = chunk_markdown("a\n\t\nb");
        assert_chunk(&chunks[0], "a", 0, 1, &[]);
        assert_chunk(&chunks[1], "b", 4, 5, &[]);

        let chunks = chunk_markdown("# H\r\n\r\na\r\nb\r\n\r\nc");
        assert_chunk(&chunks[0], "# H", 0, 3, &[]);
        assert_chunk(&chunks[1], "a\r\nb", 7, 11, &["H"]);
        assert_chunk(&chunks[2], "c", 15, 16, &["H"]);
    }

    #[test]
    fn tc_163_tc_175_fenced_block_is_one_chunk() {
        let chunks = chunk_markdown("before\n\n```rust\nline\n\n# not a heading\n```\n\nafter");
        assert_eq!(chunks.len(), 3);
        assert_chunk(&chunks[0], "before", 0, 6, &[]);
        assert_chunk(
            &chunks[1],
            "```rust\nline\n\n# not a heading\n```",
            8,
            41,
            &[],
        );
        assert_chunk(&chunks[2], "after", 43, 48, &[]);
    }

    #[test]
    fn tc_186_indented_block_keeps_interior_blank_and_hash() {
        let chunks = chunk_markdown("    a\n\n    # b");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "    a\n\n    # b", 0, 14, &[]);
    }

    #[test]
    fn tc_228_tc_229_tc_230_fence_markers() {
        let chunks = chunk_markdown("```rust\ncode\n```");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "```rust\ncode\n```", 0, 16, &[]);

        let chunks = chunk_markdown("~~~\ncode\n~~~");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "~~~\ncode\n~~~", 0, 12, &[]);

        let chunks = chunk_markdown("````\ncode\n````");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "````\ncode\n````", 0, 14, &[]);

        let chunks = chunk_markdown("```\ncode\n  ```  \nafter");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "```\ncode\n  ```", 0, 14, &[]);
        assert_chunk(&chunks[1], "after", 17, 22, &[]);

        let chunks = chunk_markdown("```\r\ncode\r\n```\r\n");
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], "```\r\ncode\r\n```", 0, 14, &[]);

        let chunks = chunk_markdown("```\nkeep\n````\n\ntail");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "```\nkeep\n````", 0, 13, &[]);
        assert_chunk(&chunks[1], "tail", 15, 19, &[]);

        let chunks = chunk_markdown("```\nkeep\n``` not\nstill\n```\n\nout");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "```\nkeep\n``` not\nstill\n```", 0, 26, &[]);
        assert_chunk(&chunks[1], "out", 28, 31, &[]);

        let chunks = chunk_markdown("````\ncode\n```\nstill\n````\n\nout");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "````\ncode\n```\nstill\n````", 0, 24, &[]);
        assert_chunk(&chunks[1], "out", 26, 29, &[]);

        let chunks = chunk_markdown("```\n~~~\ninside\n```\n\nafter");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "```\n~~~\ninside\n```", 0, 18, &[]);
        assert_chunk(&chunks[1], "after", 20, 25, &[]);

        let chunks = chunk_markdown("~~~\n```\ninside\n~~~\n\nafter");
        assert_eq!(chunks.len(), 2);
        assert_chunk(&chunks[0], "~~~\n```\ninside\n~~~", 0, 18, &[]);
        assert_chunk(&chunks[1], "after", 20, 25, &[]);
    }

    #[test]
    fn tc_86_tc_148_order_and_section_boundary() {
        let chunks = chunk_markdown(
            "intro\n\n# First\n\npara one\n\npara two\n\n    code\n\n```\nfence\n```\n\n# Second",
        );
        assert_eq!(chunks.len(), 7);
        assert_chunk(&chunks[0], "intro", 0, 5, &[]);
        assert_chunk(&chunks[1], "# First", 7, 14, &[]);
        assert_chunk(&chunks[2], "para one", 16, 24, &["First"]);
        assert_chunk(&chunks[3], "para two", 26, 34, &["First"]);
        assert_chunk(&chunks[4], "    code", 36, 44, &["First"]);
        assert_chunk(&chunks[5], "```\nfence\n```", 46, 59, &["First"]);
        assert_chunk(&chunks[6], "# Second", 61, 69, &[]);

        let chunks = chunk_markdown("# One\n\nalpha\n\n# Two\n\nbeta");
        assert_chunk(&chunks[0], "# One", 0, 5, &[]);
        assert_chunk(&chunks[1], "alpha", 7, 12, &["One"]);
        assert_chunk(&chunks[2], "# Two", 14, 19, &[]);
        assert_chunk(&chunks[3], "beta", 21, 25, &["Two"]);
        assert!(!chunks[1].text.contains("Two"));
        assert!(!chunks[1].text.contains("beta"));
        assert!(!chunks[3].text.contains("alpha"));
        assert!(!chunks[3].text.contains("One"));

        let text = format!("# One\n\n{}\n\n# Two\n\nbeta", "x".repeat(513));
        let chunks = chunk_markdown(&text);
        assert_eq!(chunks.len(), 5);
        assert_chunk(&chunks[0], "# One", 0, 5, &[]);
        assert_eq!((chunks[1].start, chunks[1].end), (7, 519));
        assert_eq!((chunks[2].start, chunks[2].end), (455, 520));
        assert_eq!(chunks[1].heading_path, vec!["One".to_string()]);
        assert_eq!(chunks[2].heading_path, vec!["One".to_string()]);
        assert!(chunks[1].text.chars().all(|c| c == 'x'));
        assert!(chunks[2].text.chars().all(|c| c == 'x'));
        assert!(!chunks[1].text.contains('#'));
        assert!(!chunks[2].text.contains('#'));
        assert_chunk(&chunks[3], "# Two", 522, 527, &[]);
        assert_chunk(&chunks[4], "beta", 529, 533, &["Two"]);
    }

    #[test]
    fn tc_140_long_heading_stays_one_chunk() {
        let text = format!("# {}", "h".repeat(600));
        let chunks = chunk_markdown(&text);
        assert_eq!(chunks.len(), 1);
        assert_chunk(&chunks[0], &text, 0, 602, &[]);
    }

    #[test]
    fn tc_176_long_fence_splits_inside_the_fence() {
        let fence = format!("```\n{}\n```", "f".repeat(600));
        let text = format!("# F\n\n{fence}");
        let chunks = chunk_markdown(&text);
        assert_eq!(chunks.len(), 4);
        assert_chunk(&chunks[0], "# F", 0, 3, &[]);
        assert_eq!((chunks[1].start, chunks[1].end), (5, 9));
        assert_eq!(chunks[1].text, "```\n");
        assert_eq!((chunks[2].start, chunks[2].end), (9, 9 + CHUNK_LIMIT));
        assert_eq!(
            (chunks[3].start, chunks[3].end),
            (9 + CHUNK_LIMIT - OVERLAP, 613)
        );
        assert!(chunks[2].text.chars().all(|c| c == 'f'));
        assert!(chunks[3].text.ends_with("```"));
        assert_eq!(chunks[1].heading_path, vec!["F".to_string()]);
        assert_eq!(chunks[3].heading_path, vec!["F".to_string()]);
        assert!(!chunks[1].text.contains("# F"));
        assert!(!chunks[3].text.contains("# F"));
        assert!(chunks[1].meta_json.is_none());
        assert!(chunks[3].meta_json.is_none());
    }

    #[test]
    fn tc_187_long_indented_block_splits_inside_the_block() {
        let text = format!("    {}", "y".repeat(CHUNK_LIMIT + 1));
        let chunks = chunk_markdown(&text);
        assert_eq!(chunks.len(), 3);
        assert_eq!((chunks[0].start, chunks[0].end), (0, 4));
        assert_eq!(chunks[0].text, "    ");
        assert_eq!((chunks[1].start, chunks[1].end), (4, 4 + CHUNK_LIMIT));
        assert_eq!(
            (chunks[2].start, chunks[2].end),
            (4 + CHUNK_LIMIT - OVERLAP, 4 + CHUNK_LIMIT + 1)
        );
        assert!(chunks[1].text.chars().all(|c| c == 'y'));
        assert!(chunks[2].text.chars().all(|c| c == 'y'));
        assert!(chunks[0].heading_path.is_empty());
        assert!(chunks[2].heading_path.is_empty());
        assert!(chunks[0].meta_json.is_none());
    }
}
