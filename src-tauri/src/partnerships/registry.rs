use crate::branding::BrandingConfig;
use crate::marketplace::OrgMarketplaceView;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationLevel {
    Basic,
    Premium,
    Exclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwarePartner {
    pub id: String,
    pub company: String,
    pub slug: String,
    pub org_id: Option<String>,
    pub device_type: String,
    pub integration_level: IntegrationLevel,
    pub certified: bool,
    pub contact_email: Option<String>,
    pub units_shipped: Option<u64>,
    pub distribution_channel: Option<String>,
    pub artifact_base_url: Option<String>,
    pub updater_pubkey: Option<String>,
    pub distribution_bundle_path: Option<String>,
    pub registered_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerDistributionBundle {
    pub partner_id: String,
    pub partner_slug: String,
    pub org_id: Option<String>,
    pub app_name: String,
    pub distribution_channel: String,
    pub artifact_base_url: String,
    pub updater_pubkey_present: bool,
    pub catalog_items: usize,
    pub manifest_path: String,
    pub manifest: serde_json::Value,
}

pub struct PartnerRegistry {
    db_path: PathBuf,
    bundle_dir: PathBuf,
}

impl PartnerRegistry {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let bundle_dir = db_path
            .parent()
            .map(|path| path.join("partner-distributions"))
            .unwrap_or_else(|| PathBuf::from("partner-distributions"));
        std::fs::create_dir_all(&bundle_dir).map_err(|e| e.to_string())?;
        let registry = Self {
            db_path,
            bundle_dir,
        };
        let conn = registry.open()?;
        Self::ensure_tables(&conn)?;
        Ok(registry)
    }

    fn open(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        Self::ensure_tables(&conn)?;
        Ok(conn)
    }

    fn ensure_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hardware_partners (
                id TEXT PRIMARY KEY,
                company TEXT NOT NULL,
                slug TEXT NOT NULL UNIQUE,
                org_id TEXT,
                device_type TEXT NOT NULL,
                integration_level TEXT NOT NULL,
                certified INTEGER NOT NULL DEFAULT 0,
                contact_email TEXT,
                units_shipped INTEGER,
                distribution_channel TEXT,
                artifact_base_url TEXT,
                updater_pubkey TEXT,
                distribution_bundle_path TEXT,
                registered_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_hardware_partners_org ON hardware_partners(org_id, updated_at DESC);",
        )
        .map_err(|e| e.to_string())
    }

    pub fn list_partners(&self) -> Result<Vec<HardwarePartner>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, company, slug, org_id, device_type, integration_level, certified,
                        contact_email, units_shipped, distribution_channel, artifact_base_url,
                        updater_pubkey, distribution_bundle_path, registered_at, updated_at
                 FROM hardware_partners
                 ORDER BY updated_at DESC, company ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_partner)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn get_partner(&self, id: &str) -> Result<Option<HardwarePartner>, String> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT id, company, slug, org_id, device_type, integration_level, certified,
                    contact_email, units_shipped, distribution_channel, artifact_base_url,
                    updater_pubkey, distribution_bundle_path, registered_at, updated_at
             FROM hardware_partners
             WHERE id = ?1",
            params![id],
            map_partner,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn register_partner(
        &self,
        company: String,
        device_type: String,
        integration_level: IntegrationLevel,
        org_id: Option<String>,
    ) -> Result<HardwarePartner, String> {
        let conn = self.open()?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let slug = slugify(&company);
        conn.execute(
            "INSERT INTO hardware_partners
             (id, company, slug, org_id, device_type, integration_level, certified, registered_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
            params![
                id,
                company,
                slug,
                org_id,
                device_type,
                integration_level.as_str(),
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
        self.get_partner(&id)?
            .ok_or_else(|| format!("Partner '{}' could not be reloaded", id))
    }

    pub fn certify(&self, id: &str) -> Result<HardwarePartner, String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "UPDATE hardware_partners
                 SET certified = 1, updated_at = ?2
                 WHERE id = ?1",
                params![id, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("Partner not found: {}", id));
        }
        self.get_partner(id)?
            .ok_or_else(|| format!("Partner '{}' missing after certify", id))
    }

    pub fn configure_distribution(
        &self,
        id: &str,
        org_id: Option<String>,
        distribution_channel: String,
        artifact_base_url: String,
        updater_pubkey: Option<String>,
        contact_email: Option<String>,
        units_shipped: Option<u64>,
    ) -> Result<HardwarePartner, String> {
        let conn = self.open()?;
        let changed = conn
            .execute(
                "UPDATE hardware_partners
                 SET org_id = COALESCE(?2, org_id),
                     distribution_channel = ?3,
                     artifact_base_url = ?4,
                     updater_pubkey = ?5,
                     contact_email = COALESCE(?6, contact_email),
                     units_shipped = COALESCE(?7, units_shipped),
                     updated_at = ?8
                 WHERE id = ?1",
                params![
                    id,
                    org_id,
                    distribution_channel,
                    artifact_base_url,
                    updater_pubkey,
                    contact_email,
                    units_shipped.map(|value| value as i64),
                    chrono::Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("Partner not found: {}", id));
        }
        self.get_partner(id)?
            .ok_or_else(|| format!("Partner '{}' missing after distribution config", id))
    }

    pub fn prepare_distribution_bundle(
        &self,
        id: &str,
        branding: &BrandingConfig,
        marketplace_view: &OrgMarketplaceView,
    ) -> Result<PartnerDistributionBundle, String> {
        let partner = self
            .get_partner(id)?
            .ok_or_else(|| format!("Partner not found: {}", id))?;
        if !partner.certified {
            return Err("Partner must be certified before preparing distribution bundle".to_string());
        }
        let distribution_channel = partner
            .distribution_channel
            .clone()
            .ok_or_else(|| "Partner distribution channel is not configured".to_string())?;
        let artifact_base_url = partner
            .artifact_base_url
            .clone()
            .ok_or_else(|| "Partner artifact base URL is not configured".to_string())?;

        let manifest = serde_json::json!({
            "partner": {
                "id": partner.id,
                "company": partner.company,
                "slug": partner.slug,
                "integration_level": partner.integration_level.as_str(),
                "distribution_channel": distribution_channel,
                "certified": partner.certified,
                "contact_email": partner.contact_email,
                "units_shipped": partner.units_shipped,
            },
            "branding": branding,
            "catalog": marketplace_view.listings,
            "distribution": {
                "artifact_base_url": artifact_base_url,
                "updater_pubkey_present": partner
                    .updater_pubkey
                    .as_ref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false),
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
        });

        let manifest_path = self.bundle_dir.join(format!("{}.json", partner.slug));
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        let conn = self.open()?;
        conn.execute(
            "UPDATE hardware_partners
             SET distribution_bundle_path = ?2, updated_at = ?3
             WHERE id = ?1",
            params![
                id,
                manifest_path.to_string_lossy().to_string(),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(PartnerDistributionBundle {
            partner_id: partner.id,
            partner_slug: partner.slug,
            org_id: partner.org_id,
            app_name: branding.app_name.clone(),
            distribution_channel,
            artifact_base_url,
            updater_pubkey_present: partner
                .updater_pubkey
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
            catalog_items: marketplace_view.listings.len(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            manifest,
        })
    }
}

fn map_partner(row: &rusqlite::Row<'_>) -> rusqlite::Result<HardwarePartner> {
    Ok(HardwarePartner {
        id: row.get(0)?,
        company: row.get(1)?,
        slug: row.get(2)?,
        org_id: row.get(3)?,
        device_type: row.get(4)?,
        integration_level: IntegrationLevel::from_str(&row.get::<_, String>(5)?),
        certified: row.get::<_, i64>(6)? != 0,
        contact_email: row.get(7)?,
        units_shipped: row.get::<_, Option<i64>>(8)?.map(|value| value as u64),
        distribution_channel: row.get(9)?,
        artifact_base_url: row.get(10)?,
        updater_pubkey: row.get(11)?,
        distribution_bundle_path: row.get(12)?,
        registered_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn slugify(input: &str) -> String {
    let slug = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.trim_matches('-')
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

impl IntegrationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrationLevel::Basic => "basic",
            IntegrationLevel::Premium => "premium",
            IntegrationLevel::Exclusive => "exclusive",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "premium" => IntegrationLevel::Premium,
            "exclusive" => IntegrationLevel::Exclusive,
            _ => IntegrationLevel::Basic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branding::config::OEMLicense;
    use crate::marketplace::{OrgListing, OrgMarketplace};
    use tempfile::tempdir;

    fn partner_branding() -> BrandingConfig {
        BrandingConfig {
            app_name: "Acme AgentOS".to_string(),
            tagline: "Partner distribution".to_string(),
            primary_color: "#ff5500".to_string(),
            accent_color: "#ff5500".to_string(),
            oem_license: Some(OEMLicense {
                tier: "premium".to_string(),
                company: "Acme".to_string(),
                issued_at: chrono::Utc::now().to_rfc3339(),
                expires_at: None,
            }),
            ..BrandingConfig::default()
        }
    }

    #[test]
    fn partner_registry_persists_and_builds_distribution_bundle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("partners.db");
        let registry = PartnerRegistry::new(db_path.clone()).unwrap();
        let market = OrgMarketplace::new(db_path).unwrap();

        let partner = registry
            .register_partner(
                "Acme Devices".to_string(),
                "laptop".to_string(),
                IntegrationLevel::Premium,
                Some("acme".to_string()),
            )
            .unwrap();
        registry.certify(&partner.id).unwrap();
        registry
            .configure_distribution(
                &partner.id,
                Some("acme".to_string()),
                "oem-installer".to_string(),
                "https://downloads.example.com/acme".to_string(),
                Some("pubkey-demo".to_string()),
                Some("ops@acme.example".to_string()),
                Some(50000),
            )
            .unwrap();

        let listing = market
            .publish(OrgListing {
                id: String::new(),
                org_id: "acme".to_string(),
                resource_type: "playbook".to_string(),
                resource_id: "acme-onboarding".to_string(),
                visibility: "public".to_string(),
                approved: false,
                created_at: String::new(),
            })
            .unwrap();
        market.approve(&listing.id).unwrap();
        market
            .set_branding("acme", &partner_branding())
            .unwrap();
        let view = market
            .get_view_for_org("acme", &BrandingConfig::default())
            .unwrap();

        let bundle = registry
            .prepare_distribution_bundle(&partner.id, &view.branding, &view)
            .unwrap();
        let reloaded = registry.get_partner(&partner.id).unwrap().unwrap();

        assert_eq!(bundle.app_name, "Acme AgentOS");
        assert_eq!(bundle.catalog_items, 1);
        assert!(std::path::Path::new(&bundle.manifest_path).exists());
        assert!(reloaded.distribution_bundle_path.is_some());
        assert_eq!(registry.list_partners().unwrap().len(), 1);
    }
}
