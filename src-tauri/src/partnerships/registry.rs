use serde::{Deserialize, Serialize};

/// Integration depth with AgentOS
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationLevel {
    Basic,
    Premium,
    Exclusive,
}

/// A hardware partner entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwarePartner {
    pub id: String,
    pub company: String,
    pub device_type: String,
    pub integration_level: IntegrationLevel,
    pub certified: bool,
    pub contact_email: Option<String>,
    pub units_shipped: Option<u64>,
    pub registered_at: String,
}

/// Registry for hardware partner management
pub struct PartnerRegistry {
    partners: Vec<HardwarePartner>,
}

impl PartnerRegistry {
    pub fn new() -> Self {
        Self {
            partners: Vec::new(),
        }
    }

    /// List all registered partners
    pub fn list_partners(&self) -> Vec<HardwarePartner> {
        self.partners.clone()
    }

    /// Get a single partner by id
    pub fn get_partner(&self, id: &str) -> Option<HardwarePartner> {
        self.partners.iter().find(|p| p.id == id).cloned()
    }

    /// Register a new hardware partner
    pub fn register_partner(
        &mut self,
        company: String,
        device_type: String,
        integration_level: IntegrationLevel,
    ) -> HardwarePartner {
        let partner = HardwarePartner {
            id: uuid::Uuid::new_v4().to_string(),
            company: company.clone(),
            device_type,
            integration_level,
            certified: false,
            contact_email: None,
            units_shipped: None,
            registered_at: chrono::Utc::now().to_rfc3339(),
        };
        self.partners.push(partner.clone());
        tracing::info!("Hardware partner registered: {} (id={})", company, partner.id);
        partner
    }

    /// Certify a partner (mark as certified after testing)
    pub fn certify(&mut self, id: &str) -> Result<HardwarePartner, String> {
        let partner = self
            .partners
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Partner not found: {}", id))?;

        partner.certified = true;
        tracing::info!("Hardware partner certified: {}", partner.company);
        Ok(partner.clone())
    }
}
