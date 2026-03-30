use serde::{Deserialize, Serialize};

/// R95: An organization marketplace listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgListing {
    pub id: String,
    pub org_id: String,
    pub resource_type: String, // "playbook", "persona", "template"
    pub resource_id: String,
    pub visibility: String,    // "org_only", "public"
    pub approved: bool,
    pub created_at: String,
}

/// R95: White-label marketplace scoped to organizations
pub struct OrgMarketplace {
    listings: Vec<OrgListing>,
}

impl OrgMarketplace {
    pub fn new() -> Self {
        Self {
            listings: Vec::new(),
        }
    }

    pub fn publish(&mut self, mut listing: OrgListing) -> OrgListing {
        listing.id = uuid::Uuid::new_v4().to_string();
        listing.created_at = chrono::Utc::now().to_rfc3339();
        listing.approved = false;
        self.listings.push(listing.clone());
        listing
    }

    pub fn list_for_org(&self, org_id: &str) -> Vec<&OrgListing> {
        self.listings.iter()
            .filter(|l| l.org_id == org_id)
            .collect()
    }

    pub fn approve(&mut self, listing_id: &str) -> Result<(), String> {
        let listing = self.listings.iter_mut()
            .find(|l| l.id == listing_id)
            .ok_or_else(|| format!("Listing not found: {}", listing_id))?;
        listing.approved = true;
        Ok(())
    }

    pub fn remove(&mut self, listing_id: &str) -> Result<(), String> {
        let idx = self.listings.iter()
            .position(|l| l.id == listing_id)
            .ok_or_else(|| format!("Listing not found: {}", listing_id))?;
        self.listings.remove(idx);
        Ok(())
    }

    pub fn search(&self, query: &str, org_id: &str) -> Vec<&OrgListing> {
        let q = query.to_lowercase();
        self.listings.iter()
            .filter(|l| {
                (l.org_id == org_id || l.visibility == "public")
                    && (l.resource_type.to_lowercase().contains(&q)
                        || l.resource_id.to_lowercase().contains(&q))
            })
            .collect()
    }
}
