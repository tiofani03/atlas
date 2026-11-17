use anyhow::{Context, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const DEFAULT_LOCAL_DIMS: usize = 256;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn model_name(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Fast, zero-dependency local semantic embedding using hashed word + character 3-gram features.
#[derive(Debug, Clone)]
pub struct LocalHashEmbedding {
    dims: usize,
}

impl Default for LocalHashEmbedding {
    fn default() -> Self {
        Self {
            dims: DEFAULT_LOCAL_DIMS,
        }
    }
}

impl LocalHashEmbedding {
    pub fn new(dims: usize) -> Self {
        Self {
            dims: dims.max(32),
        }
    }

    pub fn embed_sync(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dims];
        let lower = text.to_lowercase();

        for token in lower
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .filter(|t| t.len() >= 2)
        {
            // Word-level feature (higher weight)
            Self::add_hashed_feature(&mut vec, token.as_bytes(), 2.0);

            // Character 3-grams for morphological / fuzzy similarity
            let bytes = token.as_bytes();
            if bytes.len() >= 3 {
                for window in bytes.windows(3) {
                    Self::add_hashed_feature(&mut vec, window, 0.75);
                }
            }
        }

        l2_normalize(&mut vec);
        vec
    }

    fn add_hashed_feature(vec: &mut [f32], feature: &[u8], weight: f32) {
        let mut hasher = Sha256::new();
        hasher.update(feature);
        let digest = hasher.finalize();
        let idx_raw = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) as usize;
        let sign = if digest[4] & 1 == 0 { 1.0 } else { -1.0 };
        let idx = idx_raw % vec.len();
        vec[idx] += sign * weight;
    }
}

#[async_trait]
impl EmbeddingProvider for LocalHashEmbedding {
    fn model_name(&self) -> &str {
        "local-hash-v1"
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.embed_sync(text))
    }
}

/// Optional OpenAI-compatible embeddings provider.
pub struct OpenAiEmbedding {
    api_key: String,
    model: String,
    endpoint: String,
    dims: usize,
    client: reqwest::Client,
}

impl OpenAiEmbedding {
    pub fn new(api_key: String, model: Option<String>, endpoint: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
            endpoint: endpoint
                .unwrap_or_else(|| "https://api.openai.com/v1/embeddings".to_string()),
            dims: 1536,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedding {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "input": text,
            }))
            .send()
            .await
            .context("Failed to send OpenAI embedding request")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse OpenAI embedding response")?;
        if !status.is_success() {
            anyhow::bail!("OpenAI embedding error ({}): {}", status, body);
        }

        let arr = body["data"][0]["embedding"]
            .as_array()
            .context("Missing data[0].embedding in OpenAI response")?;
        let mut vec: Vec<f32> = arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();
        l2_normalize(&mut vec);
        Ok(vec)
    }
}

pub fn l2_normalize(vec: &mut [f32]) {
    let norm_sq: f32 = vec.iter().map(|v| v * v).sum();
    if norm_sq > 1e-12 {
        let inv = 1.0 / norm_sq.sqrt();
        for v in vec.iter_mut() {
            *v *= inv;
        }
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub fn encode_f32_le(vec: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len() * 4);
    for &f in vec {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

pub fn decode_f32_le(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Combine BM25 ranked IDs and Vector ranked IDs via Reciprocal Rank Fusion (k = 60).
pub fn reciprocal_rank_fusion(bm25_ids: &[String], vector_ids: &[String], limit: usize) -> Vec<String> {
    const K: f32 = 60.0;
    let mut scores: HashMap<&str, f32> = HashMap::new();

    for (rank, id) in bm25_ids.iter().enumerate() {
        *scores.entry(id.as_str()).or_insert(0.0) += 1.0 / (K + (rank as f32) + 1.0);
    }
    for (rank, id) in vector_ids.iter().enumerate() {
        *scores.entry(id.as_str()).or_insert(0.0) += 1.0 / (K + (rank as f32) + 1.0);
    }

    let mut ranked: Vec<(&str, f32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked
        .into_iter()
        .take(limit)
        .map(|(id, _)| id.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_hash_embedding_similarity() {
        let embedder = LocalHashEmbedding::default();
        let a = embedder.embed_sync("authentication jwt token validation middleware");
        let b = embedder.embed_sync("jwt authentication token middleware handler");
        let c = embedder.embed_sync("database migration postgresql schema vacuum");

        let sim_ab = cosine_similarity(&a, &b);
        let sim_ac = cosine_similarity(&a, &c);
        assert!(sim_ab > sim_ac, "Expected sim_ab ({}) > sim_ac ({})", sim_ab, sim_ac);
    }

    #[test]
    fn test_encode_decode_f32_roundtrip() {
        let orig = vec![0.25f32, -0.5, 0.125, 1.0];
        let encoded = encode_f32_le(&orig);
        let decoded = decode_f32_le(&encoded);
        assert_eq!(orig, decoded);
    }

    #[test]
    fn test_reciprocal_rank_fusion() {
        let bm25 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let vec = vec!["c".to_string(), "b".to_string(), "d".to_string()];
        let fused = reciprocal_rank_fusion(&bm25, &vec, 3);
        assert_eq!(fused.len(), 3);
        assert!(fused.contains(&"b".to_string()));
        assert!(fused.contains(&"c".to_string()));
    }
}
