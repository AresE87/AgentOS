use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// An entity (node) in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub properties: HashMap<String, String>,
}

/// A directed relationship (edge) between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub from_entity: String,
    pub to_entity: String,
    pub relation_type: String,
    pub weight: f64,
}

/// Graph stats summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub entity_count: u64,
    pub relationship_count: u64,
    pub entity_types: Vec<String>,
    pub relation_types: Vec<String>,
}

/// SQLite-backed knowledge graph
pub struct KnowledgeGraph {
    conn: Connection,
}

impl KnowledgeGraph {
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        let kg = Self { conn };
        kg.ensure_tables()?;
        Ok(kg)
    }

    /// Create the necessary tables if they don't exist
    fn ensure_tables(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS kg_entities (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    entity_type TEXT NOT NULL,
                    properties TEXT NOT NULL DEFAULT '{}'
                );
                CREATE TABLE IF NOT EXISTS kg_relationships (
                    id TEXT PRIMARY KEY,
                    from_entity TEXT NOT NULL,
                    to_entity TEXT NOT NULL,
                    relation_type TEXT NOT NULL,
                    weight REAL NOT NULL DEFAULT 1.0,
                    FOREIGN KEY (from_entity) REFERENCES kg_entities(id),
                    FOREIGN KEY (to_entity) REFERENCES kg_entities(id)
                );
                CREATE INDEX IF NOT EXISTS idx_kg_rel_from ON kg_relationships(from_entity);
                CREATE INDEX IF NOT EXISTS idx_kg_rel_to ON kg_relationships(to_entity);
                CREATE INDEX IF NOT EXISTS idx_kg_entity_type ON kg_entities(entity_type);
                CREATE INDEX IF NOT EXISTS idx_kg_entity_name ON kg_entities(name);
                ",
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Add or upsert an entity
    pub fn add_entity(&self, entity: &Entity) -> Result<(), String> {
        let props_json = serde_json::to_string(&entity.properties).unwrap_or_default();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO kg_entities (id, name, entity_type, properties) VALUES (?1, ?2, ?3, ?4)",
                params![entity.id, entity.name, entity.entity_type, props_json],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Add a relationship between two entities
    pub fn add_relationship(&self, rel: &Relationship) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO kg_relationships (id, from_entity, to_entity, relation_type, weight) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![rel.id, rel.from_entity, rel.to_entity, rel.relation_type, rel.weight],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get an entity by id
    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, entity_type, properties FROM kg_entities WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let props_str: String = row.get(3)?;
                let properties: HashMap<String, String> =
                    serde_json::from_str(&props_str).unwrap_or_default();
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    properties,
                })
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(Ok(entity)) => Ok(Some(entity)),
            Some(Err(e)) => Err(e.to_string()),
            None => Ok(None),
        }
    }

    /// Find all relationships involving an entity (as source or target)
    pub fn find_relationships(&self, entity_id: &str) -> Result<Vec<Relationship>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, from_entity, to_entity, relation_type, weight FROM kg_relationships WHERE from_entity = ?1 OR to_entity = ?1",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![entity_id], |row| {
                Ok(Relationship {
                    id: row.get(0)?,
                    from_entity: row.get(1)?,
                    to_entity: row.get(2)?,
                    relation_type: row.get(3)?,
                    weight: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    }

    /// Search entities by name substring
    pub fn search_entities(&self, query: &str) -> Result<Vec<Entity>, String> {
        self.search_entities_ranked(query, 50)
    }

    /// Search entities by name and type with relevance ranking.
    /// Prefix matches are ranked higher than substring matches.
    pub fn search_entities_ranked(&self, query: &str, limit: usize) -> Result<Vec<Entity>, String> {
        let pattern = format!("%{}%", query);
        let prefix = format!("{}%", query);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, entity_type, properties FROM kg_entities
                 WHERE name LIKE ?1 OR entity_type LIKE ?1
                 ORDER BY
                    CASE WHEN name LIKE ?2 THEN 0 ELSE 1 END,
                    CASE WHEN entity_type LIKE ?2 THEN 0 ELSE 1 END,
                    name
                 LIMIT ?3"
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![pattern, prefix, limit as i64], |row| {
                let props_str: String = row.get(3)?;
                let properties: HashMap<String, String> =
                    serde_json::from_str(&props_str).unwrap_or_default();
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    properties,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    }

    /// Get overall graph statistics
    pub fn get_graph_stats(&self) -> Result<GraphStats, String> {
        let entity_count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM kg_entities", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let relationship_count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM kg_relationships", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let mut entity_types = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT entity_type FROM kg_entities")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for row in rows {
                entity_types.push(row.map_err(|e| e.to_string())?);
            }
        }

        let mut relation_types = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT relation_type FROM kg_relationships")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for row in rows {
                relation_types.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(GraphStats {
            entity_count,
            relationship_count,
            entity_types,
            relation_types,
        })
    }
}
