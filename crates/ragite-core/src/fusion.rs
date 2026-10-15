//! Reciprocal rank fusion. k is 60 and both lists have equal weight.
//! Callers do not pass k, weights, or depth.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

const RRF_K: f64 = 60.0;

/// One fused Chunk, before rows are ordered for a surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FusedChunk {
    pub chunk_id: i64,
    pub doc_id: i64,
    pub score: f64,
}

/// Fuse two best-first rankings. Position 0 is rank 1, so its term is `1/61`.
/// A chunk on both lists sums its terms. A chunk on one list still scores.
pub fn reciprocal_rank_fusion(vector: &[i64], full_text: &[i64]) -> Vec<(i64, f64)> {
    let mut scores = HashMap::new();
    add_ranks(&mut scores, vector);
    add_ranks(&mut scores, full_text);
    scores.into_iter().collect()
}

fn add_ranks(scores: &mut HashMap<i64, f64>, ids: &[i64]) {
    // A repeated id keeps the rank of its first occurrence.
    let mut seen = HashSet::new();
    for (index, id) in ids.iter().enumerate() {
        if !seen.insert(*id) {
            continue;
        }
        let rank = index as f64 + 1.0;
        *scores.entry(*id).or_insert(0.0) += 1.0 / (RRF_K + rank);
    }
}

/// Highest Score first. Equal Scores break by Document id, then Chunk id.
pub fn order_fused_chunks(chunks: &mut [FusedChunk]) {
    chunks.sort_by(|left, right| {
        cmp_score_desc(left.score, right.score)
            .then(left.doc_id.cmp(&right.doc_id))
            .then(left.chunk_id.cmp(&right.chunk_id))
    });
}

/// One Score per Document: the best Chunk Score, not a sum.
/// Highest Score first. Equal Scores break by Document id.
pub fn best_document_scores(chunks: &[FusedChunk]) -> Vec<(i64, f64)> {
    let mut best: HashMap<i64, f64> = HashMap::new();
    for chunk in chunks {
        best.entry(chunk.doc_id)
            .and_modify(|score| {
                if chunk.score > *score {
                    *score = chunk.score;
                }
            })
            .or_insert(chunk.score);
    }
    let mut rows: Vec<(i64, f64)> = best.into_iter().collect();
    rows.sort_by(|left, right| cmp_score_desc(left.1, right.1).then(left.0.cmp(&right.0)));
    rows
}

fn cmp_score_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_of(fused: &[(i64, f64)], chunk_id: i64) -> f64 {
        fused
            .iter()
            .find(|(id, _)| *id == chunk_id)
            .map(|(_, score)| *score)
            .unwrap_or_else(|| panic!("missing chunk {chunk_id} in {fused:?}"))
    }

    #[test]
    fn tc_26_tc_223_rank_terms_are_one_over_sixty_plus_rank() {
        let both = reciprocal_rank_fusion(&[10, 20], &[10, 30]);
        assert_eq!(score_of(&both, 10), 2.0 / 61.0);
        assert_eq!(score_of(&both, 20), 1.0 / 62.0);
        assert_eq!(score_of(&both, 30), 1.0 / 62.0);

        let vector_only = reciprocal_rank_fusion(&[5], &[]);
        assert_eq!(vector_only, vec![(5, 1.0 / 61.0)]);

        let full_text_only = reciprocal_rank_fusion(&[], &[8]);
        assert_eq!(full_text_only, vec![(8, 1.0 / 61.0)]);
    }

    #[test]
    fn tc_224_document_score_is_the_best_chunk_score() {
        let chunks = [
            FusedChunk {
                chunk_id: 1,
                doc_id: 4,
                score: 2.0 / 61.0,
            },
            FusedChunk {
                chunk_id: 2,
                doc_id: 4,
                score: 1.0 / 61.0,
            },
        ];
        assert_eq!(best_document_scores(&chunks), vec![(4, 2.0 / 61.0)]);
    }

    #[test]
    fn tc_111_equal_scores_break_by_doc_id_then_chunk_id() {
        let mut chunks = vec![
            FusedChunk {
                chunk_id: 9,
                doc_id: 2,
                score: 1.0 / 61.0,
            },
            FusedChunk {
                chunk_id: 4,
                doc_id: 2,
                score: 1.0 / 61.0,
            },
            FusedChunk {
                chunk_id: 8,
                doc_id: 1,
                score: 1.0 / 61.0,
            },
            FusedChunk {
                chunk_id: 3,
                doc_id: 3,
                score: 2.0 / 61.0,
            },
            FusedChunk {
                chunk_id: 5,
                doc_id: 1,
                score: 1.0 / 61.0,
            },
        ];
        order_fused_chunks(&mut chunks);
        let order: Vec<(i64, i64)> = chunks
            .iter()
            .map(|chunk| (chunk.doc_id, chunk.chunk_id))
            .collect();
        assert_eq!(order, vec![(3, 3), (1, 5), (1, 8), (2, 4), (2, 9)]);

        let docs = best_document_scores(&chunks);
        assert_eq!(
            docs,
            vec![(3, 2.0 / 61.0), (1, 1.0 / 61.0), (2, 1.0 / 61.0)]
        );
    }
}
