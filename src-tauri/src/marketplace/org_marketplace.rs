use crate::branding::BrandingConfig;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgListing {
    pub id: String,
    pub org_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub visibility: String,
    pub approved: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMarketplaceView {
    pub org_id: String,
    pub branding: BrandingConfig,
    pub listings: Vec<OrgListing>,
}

pub struct OrgMarketplace {
    db_path: PathBuf,
}

impl OrgMarketplace {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let marketplace = Self { db_path };
        let conn = marketplace.open()?;
        Self::init_db(&conn)?;
        Ok(marketplace)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open org marketplace DB: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to configure org marketplace DB: {}", e))?;
        Self::init_db(&conn)?;
        Ok(conn)
    }

    fn init_db(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS org_marketplace_listings (
                id TEXT PRIMARY KEY,
                org_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                visibility TEXT NOT NULL,
                approved INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS org_branding (
                org_id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_org_marketplace_org ON org_marketplace_listings(org_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_org_marketplace_visibility ON org_marketplace_listings(visibility, approved, created_at DESC);",
        )
        .map_err(|e| format!("Failed to initialize org marketplace tables: {}", e))
    }

    pub fn publish(&self, mut listing: OrgListing) -> Result<OrgListing, String> {
        let conn = self.open()?;
        listing.id = uuid::Uuid::new_v4().to_string();
        listing.created_at = chrono::Utc::now().to_rfc3339();
        listing.approved = false;
        conn.execute(
            "INSERT INTO org_marketplace_listings
             (id, org_id, resource_type, resource_id, visibility, approved, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                listing.id,
                listing.org_id,
                listing.resource_type,
                listing.resource_id,
                listing.visibility,
                listing.approved as i64,
                listing.created_at,
            ],
        )
        .map_err(|e| format!("Failed to publish org marketplace listing: {}", e))?;
        self.get_listing(&listing.id)?
            .ok_or_else(|| format!("Failed to reload org listing {}", listing.id))
    }

    pub fn list_for_org(&self, org_id: &str) -> Result<Vec<OrgListing>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, org_id, resource_type, resource_id, visibility, approved, created_at
                 FROM org_marketplace_listings
                 WHERE (org_id = ?1 OR visibility = 'public') AND approved = 1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare marketplace listing query: {}", e))?;
        let rows = stmt
            .query_map(params![org_id], map_listing)
            .map_err(|e| format!("Failed to query org marketplace listings: {}", e))?;
        Ok(rows.flatten().collect())
    }

    pub fn approve(&self, listing_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "UPDATE org_marketplace_listings SET approved = 1 WHERE id = ?1",
                params![listing_id],
            )
            .map_err(|e| format!("Failed to approve org marketplace listing: {}", e))?;
        if changed == 0 {
            return Err(format!("Listing not found: {}", listing_id));
        }
        Ok(())
    }

    pub fn approve_for_org(&self, listing_id: &str, org_id: &str) -> Result<(), String> {
        let listing = self
            .get_listing(listing_id)?
            .ok_or_else(|| format!("Listing not found: {}", listing_id))?;
        if listing.org_id != org_id {
            return Err(format!(
                "Tenant '{}' cannot approve listing owned by '{}'",
                org_id, listing.org_id
            ));
        }
        self.approve(listing_id)
    }

    pub fn remove(&self, listing_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "DELETE FROM org_marketplace_listings WHERE id = ?1",
                params![listing_id],
            )
            .map_err(|e| format!("Failed to remove org marketplace listing: {}", e))?;
        if changed == 0 {
            return Err(format!("Listing not found: {}", listing_id));
        }
        Ok(())
    }

    pub fn remove_for_org(&self, listing_id: &str, org_id: &str) -> Result<(), String> {
        let listing = self
            .get_listing(listing_id)?
            .ok_or_else(|| format!("Listing not found: {}", listing_id))?;
        if listing.org_id != org_id {
            return Err(format!(
                "Tenant '{}' cannot remove listing owned by '{}'",
                org_id, listing.org_id
            ));
        }
        self.remove(listing_id)
    }

    pub fn search(&self, query: &str, org_id: &str) -> Result<Vec<OrgListing>, String> {
        let q = format!("%{}%", query.to_lowercase());
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, org_id, resource_type, resource_id, visibility, approved, created_at
                 FROM org_marketplace_listings
                 WHERE (org_id = ?1 OR visibility = 'public')
                   AND approved = 1
                   AND (LOWER(resource_type) LIKE ?2 OR LOWER(resource_id) LIKE ?2)
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare marketplace search query: {}", e))?;
        let rows = stmt
            .query_map(params![org_id, q], map_listing)
            .map_err(|e| format!("Failed to search org marketplace listings: {}", e))?;
        Ok(rows.flatten().collect())
    }

    pub fn get_branding(&self, org_id: &str) -> Result<Option<BrandingConfig>, String> {
        let conn = self.open()?;
        let raw = conn
            .query_row(
                "SELECT config_json FROM org_branding WHERE org_id = ?1",
                params![org_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load org branding: {}", e))?;
        raw.map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .transpose()
    }

    pub fn set_branding(
        &self,
        org_id: &str,
        config: &BrandingConfig,
    ) -> Result<BrandingConfig, String> {
        let conn = self.open()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO org_branding (org_id, config_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(org_id) DO UPDATE SET config_json = excluded.config_json, updated_at = excluded.updated_at",
            params![
                org_id,
                serde_json::to_string_pretty(config).map_err(|e| e.to_string())?,
                now,
            ],
        )
        .map_err(|e| format!("Failed to persist org branding: {}", e))?;
        self.get_branding(org_id)?
            .ok_or_else(|| format!("Failed to reload org branding for {}", org_id))
    }

    pub fn reset_branding(&self, org_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        conn.execute(
            "DELETE FROM org_branding WHERE org_id = ?1",
            params![org_id],
        )
        .map_err(|e| format!("Failed to reset org branding: {}", e))?;
        Ok(())
    }

    pub fn get_view_for_org(
        &self,
        org_id: &str,
        fallback_branding: &BrandingConfig,
    ) -> Result<OrgMarketplaceView, String> {
        let branding = self
            .get_branding(org_id)?
            .unwrap_or_else(|| fallback_branding.clone());
        let listings = self.list_for_org(org_id)?;
        Ok(OrgMarketplaceView {
            org_id: org_id.to_string(),
            branding,
            listings,
        })
    }

    fn get_listing(&self, listing_id: &str) -> Result<Option<OrgListing>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, org_id, resource_type, resource_id, visibility, approved, created_at
             FROM org_marketplace_listings WHERE id = ?1",
            params![listing_id],
            map_listing,
        )
        .optional()
        .map_err(|e| format!("Failed to load org listing: {}", e))
    }
}

