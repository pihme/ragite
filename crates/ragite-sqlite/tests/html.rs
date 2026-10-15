//! HTML document cuts through the sqlite3 CLI. Cases are numbered in TESTSPEC.md.

mod common;

use common::{cell, ok};

fn chunk_at(rows: &serde_json::Value, index: usize) -> String {
    cell(rows, index, "chunk")
}

#[test]
fn tc_95_script_and_style_only_is_stored_without_chunks() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source, format)
         VALUES ('<script>secret</script><style>x</style>', 'scripts', 'text/html');
         INSERT INTO docs(text, source, format)
         VALUES ('<SCRIPT>secret</SCRIPT><STYLE>x</STYLE>', 'scripts-upper', 'text/html');
         SELECT text, source FROM docs WHERE source = 'scripts';
         SELECT count(*) AS n FROM docs;
         SELECT count(*) AS n FROM docs_chunks;
         SELECT count(*) AS n FROM docs_vec;
         SELECT count(*) AS n FROM docs_fts;
         SELECT count(*) AS n FROM docs WHERE docs MATCH 'secret';
         SELECT count(*) AS n FROM docs_chunks WHERE docs_chunks MATCH 'secret';");
    assert_eq!(
        cell(&rows, 0, "text"),
        "<script>secret</script><style>x</style>"
    );
    assert_eq!(cell(&rows, 0, "source"), "scripts");
    assert_eq!(cell(&rows, 1, "n"), "2");
    assert_eq!(cell(&rows, 2, "n"), "0");
    assert_eq!(cell(&rows, 3, "n"), "0");
    assert_eq!(cell(&rows, 4, "n"), "0");
    assert_eq!(cell(&rows, 5, "n"), "0");
    assert_eq!(cell(&rows, 6, "n"), "0");
}

#[test]
fn tc_46_tc_97_tc_138_tc_139_tc_141_tc_170_headings() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<h1>Only</h1>', 'text/html');
         SELECT chunk, start, end, heading_path, meta_json FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Only");
    assert_eq!(cell(&rows, 0, "start"), "4");
    assert_eq!(cell(&rows, 0, "end"), "8");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert!(rows.as_array().unwrap()[0]["meta_json"].is_null());

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<h1>Title</h1><p>Body</p>', 'text/html');
         SELECT chunk, heading_path, meta_json FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Title");
    assert_eq!(chunk_at(&rows, 1), "Body");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"Title\"]");
    assert!(rows.as_array().unwrap()[0]["meta_json"].is_null());
    assert!(rows.as_array().unwrap()[1]["meta_json"].is_null());

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<h1></h1>', 'text/html');
         INSERT INTO docs(text, format) VALUES ('<h2>   </h2>', 'text/html');
         SELECT text FROM docs ORDER BY doc_id;
         SELECT count(*) AS n FROM docs_chunks;");
    assert_eq!(cell(&rows, 0, "text"), "<h1></h1>");
    assert_eq!(cell(&rows, 1, "text"), "<h2>   </h2>");
    assert_eq!(cell(&rows, 2, "n"), "0");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<h1>A <em>B</em></h1><p>C</p>', 'text/html');
         SELECT chunk, heading_path FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "A B");
    assert_eq!(chunk_at(&rows, 1), "C");
    assert_eq!(cell(&rows, 1, "heading_path"), "[\"A B\"]");
}

#[test]
fn tc_80_tc_197_tc_198_tc_233_offsets_and_whitespace() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>  Hello   \n  world  </p>', 'text/html');
         SELECT chunk, start, end FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Hello world");
    assert_eq!(cell(&rows, 0, "start"), "5");
    assert_eq!(cell(&rows, 0, "end"), "21");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<pre>  a \n b  </pre>', 'text/html');
         INSERT INTO docs(text, format) VALUES ('<textarea>  a \n b  </textarea>', 'text/html');
         SELECT chunk, start, end FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "  a \n b  ");
    assert_eq!(cell(&rows, 0, "start"), "5");
    assert_eq!(cell(&rows, 0, "end"), "14");
    assert_eq!(chunk_at(&rows, 1), "  a \n b  ");
    assert_eq!(cell(&rows, 1, "start"), "10");
    assert_eq!(cell(&rows, 1, "end"), "19");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>a😀b</p>', 'text/html');
         SELECT chunk, start, end FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "a😀b");
    assert_eq!(cell(&rows, 0, "start"), "3");
    assert_eq!(cell(&rows, 0, "end"), "6");
}

