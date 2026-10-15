//! Markdown cuts through the virtual table. Cases are numbered in TESTSPEC.md.

#![allow(dead_code)]

mod common;

use common::{cell, ok};

#[test]
fn tc_21_update_format_to_markdown_rechunks() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('before' || char(10) || char(10) || '```' || char(10) || 'code' || char(10) || char(10) || 'more' || char(10) || '```' || char(10) || char(10) || 'after');
         SELECT text, format FROM docs;
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(
        cell(&rows, 0, "text"),
        "before\n\n```\ncode\n\nmore\n```\n\nafter"
    );
    assert_eq!(cell(&rows, 0, "format"), "text/plain");
    assert_eq!(cell(&rows, 1, "chunk"), "before");
    assert_eq!(cell(&rows, 2, "chunk"), "```\ncode");
    assert_eq!(cell(&rows, 3, "chunk"), "more\n```");
    assert_eq!(cell(&rows, 4, "chunk"), "after");
    assert_eq!(cell(&rows, 1, "heading_path"), "[]");
    assert_eq!(cell(&rows, 3, "heading_path"), "[]");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text) VALUES ('before' || char(10) || char(10) || '```' || char(10) || 'code' || char(10) || char(10) || 'more' || char(10) || '```' || char(10) || char(10) || 'after');
         UPDATE docs SET format = 'text/markdown' WHERE doc_id = 1;
         SELECT text, format FROM docs;
         SELECT chunk, heading_path, meta_json FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(
        cell(&rows, 0, "text"),
        "before\n\n```\ncode\n\nmore\n```\n\nafter"
    );
    assert_eq!(cell(&rows, 0, "format"), "text/markdown");
    assert_eq!(rows.as_array().unwrap().len(), 4);
    assert_eq!(cell(&rows, 1, "chunk"), "before");
    assert_eq!(cell(&rows, 1, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[1]["meta_json"].is_null());
    assert_eq!(cell(&rows, 2, "chunk"), "```\ncode\n\nmore\n```");
    assert_eq!(cell(&rows, 2, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[2]["meta_json"].is_null());
    assert_eq!(cell(&rows, 3, "chunk"), "after");
    assert_eq!(cell(&rows, 3, "heading_path"), "[]");
}

#[test]
fn tc_46_tc_97_tc_138_tc_141_heading_chunks_and_empty_title() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# Only', 'text/markdown');
         SELECT d.text AS text, c.chunk AS chunk, c.heading_path AS heading_path, c.meta_json AS meta_json, c.start AS start, c.end AS end
         FROM docs_chunks c JOIN docs d ON d.doc_id = c.doc_id;",
    );
    assert_eq!(cell(&rows, 0, "text"), "# Only");
    assert_eq!(cell(&rows, 0, "chunk"), "# Only");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[0]["meta_json"].is_null());
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "6");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# ', 'text/markdown');
         SELECT text FROM docs;
         SELECT count(*) AS n FROM docs_chunks;");
    assert_eq!(cell(&rows, 0, "text"), "# ");
    assert_eq!(cell(&rows, 1, "n"), "0");
}

#[test]
fn tc_139_tc_168_tc_169_tc_201_paths_and_heading_line() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('## Title' || char(10) || char(10) || 'Body', 'text/markdown');
         SELECT chunk, start, \"end\", heading_path, meta_json FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "## Title");
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "8");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[0]["meta_json"].is_null());
    assert_eq!(cell(&rows, 1, "chunk"), "Body");
    assert_eq!(cell(&rows, 1, "start"), "10");
    assert_eq!(cell(&rows, 1, "end"), "14");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"Title\"]");
    assert!(rows.as_array().unwrap()[1]["meta_json"].is_null());

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('  ## Title ##', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks;");
    assert_eq!(cell(&rows, 0, "chunk"), "  ## Title ##");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# A' || char(10) || '## B' || char(10) || 'body', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "# A");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "chunk"), "## B");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"A\"]");
    assert_eq!(cell(&rows, 2, "chunk"), "body");
    assert_eq!(cell(&rows, 2, "heading_path"), "[\"A\",\"B\"]");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# Above' || char(10) || char(10) || '## Title' || char(10) || char(10) || 'Body', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "# Above");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "chunk"), "## Title");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"Above\"]");
    assert_eq!(cell(&rows, 2, "chunk"), "Body");
    assert_eq!(cell(&rows, 2, "heading_path"), "[\"Above\",\"Title\"]");
}

#[test]
fn tc_162_tc_202_blank_lines_and_non_headings() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# H' || char(10) || char(10) || 'a' || char(10) || 'b' || char(10) || char(10) || 'c', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "# H");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "chunk"), "a\nb");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"H\"]");
    assert_eq!(cell(&rows, 2, "chunk"), "c");
    assert_eq!(cell(&rows, 2, "heading_path"), "[\"H\"]");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('    # Title' || char(10) || char(10) || 'Body', 'text/markdown');
         INSERT INTO docs(text, format) VALUES ('####### Title' || char(10) || char(10) || 'Body', 'text/markdown');
         INSERT INTO docs(text, format) VALUES ('#NoSpace' || char(10) || char(10) || 'Body', 'text/markdown');
         SELECT doc_id, chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "    # Title");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "chunk"), "Body");
    assert_eq!(cell(&rows, 1, "heading_path"), "[]");
    assert_eq!(cell(&rows, 2, "chunk"), "####### Title");
    assert_eq!(cell(&rows, 2, "heading_path"), "[]");
    assert_eq!(cell(&rows, 3, "chunk"), "Body");
    assert_eq!(cell(&rows, 3, "heading_path"), "[]");
    assert_eq!(cell(&rows, 4, "chunk"), "#NoSpace");
    assert_eq!(cell(&rows, 4, "heading_path"), "[]");
    assert_eq!(cell(&rows, 5, "chunk"), "Body");
    assert_eq!(cell(&rows, 5, "heading_path"), "[]");
}

