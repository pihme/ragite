//! Chunking, embedding, and fusion for ragite. This crate does not link SQLite.

mod chunk;
mod model;

pub use chunk::{Chunk, chunk_plain};
pub use model::{BuiltinModel, Role, builtin_model};

/// Unicode scalars copied onto the next piece when a unit is split.
pub const OVERLAP: usize = 64;

/// Maximum Unicode scalars in a piece that the length limit is allowed to split.
pub const CHUNK_LIMIT: usize = 512;

/// Chunks taken from each ranking before fusion.
pub const FUSION_DEPTH: usize = 50;

pub const MODEL_NAME: &str = "potion-base-8M";

pub const TECHNICAL_ID: &str =
    "potion-base-8M@bf8b056651a2c21b8d2565580b8569da283cab23";

pub const DIMENSION: usize = 256;

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