#[test]
fn tc_164_tc_165_tc_166_tc_167_tc_177_tc_178_paragraphs() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>a<br>b</p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(rows.as_array().unwrap().len(), 1);
    let text = chunk_at(&rows, 0);
    assert!(text.find('a').unwrap() < text.find('b').unwrap());

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES (
           '<p>P</p><li>L</li><pre>R</pre><blockquote>Q</blockquote><td>D</td><th>H</th><textarea>T</textarea>',
           'text/html');
         SELECT chunk, heading_path FROM docs_chunks;",
    );
    let got: Vec<String> = (0..7).map(|i| chunk_at(&rows, i)).collect();
    assert_eq!(got, vec!["P", "L", "R", "Q", "D", "H", "T"]);
    for index in 0..7 {
        assert_eq!(cell(&rows, index, "heading_path"), "[]");
    }

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<th>A</th><td>B</td>', 'text/html');
         SELECT chunk, heading_path FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "A");
    assert_eq!(chunk_at(&rows, 1), "B");
    assert_eq!(cell(&rows, 0, "heading_path"), "[]");
    assert_eq!(cell(&rows, 1, "heading_path"), "[]");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<div>before <p>in</p> after</div>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "before after");
    assert_eq!(chunk_at(&rows, 1), "in");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES (
           '<ul><li>u</li></ul><ol><li>o</li></ol><table><tr><td>c</td></tr></table><div><p>d</p></div>',
           'text/html');
         SELECT chunk FROM docs_chunks;",
    );
    let got: Vec<String> = (0..4).map(|i| chunk_at(&rows, i)).collect();
    assert_eq!(got, vec!["u", "o", "c", "d"]);

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<img alt=\"hidden\"><p title=\"nope\">Yes</p>', 'text/html');
         SELECT chunk FROM docs_chunks;",
    );
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(chunk_at(&rows, 0), "Yes");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format)
         VALUES ('<p>See <a>this</a> <em>and</em> <code>x</code></p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "See this and x");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format)
         VALUES ('<p>Before <h2>Mid</h2> after <span>end</span></p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Before after end");
    assert_eq!(chunk_at(&rows, 1), "Mid");
}

#[test]
fn tc_209_element_names_ignore_letter_case() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES (
           '<H1>Title</H1><P>Body</P><SCRIPT>no</SCRIPT><STYLE>no</STYLE><PRE> keep </PRE>',
           'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Title");
    assert_eq!(chunk_at(&rows, 1), "Body");
    assert_eq!(chunk_at(&rows, 2), " keep ");
    assert_eq!(rows.as_array().unwrap().len(), 3);
}

#[test]
fn tc_234_title_is_a_collapsing_container() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<title>  Hello   there </title>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "Hello there");

    let rows = ok(
        "CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<title>Hello <p>In</p> there</title>', 'text/html');
         SELECT chunk FROM docs_chunks;",
    );
    assert_eq!(chunk_at(&rows, 0), "Hello there");
    assert_eq!(chunk_at(&rows, 1), "In");
}

#[test]
fn tc_149_tc_219_tc_220_tc_221_comments() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>A</p><!--x--><p>B</p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "A");
    assert_eq!(chunk_at(&rows, 1), "x");
    assert_eq!(chunk_at(&rows, 2), "B");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<!-- raw &amp; <b>no</b>\n-->', 'text/html');
         SELECT chunk, start, end FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), " raw &amp; <b>no</b>\n");
    assert_eq!(cell(&rows, 0, "start"), "0");
    assert_eq!(cell(&rows, 0, "end"), "28");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, source, format)
         VALUES ('<!----><!DOCTYPE html><?note ?>', 'markup', 'text/html');
         SELECT text FROM docs WHERE source = 'markup';
         SELECT count(*) AS n FROM docs_chunks;");
    assert_eq!(cell(&rows, 0, "text"), "<!----><!DOCTYPE html><?note ?>");
    assert_eq!(cell(&rows, 1, "n"), "0");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>A<!--x-->B</p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "AB");
    assert_eq!(chunk_at(&rows, 1), "x");
    assert!(!chunk_at(&rows, 0).contains('x'));
}

#[test]
fn tc_235_tc_236_character_references() {
    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format)
         VALUES ('<p>&amp; &#65; &#x42; &unknown;</p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), "& A B &unknown;");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<!-- &amp; -->', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(chunk_at(&rows, 0), " &amp; ");

    let rows = ok("CREATE VIRTUAL TABLE docs USING ragite;
         INSERT INTO docs(text, format) VALUES ('<p>&lt;!-- not --&gt;</p>', 'text/html');
         SELECT chunk FROM docs_chunks;");
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(chunk_at(&rows, 0), "<!-- not -->");
}
