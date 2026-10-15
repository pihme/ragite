//! HTML subset cutter for SPEC section 5.4. Not a general HTML parser.

use crate::chunk::{Chunk, split_range};

include!("html_entities.rs");

/// One decoded scalar and the stored-HTML span that produced it.
struct Atom {
    ch: char,
    start: usize,
    end: usize,
}

struct Heading {
    level: u8,
    title: String,
}

/// Loose text accumulated inside one container. `path` is the outline it belongs to.
struct Acc {
    atoms: Vec<Atom>,
    path: Vec<String>,
}

impl Acc {
    fn flush(&mut self, out: &mut Vec<Chunk>) {
        let atoms = collapse(std::mem::take(&mut self.atoms));
        emit_split(out, &atoms, &self.path);
    }
}

enum Node {
    Text {
        start: usize,
        end: usize,
    },
    Comment {
        text: String,
        start: usize,
        end: usize,
    },
    Element {
        name: String,
        start: usize,
        end: usize,
        children: Vec<Node>,
    },
}

pub(crate) fn cut(text: &str) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let nodes = parse(&chars);
    let mut outline = Vec::new();
    let mut out = Vec::new();
    process_container(&nodes, &chars, &mut outline, &mut out);
    // Document order follows where each piece starts in the stored HTML.
    out.sort_by_key(|chunk| chunk.start);
    out
}

fn titles(outline: &[Heading]) -> Vec<String> {
    outline
        .iter()
        .map(|heading| heading.title.clone())
        .collect()
}

fn sync_path(acc: &mut Acc, outline: &[Heading], out: &mut Vec<Chunk>) {
    let path = titles(outline);
    if path != acc.path {
        acc.flush(out);
        acc.path = path;
    }
}

fn process_container(
    nodes: &[Node],
    chars: &[char],
    outline: &mut Vec<Heading>,
    out: &mut Vec<Chunk>,
) {
    let mut acc = Acc {
        atoms: Vec::new(),
        path: titles(outline),
    };
    walk_sequence(nodes, chars, outline, out, &mut acc);
    acc.flush(out);
}

fn walk_sequence(
    nodes: &[Node],
    chars: &[char],
    outline: &mut Vec<Heading>,
    out: &mut Vec<Chunk>,
    acc: &mut Acc,
) {
    for node in nodes {
        match node {
            Node::Text { start, end } => acc.atoms.extend(decode(chars, *start, *end)),
            Node::Comment { text, start, end } => emit_comment(out, text, *start, *end, outline),
            Node::Element {
                name,
                start,
                end,
                children,
            } => {
                if is_dropped(name) {
                    continue;
                }
                if name == "br" {
                    acc.atoms.push(Atom {
                        ch: '\n',
                        start: *start,
                        end: *end,
                    });
                    continue;
                }
                if let Some(level) = heading_level(name) {
                    // A heading opens a section, so loose text before it is its own paragraph.
                    acc.flush(out);
                    process_heading(level, children, chars, outline, out);
                    acc.path = titles(outline);
                    continue;
                }
                if is_paragraph(name) {
                    process_paragraph(name, children, chars, outline, out);
                    sync_path(acc, outline, out);
                    continue;
                }
                if is_inline(name) {
                    walk_sequence(children, chars, outline, out, acc);
                    sync_path(acc, outline, out);
                    continue;
                }
                process_container(children, chars, outline, out);
                sync_path(acc, outline, out);
            }
        }
    }
}

fn process_heading(
    level: u8,
    children: &[Node],
    chars: &[char],
    outline: &mut Vec<Heading>,
    out: &mut Vec<Chunk>,
) {
    let mut atoms = Vec::new();
    collect_absorbed(children, chars, &mut atoms);
    let atoms = collapse(atoms);
    while outline.last().is_some_and(|heading| heading.level >= level) {
        outline.pop();
    }
    if !atoms.is_empty() {
        emit_whole(out, &atoms, &titles(outline));
        let title: String = atoms.iter().map(|atom| atom.ch).collect();
        outline.push(Heading { level, title });
    }
    walk_absorbed(children, chars, outline, out);
}

