//! BM25-lite lexical ranking for deferred tool search.
//!
//! A dependency-free scoring kernel: tokenize the query and each candidate's
//! `name` + `description`, then score with the Okapi BM25 formula
//! (`k1 = 1.2`, `b = 0.75`). Callers sort the returned scores descending and
//! drop zero-score candidates. Kept private to the crate so `loaders.rs` stays
//! within the file-size budget.

/// BM25 term-frequency saturation parameter.
const K1: f32 = 1.2;
/// BM25 length-normalisation parameter.
const B: f32 = 0.75;

/// Lowercase alphanumeric tokens from `text` (splits on `_`, `-`, punctuation).
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

/// Score each document against `query` with Okapi BM25.
///
/// `docs` are `(name, description)` pairs; the scored text is their
/// concatenation. Returns one score per document, aligned by index; a document
/// sharing no query term scores `0.0`.
pub(crate) fn score(query: &str, docs: &[(&str, &str)]) -> Vec<f32> {
    let query_terms = dedup(tokenize(query));
    if query_terms.is_empty() || docs.is_empty() {
        return vec![0.0; docs.len()];
    }

    let tokenized: Vec<Vec<String>> = docs
        .iter()
        .map(|(name, description)| tokenize(&format!("{name} {description}")))
        .collect();
    let total_len: usize = tokenized.iter().map(Vec::len).sum();
    let avg_len = if tokenized.is_empty() {
        0.0
    } else {
        total_len as f32 / tokenized.len() as f32
    };
    let doc_count = docs.len() as f32;

    query_terms
        .iter()
        .map(|term| {
            let doc_freq = tokenized
                .iter()
                .filter(|tokens| tokens.iter().any(|token| token == term))
                .count() as f32;
            let idf = (1.0 + (doc_count - doc_freq + 0.5) / (doc_freq + 0.5)).ln();
            tokenized
                .iter()
                .map(|tokens| {
                    let term_freq = tokens.iter().filter(|token| *token == term).count() as f32;
                    if term_freq == 0.0 {
                        0.0
                    } else {
                        let len_norm = if avg_len == 0.0 {
                            0.0
                        } else {
                            tokens.len() as f32 / avg_len
                        };
                        let denominator = term_freq + K1 * (1.0 - B + B * len_norm);
                        idf * (term_freq * (K1 + 1.0)) / denominator
                    }
                })
                .collect::<Vec<f32>>()
        })
        .fold(vec![0.0_f32; docs.len()], |mut acc, per_doc| {
            for (slot, value) in acc.iter_mut().zip(per_doc) {
                *slot += value;
            }
            acc
        })
}

/// Preserve first-seen order while removing duplicate tokens.
fn dedup(mut tokens: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    tokens.retain(|token| seen.insert(token.clone()));
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevant_document_outranks_irrelevant() {
        let docs = [
            ("search_docs", "Search the documentation for a phrase"),
            ("send_email", "Send an email to a recipient"),
        ];
        let scores = score("search docs", &docs);
        assert!(scores[0] > scores[1]);
        assert_eq!(scores[1], 0.0);
    }

    #[test]
    fn description_terms_contribute_to_ranking() {
        let docs = [
            ("alpha", "unrelated"),
            ("beta", "retrieve documentation snippets"),
        ];
        let scores = score("documentation", &docs);
        assert_eq!(scores[0], 0.0);
        assert!(scores[1] > 0.0);
    }

    #[test]
    fn empty_query_scores_zero() {
        let docs = [("search", "search things")];
        assert_eq!(score("   ", &docs), vec![0.0]);
    }

    #[test]
    fn rarer_term_has_higher_idf() {
        let docs = [
            ("common", "alpha beta"),
            ("rare", "alpha gamma"),
            ("other", "alpha delta"),
        ];
        let scores = score("gamma", &docs);
        // Only the document containing the rare term scores.
        assert_eq!(scores[0], 0.0);
        assert!(scores[1] > 0.0);
        assert_eq!(scores[2], 0.0);
    }
}
