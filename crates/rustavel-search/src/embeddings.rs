//! Embedding generation: `Str::to_embeddings` and the vector store types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Alias for results produced by embedding generation.
pub type Result<T> = std::result::Result<T, EmbeddingError>;

/// Errors produced while turning text into embeddings.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    /// The embedding provider rejected the request.
    #[error("embedding provider error: {0}")]
    Provider(String),
    /// The text could not be embedded (empty after normalization).
    #[error("cannot embed empty text")]
    EmptyText,
    /// The provider returned a dimension inconsistent with the model.
    #[error("embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected embedding dimension.
        expected: usize,
        /// Actual embedding dimension.
        actual: usize,
    },
}

/// One normalized embedding vector (`Vec<f32>` alias).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    /// Embedding dimension.
    pub fn dim(&self) -> usize {
        self.0.len()
    }
}

/// Embeddings payload returned by providers for one or more inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorEmbeddings {
    /// One embedding per input string, in order.
    pub data: Vec<Embedding>,
    /// Provider/model that produced the embeddings.
    pub model: String,
}

/// `to_embeddings` extension trait over string-like types (Laravel `Str`
/// parity). Implemented for `str` and `String` via `AsRef<str>`.
///
/// Real provider dispatch is delegated by the caller-provided closure so this
/// crate stays engine-agnostic; the default implementation is a deterministic
/// hashing stub that always yields `dim`-length unit vectors.
pub trait Str {
    /// Embed `self` with `dim` dimensions via `embed`.
    ///
    /// `embed` maps input text to a provider embedding; when `None` is
    /// returned the deterministic stub embedding is used instead.
    fn to_embeddings<F>(&self, dim: usize, embed: F) -> Result<VectorEmbeddings>
    where
        F: Fn(&str) -> Option<Vec<f32>>,
    {
        let input: &str = self.as_str();
        let vector = match embed(input) {
            Some(v) => v,
            None => stub_embedding(input, dim),
        };
        if vector.len() != dim {
            return Err(EmbeddingError::DimensionMismatch {
                expected: dim,
                actual: vector.len(),
            });
        }
        Ok(VectorEmbeddings {
            data: vec![Embedding(vector)],
            model: "stub".to_string(),
        })
    }

    /// Borrow `self` as a string slice.
    fn as_str(&self) -> &str;
}

impl Str for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl Str for String {
    fn as_str(&self) -> &str {
        self
    }
}

/// Deterministic unit-norm stub embedding derived from the input hash.
pub fn stub_embedding(input: &str, dim: usize) -> Vec<f32> {
    if dim == 0 {
        return Vec::new();
    }
    let hash = fnv1a(input.as_bytes());
    let mut seed = hash as f32 / u32::MAX as f32 * std::f32::consts::TAU;
    let mut out = Vec::with_capacity(dim);
    let mut carry = 0.0_f32;
    for _ in 0..dim {
        seed = fract_sin(seed * 12.9898 + 78.233);
        let v = seed * 2.0 - 1.0;
        carry += v * v;
        out.push(v);
    }
    let norm = carry.sqrt().max(f32::EPSILON);
    for v in &mut out {
        *v /= norm;
    }
    out
}

/// FNV-1a 32-bit hash of the input bytes.
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for b in bytes {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Fractal pseudo-random in (0, 1) from `x`.
fn fract_sin(x: f32) -> f32 {
    (x.sin() * 43_758.5453).fract().abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_embedding_is_unit_norm() {
        let v = stub_embedding("hello", 8);
        assert_eq!(v.len(), 8);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3);
    }

    #[test]
    fn to_embeddings_uses_provider_when_present() {
        let result = "doc"
            .to_embeddings(3, |_| Some(vec![1.0, 0.0, 0.0]))
            .unwrap();
        assert_eq!(result.data[0].0, vec![1.0, 0.0, 0.0]);
        assert_eq!(result.model, "stub");
    }
}