fn process_paragraph(
    name: &str,
    children: &[Node],
    chars: &[char],
    outline: &mut Vec<Heading>,
    out: &mut Vec<Chunk>,
) {
    let mut atoms = Vec::new();
    collect_absorbed(children, chars, &mut atoms);
    if name != "pre" && name != "textarea" {
        atoms = collapse(atoms);
    }
    emit_split(out, &atoms, &titles(outline));
    walk_absorbed(children, chars, outline, out);
}

/// Inside a heading or paragraph the parent already owns the inline text.
fn walk_absorbed(nodes: &[Node], chars: &[char], outline: &mut Vec<Heading>, out: &mut Vec<Chunk>) {
    for node in nodes {
        match node {
            Node::Comment { text, start, end } => emit_comment(out, text, *start, *end, outline),
            Node::Text { .. } => {}
            Node::Element { name, children, .. } => {
                if is_dropped(name) || name == "br" {
                    continue;
                }
                if let Some(level) = heading_level(name) {
                    process_heading(level, children, chars, outline, out);
                    continue;
                }
                if is_paragraph(name) {
                    process_paragraph(name, children, chars, outline, out);
                    continue;
                }
                walk_absorbed(children, chars, outline, out);
            }
        }
    }
}

fn collect_absorbed(nodes: &[Node], chars: &[char], atoms: &mut Vec<Atom>) {
    for node in nodes {
        match node {
            Node::Text { start, end } => atoms.extend(decode(chars, *start, *end)),
            Node::Comment { .. } => {}
            Node::Element {
                name,
                start,
                end,
                children,
            } => {
                if is_dropped(name) || heading_level(name).is_some() || is_paragraph(name) {
                    continue;
                }
                if name == "br" {
                    atoms.push(Atom {
                        ch: '\n',
                        start: *start,
                        end: *end,
                    });
                    continue;
                }
                collect_absorbed(children, chars, atoms);
            }
        }
    }
}

fn emit_comment(out: &mut Vec<Chunk>, text: &str, start: usize, end: usize, outline: &[Heading]) {
    if text.is_empty() {
        return;
    }
    push(out, text.to_string(), start, end, &titles(outline));
}

fn emit_whole(out: &mut Vec<Chunk>, atoms: &[Atom], path: &[String]) {
    if atoms.is_empty() {
        return;
    }
    let text: String = atoms.iter().map(|atom| atom.ch).collect();
    push(out, text, atoms[0].start, atoms[atoms.len() - 1].end, path);
}

fn emit_split(out: &mut Vec<Chunk>, atoms: &[Atom], path: &[String]) {
    if atoms.is_empty() {
        return;
    }
    for (from, to) in split_range(0, atoms.len()) {
        let text: String = atoms[from..to].iter().map(|atom| atom.ch).collect();
        push(out, text, atoms[from].start, atoms[to - 1].end, path);
    }
}

fn push(out: &mut Vec<Chunk>, text: String, start: usize, end: usize, path: &[String]) {
    if text.is_empty() {
        return;
    }
    out.push(Chunk {
        text,
        start,
        end,
        heading_path: path.to_vec(),
        meta_json: None,
    });
}

fn is_ws(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}

fn collapse(atoms: Vec<Atom>) -> Vec<Atom> {
    let mut out = Vec::new();
    let mut pending: Option<(usize, usize)> = None;
    for atom in atoms {
        if is_ws(atom.ch) {
            pending = Some(match pending {
                Some((start, _)) => (start, atom.end),
                None => (atom.start, atom.end),
            });
            continue;
        }
        if let Some((start, end)) = pending.take() {
            if !out.is_empty() {
                out.push(Atom {
                    ch: ' ',
                    start,
                    end,
                });
            }
        }
        out.push(atom);
    }
    out
}

fn is_dropped(name: &str) -> bool {
    name == "script" || name == "style"
}

fn is_paragraph(name: &str) -> bool {
    matches!(
        name,
        "p" | "li" | "pre" | "blockquote" | "td" | "th" | "textarea"
    )
}

