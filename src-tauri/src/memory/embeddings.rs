//! Real embedding pipeline: OpenAI text-embedding-3-small with Ollama fallback.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub id: String,
    pub content: String,
    pub source: String,
    pub source_id: Option<String>,
    pub dimensions: usize,
    pub model: String,
    pub created_at: String,
}

/// Get embedding from OpenAI text-embedding-3-small
pub async fn get_openai_embedding(text: &str, api_key: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "text-embedding-3-small",
            "input": text
        }))
        .send()
        .await
        .map_err(|e| format!("OpenAI embedding request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI embedding error {}: {}", status, body));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse embedding response: {}", e))?;

    let embedding = json["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding in response")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Vec<f32>>();

    if embedding.is_empty() {
        return Err("Empty embedding returned".into());
    }

    Ok(embedding)
}

/// Get embedding from local Ollama
pub async fn get_ollama_embedding(text: &str, ollama_url: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/embeddings", ollama_url))
        .json(&serde_json::json!({
            "model": "nomic-embed-text",
            "prompt": text
        }))
        .send()
        .await
        .map_err(|e| format!("Ollama embedding failed: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let embedding = json["embedding"]
        .as_array()
        .ok_or("No embedding in Ollama response")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Vec<f32>>();

    Ok(embedding)
}

/// Get embedding with fallback: OpenAI → Ollama → error
pub async fn get_embedding(
    text: &str,
    openai_key: Option<&str>,
    ollama_url: Option<&str>,
) -> Result<(Vec<f32>, String), String> {
    // Try OpenAI first
    if let Some(key) = openai_key {
        if !key.is_empty() {
            match get_openai_embedding(text, key).await {
                Ok(emb) => return Ok((emb, "text-embedding-3-small".to_string())),
                Err(e) => tracing::warn!("OpenAI embedding failed, trying fallback: {}", e),
            }
        }
    }

    // Try Ollama
    if let Some(url) = ollama_url {
        if !url.is_empty() {
            match get_ollama_embedding(text, url).await {
                Ok(emb) => return Ok((emb, "nomic-embed-text".to_string())),
                Err(e) => tracing::warn!("Ollama embedding failed: {}", e),
            }
        }
    }

    Err("No embedding provider available (need OpenAI API key or Ollama)".into())
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Serialize f32 vector to bytes for SQLite BLOB storage
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize bytes from SQLite BLOB to f32 vector
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Store an embedding in the database
pub fn store_embedding(
    db: &rusqlite::Connection,
    content: &str,
    source: &str,
    source_id: Option<&str>,
    embedding: &[f32],
    model: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let blob = embedding_to_bytes(embedding);
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(
        "INSERT OR REPLACE INTO embeddings (id, content, source, source_id, embedding, dimensions, model, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, content, source, source_id, blob, embedding.len(), model, now],
    )
    .map_err(|e| format!("Failed to store embedding: {}", e))?;

    Ok(id)
}

/// Search for similar content using cosine similarity
pub fn semantic_search(
    db: &rusqlite::Connection,
    query_embedding: &[f32],
    source_filter: Option<&str>,
    top_k: usize,
) -> Result<Vec<(String, String, f32)>, String> {
    let query = if let Some(source) = source_filter {
        format!(
            "SELECT id, content, embedding FROM embeddings WHERE source = '{}'",
            source
        )
    } else {
        "SELECT id, content, embedding FROM embeddings".to_string()
    };

    let conn = db;
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let blob: Vec<u8> = row.get(2)?;
            Ok((id, content, blob))
        })
        .map_err(|e| e.to_string())?;

    let mut scored: Vec<(String, String, f32)> = Vec::new();
    for row in rows {
        if let Ok((id, content, blob)) = row {
            let emb = bytes_to_embedding(&blob);
            let score = cosine_similarity(query_embedding, &emb);
            scored.push((id, content, score));
        }
    }

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&a, &a);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b);
        assert!(score.abs() < 0.001);
    }

    #[test]
    fn test_embedding_serialization() {
        let original = vec![1.5, -2.3, 0.0, 42.0];
        let bytes = embedding_to_bytes(&original);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }
}
