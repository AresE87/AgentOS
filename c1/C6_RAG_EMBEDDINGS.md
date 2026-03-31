# CONSOLIDACIÓN C6 — RAG REAL CON EMBEDDINGS

**Estado actual:** ⚠️ Agent memory usa `WHERE content LIKE '%query%'`. NO hay embeddings. NO hay similarity search real.
**Objetivo:** Embeddings reales (OpenAI API o local) + cosine similarity. El agente RECUERDA contexto relevante aunque el usuario use palabras diferentes.

---

## Qué YA existe

```
src-tauri/src/memory/
- MemoryStore con remember(), recall(), forget()
- recall() usa: SELECT * FROM memories WHERE content LIKE '%query%'
- SQLite table: memories (id, content, category, timestamp)
```

## Qué REEMPLAZAR

### 1. Agregar columna embedding a SQLite

```sql
-- Migration:
ALTER TABLE memories ADD COLUMN embedding BLOB;
-- embedding es un Vec<f32> serializado (1536 floats para OpenAI, 384 para MiniLM)
```

### 2. Embedding provider

```rust
pub struct EmbeddingProvider {
    mode: EmbeddingMode,
}

pub enum EmbeddingMode {
    OpenAI { api_key: String },      // $0.0001/1K tokens — preciso
    Local { /* futuro: ONNX MiniLM */ },  // Gratis pero necesita modelo descargado
}

impl EmbeddingProvider {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match &self.mode {
            EmbeddingMode::OpenAI { api_key } => {
                // POST https://api.openai.com/v1/embeddings
                // model: "text-embedding-3-small" (1536 dims, barato)
                let resp = reqwest::Client::new()
                    .post("https://api.openai.com/v1/embeddings")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&json!({"model": "text-embedding-3-small", "input": text}))
                    .send().await?;
                let data: Value = resp.json().await?;
                let embedding: Vec<f32> = serde_json::from_value(data["data"][0]["embedding"].clone())?;
                Ok(embedding)
            }
            EmbeddingMode::Local { .. } => {
                // Futuro: ONNX Runtime con MiniLM
                Err("Local embeddings not yet available".into())
            }
        }
    }
}
```

### 3. Cosine similarity search

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot / (norm_a * norm_b)
}

impl MemoryStore {
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        // ANTES: SELECT WHERE content LIKE '%query%'
        // AHORA:
        let query_embedding = self.embedder.embed(query).await?;
        
        // Cargar todos los embeddings de SQLite
        let all_memories = self.db.query("SELECT id, content, embedding FROM memories WHERE embedding IS NOT NULL")?;
        
        // Calcular similarity y rankear
        let mut scored: Vec<(f32, Memory)> = all_memories.iter()
            .map(|m| {
                let emb: Vec<f32> = bincode::deserialize(&m.embedding).unwrap();
                let score = cosine_similarity(&query_embedding, &emb);
                (score, m.clone())
            })
            .collect();
        
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        Ok(scored.into_iter().take(limit).map(|(_, m)| m).collect())
    }
    
    pub async fn remember(&self, content: &str, category: &str) -> Result<()> {
        // ANTES: solo guardaba texto
        // AHORA: también genera y guarda embedding
        let embedding = self.embedder.embed(content).await?;
        let emb_bytes = bincode::serialize(&embedding)?;
        
        self.db.execute(
            "INSERT INTO memories (id, content, category, embedding, timestamp) VALUES (?, ?, ?, ?, ?)",
            params![uuid(), content, category, emb_bytes, now()],
        )?;
        Ok(())
    }
}
```

### 4. Migrar memories existentes

```rust
// Al iniciar, si hay memories sin embedding → generar en background:
pub async fn backfill_embeddings(&self) -> Result<usize> {
    let missing = self.db.query("SELECT id, content FROM memories WHERE embedding IS NULL")?;
    for memory in &missing {
        let embedding = self.embedder.embed(&memory.content).await?;
        self.db.execute("UPDATE memories SET embedding = ? WHERE id = ?",
            params![bincode::serialize(&embedding)?, memory.id])?;
    }
    Ok(missing.len())
}
```

---

## Verificación

1. ✅ "Mi jefe se llama Juan" → embedding guardado
2. ✅ "¿Quién es mi manager?" → similarity search encuentra "jefe = Juan" (aunque dijo "manager" no "jefe")
3. ✅ LIKE search NO encontraría esto (palabras diferentes). Embeddings SÍ.
4. ✅ recall() retorna memories rankeadas por relevancia, no por keyword match
5. ✅ 1000 memories → search en < 200ms (embeddings son vectores, cosine es rápido)