fn heading_level(name: &str) -> Option<u8> {
    match name {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn is_inline(name: &str) -> bool {
    matches!(
        name,
        "a" | "abbr"
            | "b"
            | "bdi"
            | "bdo"
            | "big"
            | "button"
            | "cite"
            | "code"
            | "data"
            | "del"
            | "dfn"
            | "em"
            | "font"
            | "i"
            | "ins"
            | "kbd"
            | "label"
            | "mark"
            | "nobr"
            | "q"
            | "rp"
            | "rt"
            | "ruby"
            | "s"
            | "samp"
            | "small"
            | "span"
            | "strike"
            | "strong"
            | "sub"
            | "sup"
            | "time"
            | "tt"
            | "u"
            | "var"
            | "wbr"
    )
}

fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn decode(chars: &[char], start: usize, end: usize) -> Vec<Atom> {
    let mut atoms = Vec::new();
    let mut index = start;
    while index < end {
        if chars[index] == '&' {
            if let Some((value, next)) = character_reference(chars, index, end) {
                if next > index {
                    for ch in value.chars() {
                        atoms.push(Atom {
                            ch,
                            start: index,
                            end: next,
                        });
                    }
                    index = next;
                    continue;
                }
            }
        }
        atoms.push(Atom {
            ch: chars[index],
            start: index,
            end: index + 1,
        });
        index += 1;
    }
    atoms
}

fn character_reference(chars: &[char], start: usize, end: usize) -> Option<(String, usize)> {
    let mut index = start + 1;
    if index >= end {
        return None;
    }
    if chars[index] == '#' {
        index += 1;
        let hex = if index < end && (chars[index] == 'x' || chars[index] == 'X') {
            index += 1;
            true
        } else {
            false
        };
        let digits = index;
        while index < end
            && if hex {
                chars[index].is_ascii_hexdigit()
            } else {
                chars[index].is_ascii_digit()
            }
        {
            index += 1;
        }
        if index == digits {
            return None;
        }
        let mut next = index;
        if index < end && chars[index] == ';' {
            next = index + 1;
        }
        let number: String = chars[digits..index].iter().collect();
        let cp = if hex {
            u32::from_str_radix(&number, 16).unwrap_or(0x11_0000)
        } else {
            number.parse().unwrap_or(0x11_0000)
        };
        return Some((from_numeric(cp).to_string(), next));
    }
    let name_at = index;
    let mut legacy = None;
    while index < end && chars[index].is_ascii_alphanumeric() {
        index += 1;
        let name: String = chars[name_at..index].iter().collect();
        if is_legacy_name(&name) {
            if let Some(value) = named_entity(&name) {
                legacy = Some((value.to_string(), index));
            }
        }
    }
    if index < end && chars[index] == ';' {
        let name: String = chars[name_at..index].iter().collect();
        if let Some(value) = named_entity(&name) {
            return Some((value.to_string(), index + 1));
        }
    }
    legacy
}

/// Names the HTML table accepts without a trailing semicolon.
fn is_legacy_name(name: &str) -> bool {
    const LEGACY: &[&str] = &[
        "AElig", "AMP", "Aacute", "Acirc", "Agrave", "Aring", "Atilde", "Auml", "COPY", "Ccedil",
        "ETH", "Eacute", "Ecirc", "Egrave", "Euml", "GT", "Iacute", "Icirc", "Igrave", "Iuml", "LT",
        "Ntilde", "Oacute", "Ocirc", "Ograve", "Oslash", "Otilde", "Ouml", "QUOT", "REG", "THORN",
        "Uacute", "Ucirc", "Ugrave", "Uuml", "Yacute", "aacute", "acirc", "acute", "aelig", "agrave",
        "amp", "aring", "atilde", "auml", "brvbar", "ccedil", "cedil", "cent", "copy", "curren",
        "deg", "divide", "eacute", "ecirc", "egrave", "eth", "euml", "frac12", "frac14", "frac34",
        "gt", "iacute", "icirc", "iexcl", "igrave", "iquest", "iuml", "laquo", "lt", "macr",
        "micro", "middot", "nbsp", "not", "ntilde", "oacute", "ocirc", "ograve", "ordf", "ordm",
        "oslash", "otilde", "ouml", "para", "plusmn", "pound", "quot", "raquo", "reg", "sect",
        "shy", "sup1", "sup2", "sup3", "szlig", "thorn", "times", "uacute", "ucirc", "ugrave",
        "uml", "uuml", "yacute", "yen", "yuml",
    ];
    LEGACY.binary_search(&name).is_ok()
}

/// HTML numeric references, including the C1 Windows-1252 remap.
fn from_numeric(cp: u32) -> char {
    let cp = match cp {
        0x00 => 0xFFFD,
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        other => other,
    };
    if (0xD800..=0xDFFF).contains(&cp) || cp > 0x10FFFF {
        '\u{FFFD}'
    } else {
        char::from_u32(cp).unwrap_or('\u{FFFD}')
    }
}

struct Parser<'a> {
    chars: &'a [char],
    i: usize,
}

fn parse(chars: &[char]) -> Vec<Node> {
    let mut parser = Parser { chars, i: 0 };
    let mut nodes = Vec::new();
    parser.parse_children(None, &mut nodes);
    nodes
}

impl<'a> Parser<'a> {
    fn parse_children(&mut self, expected: Option<&str>, out: &mut Vec<Node>) {
        while self.i < self.chars.len() {
            if self.starts_with("</") {
                if let Some(expected) = expected {
                    let save = self.i;
                    self.i += 2;
                    let close = self.parse_name().to_ascii_lowercase();
                    if close == expected {
                        self.skip_to_tag_end();
                        return;
                    }
                    self.i = save;
                    return;
                }
                self.i += 2;
                self.parse_name();
                self.skip_to_tag_end();
                continue;
            }
            if self.starts_with("<!--") {
                out.push(self.parse_comment());
                continue;
            }
            if self.starts_with("<!") {
                self.skip_declaration();
                continue;
            }
            if self.starts_with("<?") {
                self.skip_until_gt();
                continue;
            }
            if self.at_tag() {
                out.push(self.parse_element());
                continue;
            }
            out.push(self.parse_text());
        }
    }

    fn parse_element(&mut self) -> Node {
        let start = self.i;
        self.i += 1;
        let name = self.parse_name().to_ascii_lowercase();
        let self_closing = self.skip_to_tag_end();
        if self_closing || is_void(&name) {
            return Node::Element {
                name,
                start,
                end: self.i,
                children: Vec::new(),
            };
        }
        if is_dropped(&name) {
            // Inner markup is not structure. The element ends at its own end tag.
            self.skip_raw(&name);
            return Node::Element {
                name,
                start,
                end: self.i,
                children: Vec::new(),
            };
        }
        let mut children = Vec::new();
        self.parse_children(Some(&name), &mut children);
        Node::Element {
            name,
            start,
            end: self.i,
            children,
        }
    }

    fn parse_comment(&mut self) -> Node {
        let start = self.i;
        self.i += 4;
        let body_start = self.i;
        while self.i + 2 < self.chars.len()
            && !(self.chars[self.i] == '-'
                && self.chars[self.i + 1] == '-'
                && self.chars[self.i + 2] == '>')
        {
            self.i += 1;
        }
        let body_end = self.i;
        if self.i + 2 < self.chars.len() {
            self.i += 3;
        } else {
            self.i = self.chars.len();
        }
        let text: String = self.chars[body_start..body_end].iter().collect();
        Node::Comment {
            text,
            start,
            end: self.i,
        }
    }

    fn parse_text(&mut self) -> Node {
        let start = self.i;
        while self.i < self.chars.len() && !self.at_markup() {
            self.i += 1;
        }
        if self.i == start && self.i < self.chars.len() {
            self.i += 1;
        }
        Node::Text { start, end: self.i }
    }

    fn parse_name(&mut self) -> String {
        let start = self.i;
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == ':' {
                self.i += 1;
            } else {
                break;
            }
        }
        self.chars[start..self.i].iter().collect()
    }

    fn skip_to_tag_end(&mut self) -> bool {
        let mut quote = None;
        let mut self_closing = false;
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            self.i += 1;
            if let Some(mark) = quote {
                if ch == mark {
                    quote = None;
                }
                continue;
            }
            match ch {
                '"' | '\'' => quote = Some(ch),
                '/' => self_closing = true,
                '>' => return self_closing,
                _ => {}
            }
        }
        self_closing
    }

    fn skip_raw(&mut self, name: &str) {
        let end_name: Vec<char> = name.chars().collect();
        while self.i < self.chars.len() {
            if self.chars[self.i] == '<' && self.chars.get(self.i + 1) == Some(&'/') {
                let mut cursor = self.i + 2;
                let mut matched = 0;
                while matched < end_name.len()
                    && self
                        .chars
                        .get(cursor)
                        .is_some_and(|ch| ch.to_ascii_lowercase() == end_name[matched])
                {
                    cursor += 1;
                    matched += 1;
                }
                let boundary = self
                    .chars
                    .get(cursor)
                    .is_none_or(|ch| !ch.is_ascii_alphanumeric() && *ch != '-' && *ch != ':');
                if matched == end_name.len() && boundary {
                    self.i = cursor;
                    self.skip_to_tag_end();
                    return;
                }
            }
            self.i += 1;
        }
    }

    fn skip_declaration(&mut self) {
        self.i += 2;
        let mut quote = None;
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            self.i += 1;
            if let Some(mark) = quote {
                if ch == mark {
                    quote = None;
                }
                continue;
            }
            match ch {
                '"' | '\'' => quote = Some(ch),
                '>' => return,
                _ => {}
            }
        }
    }

    fn skip_until_gt(&mut self) {
        self.i += 2;
        while self.i < self.chars.len() {
            let ch = self.chars[self.i];
            self.i += 1;
            if ch == '>' {
                return;
            }
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        let mut cursor = self.i;
        for ch in prefix.chars() {
            if self.chars.get(cursor) != Some(&ch) {
                return false;
            }
            cursor += 1;
        }
        true
    }

    fn at_tag(&self) -> bool {
        self.chars.get(self.i) == Some(&'<')
            && self
                .chars
                .get(self.i + 1)
                .is_some_and(|ch| ch.is_ascii_alphabetic())
    }

    fn at_markup(&self) -> bool {
        if self.chars.get(self.i) != Some(&'<') {
            return false;
        }
        match self.chars.get(self.i + 1) {
            Some('!' | '?' | '/') => true,
            Some(ch) => ch.is_ascii_alphabetic(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CHUNK_LIMIT, OVERLAP, chunk_html};

    fn texts(chunks: &[Chunk]) -> Vec<&str> {
        chunks.iter().map(|chunk| chunk.text.as_str()).collect()
    }

    fn paths(chunks: &[Chunk]) -> Vec<Vec<String>> {
        chunks
            .iter()
            .map(|chunk| chunk.heading_path.clone())
            .collect()
    }

    fn assert_no_meta(chunks: &[Chunk]) {
        for chunk in chunks {
            assert!(chunk.meta_json.is_none());
        }
    }

    #[test]
    fn tc_95_script_and_style_yield_no_chunks() {
        assert!(chunk_html("<script>secret</script><style>x</style>").is_empty());
        assert!(chunk_html("<SCRIPT>secret</SCRIPT><STYLE>x</STYLE>").is_empty());
        assert!(chunk_html("<script><p>secret</p></script>").is_empty());
    }

    #[test]
    fn tc_46_tc_97_tc_138_tc_139_tc_141_tc_170_headings() {
        let only = chunk_html("<h1>Only</h1>");
        assert_eq!(texts(&only), vec!["Only"]);
        assert_eq!(only[0].start, 4);
        assert_eq!(only[0].end, 8);
        assert!(only[0].heading_path.is_empty());
        assert_no_meta(&only);

        let pair = chunk_html("<h1>Title</h1><p>Body</p>");
        assert_eq!(texts(&pair), vec!["Title", "Body"]);
        assert_eq!(paths(&pair), vec![vec![], vec!["Title".to_string()]]);
        assert_no_meta(&pair);

        assert!(chunk_html("<h1></h1>").is_empty());
        assert!(chunk_html("<h2>   </h2>").is_empty());

        let nested = chunk_html("<h1>A <em>B</em></h1><p>C</p>");
        assert_eq!(texts(&nested), vec!["A B", "C"]);
        assert_eq!(paths(&nested), vec![vec![], vec!["A B".to_string()]]);
    }

    #[test]
    fn tc_80_offsets_mark_the_stored_html_span() {
        let hello = chunk_html("<p>  Hello   \n  world  </p>");
        assert_eq!(texts(&hello), vec!["Hello world"]);
        assert_eq!(hello[0].start, 5);
        assert_eq!(hello[0].end, 21);

        let inner = chunk_html("<p>A<em>B</em></p>");
        assert_eq!(texts(&inner), vec!["AB"]);
        assert_eq!(inner[0].start, 3);
        assert_eq!(inner[0].end, 9);

        let emoji = chunk_html("<p>a😀b</p>");
        assert_eq!(emoji[0].text, "a😀b");
        assert_eq!(emoji[0].start, 3);
        assert_eq!(emoji[0].end, 6);

        let comment = chunk_html("<!-- raw &amp; <b>no</b>\n-->");
        assert_eq!(texts(&comment), vec![" raw &amp; <b>no</b>\n"]);
        assert_eq!(comment[0].start, 0);
        assert_eq!(comment[0].end, 28);
    }

    #[test]
    fn tc_164_tc_165_tc_166_tc_167_tc_177_tc_178_tc_263_tc_264_paragraphs() {
        let broken = chunk_html("<p>a<br>b</p>");
        assert_eq!(broken.len(), 1);
        let at = broken[0].text.find('a').unwrap();
        let bt = broken[0].text.find('b').unwrap();
        assert!(at < bt);

        let blocks = chunk_html(
            "<p>P</p><li>L</li><pre>R</pre><blockquote>Q</blockquote><td>D</td><th>H</th><textarea>T</textarea>",
        );
        assert_eq!(texts(&blocks), vec!["P", "L", "R", "Q", "D", "H", "T"]);
        assert!(blocks.iter().all(|chunk| chunk.heading_path.is_empty()));

        let cells = chunk_html("<th>A</th><td>B</td>");
        assert_eq!(texts(&cells), vec!["A", "B"]);
        assert!(cells.iter().all(|chunk| chunk.heading_path.is_empty()));

        let loose = chunk_html("<div>before <p>in</p> after</div>");
        assert_eq!(texts(&loose), vec!["before after", "in"]);

        let wrappers = chunk_html(
            "<ul><li>u</li></ul><ol><li>o</li></ol><table><tr><td>c</td></tr></table><div><p>d</p></div>",
        );
        assert_eq!(texts(&wrappers), vec!["u", "o", "c", "d"]);

        assert_eq!(
            texts(&chunk_html("<img alt=\"hidden\"><p title=\"nope\">Yes</p>")),
            vec!["Yes"]
        );
        assert_eq!(
            texts(&chunk_html(
                "<p>See <a>this</a> <em>and</em> <code>x</code></p>"
            )),
            vec!["See this and x"]
        );
        assert_eq!(
            texts(&chunk_html(
                "<p>Before <h2>Mid</h2> after <span>end</span></p>"
            )),
            vec!["Before after end", "Mid"]
        );

        assert_eq!(texts(&chunk_html("<div><em>a</em> b</div>")), vec!["a b"]);
        assert_eq!(texts(&chunk_html("<div>a</div>b")), vec!["a", "b"]);
        assert_eq!(texts(&chunk_html("<p title=\"a>b\">c</p>")), vec!["c"]);
    }

    #[test]
    fn tc_255_tc_262_deeper_headings_and_path_after_a_container() {
        let chunks = chunk_html("<h1>A</h1><h2>B</h2><h3>C</h3><h4>D</h4><h5>E</h5><h6>F</h6><p>G</p>");
        assert_eq!(texts(&chunks), vec!["A", "B", "C", "D", "E", "F", "G"]);
        assert_eq!(
            paths(&chunks),
            vec![
                vec![],
                vec!["A".to_string()],
                vec!["A".to_string(), "B".to_string()],
                vec!["A".to_string(), "B".to_string(), "C".to_string()],
                vec![
                    "A".to_string(),
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                ],
                vec![
                    "A".to_string(),
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                    "E".to_string(),
                ],
                vec![
                    "A".to_string(),
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                    "E".to_string(),
                    "F".to_string(),
                ],
            ]
        );

        let after = chunk_html("<div><h1>T</h1></div> after");
        assert_eq!(texts(&after), vec!["T", "after"]);
        assert_eq!(
            paths(&after),
            vec![vec![], vec!["T".to_string()]]
        );
    }

    #[test]
    fn tc_265_doctype_does_not_swallow_the_following_paragraph() {
        assert_eq!(texts(&chunk_html("<!DOCTYPE html><p>Hi</p>")), vec!["Hi"]);
        assert_eq!(texts(&chunk_html("<!DOCTYPE \"a>b\"><p>Hi</p>")), vec!["Hi"]);
    }

    #[test]
    fn tc_197_tc_198_tc_233_tc_234_whitespace() {
        assert_eq!(
            texts(&chunk_html("<p>  Hello   \n  world  </p>")),
            vec!["Hello world"]
        );
        let pre = chunk_html("<pre>  a \n b  </pre>");
        assert_eq!(texts(&pre), vec!["  a \n b  "]);
        assert_eq!(pre[0].start, 5);
        assert_eq!(pre[0].end, 14);

        let area = chunk_html("<textarea>  a \n b  </textarea>");
        assert_eq!(texts(&area), vec!["  a \n b  "]);
        assert_eq!(area[0].start, 10);
        assert_eq!(area[0].end, 19);

        assert_eq!(
            texts(&chunk_html("<title>  Hello   there </title>")),
            vec!["Hello there"]
        );
        assert_eq!(
            texts(&chunk_html("<title>Hello <p>In</p> there</title>")),
            vec!["Hello there", "In"]
        );
    }

    #[test]
    fn tc_209_element_names_ignore_letter_case() {
        let chunks = chunk_html(
            "<H1>Title</H1><P>Body</P><SCRIPT>no</SCRIPT><STYLE>no</STYLE><PRE> keep </PRE>",
        );
        assert_eq!(texts(&chunks), vec!["Title", "Body", " keep "]);
        assert_eq!(
            paths(&chunks),
            vec![vec![], vec!["Title".to_string()], vec!["Title".to_string()]]
        );
        assert_no_meta(&chunks);
    }

    #[test]
    fn tc_149_tc_219_tc_220_tc_221_comments() {
        assert_eq!(
            texts(&chunk_html("<p>A</p><!--x--><p>B</p>")),
            vec!["A", "x", "B"]
        );

        let raw = chunk_html("<!-- raw &amp; <b>no</b>\n-->");
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].text, " raw &amp; <b>no</b>\n");
        assert_eq!(raw[0].start, 0);
        assert_eq!(raw[0].end, 28);

        assert!(chunk_html("<!----><!DOCTYPE html><?note ?>").is_empty());

        let inside = chunk_html("<p>A<!--x-->B</p>");
        assert_eq!(texts(&inside), vec!["AB", "x"]);
        assert!(!inside[0].text.contains('x'));
    }

    #[test]
    fn tc_222_comment_keeps_the_heading_above_it() {
        let chunks = chunk_html("<h1>T</h1><!--note-->");
        assert_eq!(texts(&chunks), vec!["T", "note"]);
        assert_eq!(chunks[1].heading_path, vec!["T".to_string()]);
    }

    #[test]
    fn tc_235_tc_236_tc_260_tc_261_character_references() {
        assert_eq!(
            texts(&chunk_html("<p>&amp; &#65; &#x42; &unknown;</p>")),
            vec!["& A B &unknown;"]
        );
        assert_eq!(texts(&chunk_html("<!-- &amp; -->")), vec![" &amp; "]);
        assert_eq!(texts(&chunk_html("<p>&#65 &#x42</p>")), vec!["A B"]);
        assert_eq!(
            texts(&chunk_html("<p>&#1114111;</p>")),
            vec!["\u{10FFFF}"]
        );
        assert_eq!(texts(&chunk_html("<p>&notin x</p>")), vec!["\u{ac}in x"]);
        assert_eq!(texts(&chunk_html("<p>&amp x</p>")), vec!["& x"]);
        assert_eq!(texts(&chunk_html("<p>&notit;</p>")), vec!["\u{ac}it;"]);
        assert_eq!(texts(&chunk_html("<p>&notin;</p>")), vec!["\u{2209}"]);
        assert_eq!(texts(&chunk_html("<!-- &#65 &amp -->")), vec![" &#65 &amp "]);
        let disguised = chunk_html("<p>&lt;!-- not --&gt;</p>");
        assert_eq!(texts(&disguised), vec!["<!-- not -->"]);
        assert_eq!(disguised.len(), 1);
    }

    #[test]
    fn tc_248_numeric_references_follow_html_replacements() {
        assert_eq!(
            texts(&chunk_html("<p>&#128; &#150; &#0; &#xD800; &#1114112;</p>")),
            vec!["\u{20AC} \u{2013} \u{FFFD} \u{FFFD} \u{FFFD}"]
        );
        assert_eq!(texts(&chunk_html("<!-- &#128; -->")), vec![" &#128; "]);
    }

    #[test]
    fn tc_87_tc_140_tc_200_tc_222_length_limit() {
        let title: String = std::iter::repeat('h').take(CHUNK_LIMIT + 1).collect();
        let heading = chunk_html(&format!("<h1>{title}</h1><p>Body</p>"));
        assert_eq!(heading.len(), 2);
        assert_eq!(heading[0].text, title);
        assert_eq!(heading[1].text, "Body");
        assert_eq!(heading[1].heading_path, vec![title]);

        let body: String = std::iter::repeat('a').take(CHUNK_LIMIT + 1).collect();
        let split = chunk_html(&format!("<h1>T</h1><p>{body}</p>"));
        assert_eq!(split.len(), 3);
        assert_eq!(split[0].text, "T");
        assert_eq!(split[1].text.chars().count(), CHUNK_LIMIT);
        assert_eq!(split[2].text.chars().count(), 1 + OVERLAP);
        assert!(split[1].text.chars().all(|ch| ch == 'a'));
        assert!(split[2].text.chars().all(|ch| ch == 'a'));
        assert_eq!(split[1].start, 13);
        assert_eq!(split[1].end, 13 + CHUNK_LIMIT);
        assert_eq!(split[2].start, 13 + CHUNK_LIMIT - OVERLAP);
        assert_eq!(split[2].end, 13 + CHUNK_LIMIT + 1);
        assert_eq!(split[1].end - split[2].start, OVERLAP);
        assert_eq!(split[1].heading_path, vec!["T".to_string()]);
        assert_eq!(split[2].heading_path, vec!["T".to_string()]);
        let shared: String = split[1].text.chars().skip(CHUNK_LIMIT - OVERLAP).collect();
        assert!(split[2].text.starts_with(&shared));

        let note: String = std::iter::repeat('c').take(CHUNK_LIMIT + 1).collect();
        let comment = chunk_html(&format!("<h1>T</h1><!--{note}-->"));
        assert_eq!(comment.len(), 2);
        assert_eq!(comment[1].text, note);
        assert_eq!(comment[1].heading_path, vec!["T".to_string()]);
        assert_eq!(comment[1].text.chars().count(), CHUNK_LIMIT + 1);
    }
}
