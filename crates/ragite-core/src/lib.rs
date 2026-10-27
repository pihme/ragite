//! Chunking, embedding, and fusion for ragite. This crate does not link SQLite.

mod chunk;
mod docling;
mod fusion;
mod html;
mod markdown;
mod model;

#[cfg(any(test, feature = "fuzz-invariants"))]
mod invariants;
#[cfg(any(test, feature = "fuzz-invariants"))]
pub use invariants::{check_docling, check_html, check_markdown, check_plain};

pub use chunk::{Chunk, chunk_html, chunk_markdown, chunk_plain};
pub use docling::chunk_docling;
pub use fusion::{FusedChunk, best_document_scores, order_fused_chunks, reciprocal_rank_fusion};
pub use model::{BuiltinModel, Role, builtin_model};

/// Unicode scalars copied onto the next piece when a unit is split.
pub const OVERLAP: usize = 64;

/// Maximum Unicode scalars in a piece that the length limit is allowed to split.
pub const CHUNK_LIMIT: usize = 512;

/// Chunks taken from each ranking before fusion.
pub const FUSION_DEPTH: usize = 50;

#[cfg(feature = "builtin-model")]
pub const MODEL_NAME: &str = "potion-retrieval-32M";

#[cfg(feature = "builtin-model")]
pub const TECHNICAL_ID: &str = "potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7";

#[cfg(ragite_provider_model)]
pub const MODEL_NAME: &str = env!("RAGITE_COMPILED_NAME");

#[cfg(ragite_provider_model)]
pub const TECHNICAL_ID: &str = concat!(
    env!("RAGITE_COMPILED_NAME"),
    "@",
    env!("RAGITE_COMPILED_REVISION")
);

#[cfg(all(not(feature = "builtin-model"), not(ragite_provider_model)))]
pub const MODEL_NAME: &str = "";

#[cfg(all(not(feature = "builtin-model"), not(ragite_provider_model)))]
pub const TECHNICAL_ID: &str = "";

/// Dimension of the built-in potion-retrieval-32M model. A provider model uses the dimension of its own weights.
pub const DIMENSION: usize = 512;

/// SHA-256 of the UTF-8 bytes of `text`, as 64 lowercase hex characters.
pub fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