#[test]
fn tc_175_tc_186_tc_228_tc_229_tc_230_fences_and_indented_blocks() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('before' || char(10) || char(10) || '```rust' || char(10) || 'line' || char(10) || char(10) || '# not a heading' || char(10) || '```' || char(10) || char(10) || 'after', 'text/markdown');
         SELECT chunk, start, end, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "before");
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "6");
    assert_eq!(
        cell(&rows, 1, "chunk"),
        "```rust\nline\n\n# not a heading\n```"
    );
    assert_eq!(cell(&rows, 1, "start"), "8");
    assert_eq!(cell(&rows, 1, "end"), "41");
    assert_eq!(cell(&rows, 2, "chunk"), "after");
    assert_eq!(cell(&rows, 2, "start"), "43");
    assert_eq!(cell(&rows, 2, "end"), "48");
    assert_eq!(cell(&rows, 1, "heading_path"), "[]");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('    a' || char(10) || char(10) || '    # b', 'text/markdown');
         SELECT chunk FROM docs_chunks;",
    );
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(cell(&rows, 0, "chunk"), "    a\n\n    # b");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('```' || char(10) || 'keep' || char(10) || '````' || char(10) || char(10) || 'tail', 'text/markdown');
         SELECT chunk FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "```\nkeep\n````");
    assert_eq!(cell(&rows, 1, "chunk"), "tail");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('```' || char(10) || 'keep' || char(10) || '``` not' || char(10) || 'still' || char(10) || '```' || char(10) || char(10) || 'out', 'text/markdown');
         SELECT chunk FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "```\nkeep\n``` not\nstill\n```");
    assert_eq!(cell(&rows, 1, "chunk"), "out");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('```' || char(10) || '~~~' || char(10) || 'inside' || char(10) || '```' || char(10) || char(10) || 'after', 'text/markdown');
         INSERT INTO docs(text, format) VALUES ('~~~' || char(10) || '```' || char(10) || 'inside' || char(10) || '~~~' || char(10) || char(10) || 'after', 'text/markdown');
         SELECT doc_id, chunk FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 0, "chunk"), "```\n~~~\ninside\n```");
    assert_eq!(cell(&rows, 1, "chunk"), "after");
    assert_eq!(cell(&rows, 2, "chunk"), "~~~\n```\ninside\n~~~");
    assert_eq!(cell(&rows, 3, "chunk"), "after");
}

#[test]
fn tc_86_tc_148_chunks_stay_in_cut_order() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('intro' || char(10) || char(10) || '# First' || char(10) || char(10) || 'para one' || char(10) || char(10) || 'para two' || char(10) || char(10) || '    code' || char(10) || char(10) || '```' || char(10) || 'fence' || char(10) || '```' || char(10) || char(10) || '# Second', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    let chunks = [
        ("intro", "[]"),
        ("# First", "[]"),
        ("para one", "[\"First\"]"),
        ("para two", "[\"First\"]"),
        ("    code", "[\"First\"]"),
        ("```\nfence\n```", "[\"First\"]"),
        ("# Second", "[]"),
    ];
    assert_eq!(rows.as_array().unwrap().len(), chunks.len());
    for (index, (text, path)) in chunks.iter().enumerate() {
        assert_eq!(cell(&rows, index, "chunk"), *text);
        assert_eq!(cell(&rows, index, "heading_path"), *path);
    }

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# One' || char(10) || char(10) || 'alpha' || char(10) || char(10) || '# Two' || char(10) || char(10) || 'beta', 'text/markdown');
         SELECT chunk, heading_path FROM docs_chunks ORDER BY rowid;",
    );
    assert_eq!(cell(&rows, 1, "chunk"), "alpha");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"One\"]");
    assert!(!cell(&rows, 1, "chunk").contains("Two"));
    assert!(!cell(&rows, 1, "chunk").contains("beta"));
    assert_eq!(cell(&rows, 3, "chunk"), "beta");
    assert!(!cell(&rows, 3, "chunk").contains("alpha"));
    assert!(!cell(&rows, 3, "chunk").contains("One"));
}

#[test]
fn tc_183_body_is_not_an_fts_hit_for_the_heading() {
    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('# Zephyr' || char(10) || char(10) || 'plain words', 'text/markdown');
         SELECT c.text AS chunk
         FROM docs_fts
         JOIN docs_chunk_rows c ON c.chunk_id = docs_fts.rowid
         WHERE docs_fts MATCH 'Zephyr';
         SELECT c.text AS chunk
         FROM docs_fts
         JOIN docs_chunk_rows c ON c.chunk_id = docs_fts.rowid
         WHERE docs_fts MATCH 'words';",
    );
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(cell(&rows, 0, "chunk"), "# Zephyr");
    assert_eq!(cell(&rows, 1, "chunk"), "plain words");
}
