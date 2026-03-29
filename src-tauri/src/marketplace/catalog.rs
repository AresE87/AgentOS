use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: String,
    pub downloads: u64,
    pub rating: f32,
    pub tags: Vec<String>,
    pub preview_steps: Vec<String>,
    pub file_size_kb: u64,
}

pub struct MarketplaceCatalog {
    entries: Vec<CatalogEntry>,
}

// Embedded at compile time — no runtime file I/O needed
const CATALOG_JSON: &str = include_str!("../../marketplace/index.json");

impl MarketplaceCatalog {
    /// Load catalog from the embedded JSON string (compile-time embedded).
    pub fn load() -> Result<Self, String> {
        let entries: Vec<CatalogEntry> =
            serde_json::from_str(CATALOG_JSON).map_err(|e| format!("Failed to parse catalog: {}", e))?;
        Ok(Self { entries })
    }

    /// Fuzzy search across name, description, and tags.
    pub fn search(&self, query: &str) -> Vec<&CatalogEntry> {
        if query.trim().is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q)
                    || e.description.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Filter entries by category.
    pub fn filter_by_category(&self, category: &str) -> Vec<&CatalogEntry> {
        let cat = category.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.category.to_lowercase() == cat)
            .collect()
    }

    /// Look up an entry by its ID.
    pub fn get_by_id(&self, id: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Return all entries.
    pub fn all(&self) -> &[CatalogEntry] {
        &self.entries
    }
}