/// JSON array of strings. Plain text passes an empty path.
pub fn heading_path_json(titles: &[String]) -> String {
    serde_json::to_string(titles).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_258_heading_path_json_escapes() {
        assert_eq!(heading_path_json(&[]), "[]");
        assert_eq!(
            heading_path_json(&["A".to_string(), "B".to_string()]),
            r#"["A","B"]"#
        );
        assert_eq!(
            heading_path_json(&["say \"hi\"".to_string()]),
            r#"["say \"hi\""]"#
        );
    }

    #[test]
    fn tc_90_sha256_of_hello() {
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn tc_160_plain_blank_line() {
        let chunks = chunk_plain("a\nb\n\nc");
        assert_eq!(texts(&chunks), vec!["a\nb", "c"]);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 3);
        assert_eq!(chunks[1].start, 5);
        assert_eq!(chunks[1].end, 6);
    }

    #[test]
    fn tc_161_plain_run_of_blank_lines() {
        let chunks = chunk_plain("a\n\n\nb");
        assert_eq!(texts(&chunks), vec!["a", "b"]);
    }

    #[test]
    fn tc_179_crlf_and_space_blank_lines() {
        assert_eq!(texts(&chunk_plain("a\r\n\r\nb")), vec!["a", "b"]);
        assert_eq!(texts(&chunk_plain("a\n \nb")), vec!["a", "b"]);
        assert_eq!(texts(&chunk_plain("a\n\t\nb")), vec!["a", "b"]);
    }

    #[test]
    fn tc_180_lone_cr_stays_in_the_chunk() {
        let chunks = chunk_plain("a\rb\n\nc");
        assert_eq!(chunks[0].text, "a\rb");
    }

    #[test]
    fn tc_284_long_unit_splits_on_a_word_boundary() {
        let words = std::iter::repeat("abcdefghij")
            .take(60)
            .collect::<Vec<_>>()
            .join(" ");
        let plain = chunk_plain(&words);
        assert_word_pieces(&plain);
        assert!(plain[0].text.ends_with(' '));
        assert!(plain[0].end <= CHUNK_LIMIT);
        assert!(plain[1].start < plain[0].end);
        assert!(plain[0].end - plain[1].start >= OVERLAP);
        assert!(plain[1].text.starts_with(|ch: char| !is_split_space(ch)));

        let mut token = String::from("hi ");
        token.push_str(&"x".repeat(CHUNK_LIMIT + 20));
        let hard = chunk_plain(&token);
        assert_eq!(hard[0].text, "hi ");
        assert_eq!(hard[1].start, 3);
        assert_eq!(hard[1].text.chars().count(), CHUNK_LIMIT);
        assert_eq!(hard[2].start, 3 + CHUNK_LIMIT - OVERLAP);
        assert!(hard[1].text.chars().all(|ch| ch == 'x'));

        assert_word_pieces(&chunk_markdown(&words));
        let fenced = format!("```\n{words}\n```\n");
        assert_word_pieces(&chunk_markdown(&fenced));
        let indented = words
            .split(' ')
            .map(|word| format!("    {word}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_word_pieces(&chunk_markdown(&indented));
        assert_word_pieces(&chunk_html(&format!("<p>{words}</p>")));
        let doc = serde_json::json!({
            "schema_name": "DoclingDocument",
            "texts": [{
                "self_ref": "#/texts/0",
                "label": "text",
                "text": words
            }],
            "body": {"children": [{"$ref": "#/texts/0"}]}
        });
        let doc_chunks = chunk_docling(&doc.to_string()).expect("docling");
        assert_word_pieces(&doc_chunks);

        // The only boundary in the first window sits less than `OVERLAP` before
        // the length limit, so the piece ends on it and the next piece still
        // starts exactly `OVERLAP` earlier, inside the word.
        for (sep, label) in [(' ', "space"), ('\t', "tab"), ('\n', "lf"), ('\r', "cr")] {
            let mut text = "a".repeat(CHUNK_LIMIT - OVERLAP);
            text.push(sep);
            text.push_str(&"b".repeat(CHUNK_LIMIT));
            let chunks = chunk_plain(&text);
            assert_eq!(chunks.len(), 3, "{label}");
            assert!(chunks[0].text.ends_with(sep), "{label}");
            assert_eq!(chunks[0].end, CHUNK_LIMIT - OVERLAP + 1, "{label}");
            assert_eq!(chunks[1].start, chunks[0].end - OVERLAP, "{label}");
            assert!(chunks[1].text.starts_with('a'), "{label}");
            assert!(chunks[1].text.ends_with(sep), "{label}");
            assert!(chunks[2].text.chars().all(|ch| ch == 'b'), "{label}");
            crate::invariants::check_plain(&text).unwrap_or_else(|err| panic!("{label}: {err}"));
        }
    }

    fn is_split_space(ch: char) -> bool {
        matches!(ch, ' ' | '\t' | '\n' | '\r')
    }

    fn assert_word_pieces(chunks: &[Chunk]) {
        assert!(chunks.len() >= 2, "expected a split");
        for chunk in chunks {
            assert!(chunk.text.chars().count() <= CHUNK_LIMIT);
        }
        let mut saw_overlap = false;
        for pair in chunks.windows(2) {
            if pair[1].start >= pair[0].end || pair[1].start <= pair[0].start {
                continue;
            }
            saw_overlap = true;
            assert!(pair[0].end - pair[1].start >= OVERLAP);
            assert!(pair[0].text.ends_with([' ', '\t', '\n', '\r']));
        }
        assert!(saw_overlap, "expected an overlapping next piece");
    }

    #[test]
    fn tc_200_overlap_copies_the_last_n_scalars() {
        let text: String = (0..CHUNK_LIMIT + 1).map(|_| 'x').collect();
        let chunks = chunk_plain(&text);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, CHUNK_LIMIT);
        assert_eq!(chunks[1].start, CHUNK_LIMIT - OVERLAP);
        assert_eq!(chunks[1].end, CHUNK_LIMIT + 1);
        let shared = &chunks[0].text[chunks[0].text.chars().count() - OVERLAP..];
        assert!(chunks[1].text.starts_with(shared));
    }

    #[test]
    fn tc_35_plain_path_and_meta() {
        let chunks = chunk_plain("hello");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].heading_path.is_empty());
        assert!(chunks[0].meta_json.is_none());
        assert_eq!(chunks[0].text, "hello");
        assert_eq!((chunks[0].start, chunks[0].end), (0, 5));
    }

    fn texts(chunks: &[Chunk]) -> Vec<&str> {
        chunks.iter().map(|c| c.text.as_str()).collect()
    }
}