fn map_listing(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrgListing> {
    Ok(OrgListing {
        id: row.get(0)?,
        org_id: row.get(1)?,
        resource_type: row.get(2)?,
        resource_id: row.get(3)?,
        visibility: row.get(4)?,
        approved: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn demo_brand(name: &str, color: &str) -> BrandingConfig {
        BrandingConfig {
            app_name: name.to_string(),
            tagline: format!("{} workspace", name),
            primary_color: color.to_string(),
            accent_color: color.to_string(),
            oem_license: Some(crate::branding::config::OEMLicense {
                tier: "partner".to_string(),
                company: name.to_string(),
                issued_at: chrono::Utc::now().to_rfc3339(),
                expires_at: None,
            }),
            ..BrandingConfig::default()
        }
    }

    #[test]
    fn tenants_get_distinct_branding_and_catalogs() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("org-marketplace.db");
        let market = OrgMarketplace::new(db_path).unwrap();

        market
            .set_branding("acme", &demo_brand("Acme AgentOS", "#ff5500"))
            .unwrap();
        market
            .set_branding("northwind", &demo_brand("Northwind Desk", "#0088ff"))
            .unwrap();

        let acme_private = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "acme".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "acme-finance-close".to_string(),
                visibility: "org_only".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();
        let shared = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "acme".to_string(),
                resource_type: "template".to_string(),
                resource_id: "shared-customer-intake".to_string(),
                visibility: "public".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();
        let northwind_private = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "northwind".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "northwind-warehouse-sync".to_string(),
                visibility: "org_only".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();

        market.approve(&acme_private.id).unwrap();
        market.approve(&shared.id).unwrap();
        market.approve(&northwind_private.id).unwrap();

        let acme = market
            .get_view_for_org("acme", &BrandingConfig::default())
            .unwrap();
        let northwind = market
            .get_view_for_org("northwind", &BrandingConfig::default())
            .unwrap();

        assert_eq!(acme.branding.app_name, "Acme AgentOS");
        assert_eq!(northwind.branding.app_name, "Northwind Desk");
        assert!(acme
            .listings
            .iter()
            .any(|item| item.resource_id == "acme-finance-close"));
        assert!(acme
            .listings
            .iter()
            .any(|item| item.resource_id == "shared-customer-intake"));
        assert!(!acme
            .listings
            .iter()
            .any(|item| item.resource_id == "northwind-warehouse-sync"));
        assert!(northwind
            .listings
            .iter()
            .any(|item| item.resource_id == "northwind-warehouse-sync"));
    }

    #[test]
    fn search_is_filtered_to_visible_catalog() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("org-marketplace-search.db");
        let market = OrgMarketplace::new(db_path).unwrap();

        let visible = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "acme".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "invoice-reconciliation".to_string(),
                visibility: "public".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();
        let hidden = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "northwind".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "invoice-reconciliation-internal".to_string(),
                visibility: "org_only".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();
        market.approve(&visible.id).unwrap();
        market.approve(&hidden.id).unwrap();

        let acme_results = market.search("invoice", "acme").unwrap();
        assert_eq!(acme_results.len(), 1);
        assert_eq!(acme_results[0].resource_id, "invoice-reconciliation");
    }

    #[test]
    fn tenant_cannot_remove_or_approve_other_org_listing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("org-marketplace-guard.db");
        let market = OrgMarketplace::new(db_path).unwrap();

        let listing = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "northwind".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "northwind-internal".to_string(),
                visibility: "org_only".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();

        let approve_error = market.approve_for_org(&listing.id, "acme").unwrap_err();
        let remove_error = market.remove_for_org(&listing.id, "acme").unwrap_err();

        assert!(approve_error.contains("cannot approve"));
        assert!(remove_error.contains("cannot remove"));
    }
}
