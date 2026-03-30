use serde::{Deserialize, Serialize};

/// R134 — Real Estate vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub id: String,
    pub address: String,
    pub price: f64,
    pub bedrooms: u32,
    pub bathrooms: f64,
    pub sqft: u32,
    pub status: PropertyStatus,
    pub property_type: String,
    pub year_built: Option<u32>,
    pub listed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PropertyStatus {
    Active,
    Pending,
    Sold,
    OffMarket,
}

pub struct RealEstateAgent {
    properties: Vec<Property>,
    next_id: u64,
}

impl RealEstateAgent {
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new property listing.
    pub fn add_property(
        &mut self,
        address: String,
        price: f64,
        bedrooms: u32,
        bathrooms: f64,
        sqft: u32,
        property_type: String,
    ) -> Property {
        let property = Property {
            id: format!("prop_{}", self.next_id),
            address,
            price,
            bedrooms,
            bathrooms,
            sqft,
            status: PropertyStatus::Active,
            property_type,
            year_built: None,
            listed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.properties.push(property.clone());
        property
    }

    /// Search properties by criteria.
    pub fn search_properties(
        &self,
        min_price: Option<f64>,
        max_price: Option<f64>,
        min_bedrooms: Option<u32>,
        min_sqft: Option<u32>,
    ) -> Vec<&Property> {
        self.properties
            .iter()
            .filter(|p| {
                p.status == PropertyStatus::Active
                    && min_price.map_or(true, |mp| p.price >= mp)
                    && max_price.map_or(true, |mp| p.price <= mp)
                    && min_bedrooms.map_or(true, |mb| p.bedrooms >= mb)
                    && min_sqft.map_or(true, |ms| p.sqft >= ms)
            })
            .collect()
    }

    /// Calculate ROI for a property given purchase price and expected monthly rent.
    pub fn calculate_roi(
        &self,
        property_id: &str,
        monthly_rent: f64,
        annual_expenses: f64,
    ) -> Result<serde_json::Value, String> {
        let property = self
            .properties
            .iter()
            .find(|p| p.id == property_id)
            .ok_or_else(|| format!("Property not found: {}", property_id))?;

        let annual_income = monthly_rent * 12.0;
        let net_income = annual_income - annual_expenses;
        let cap_rate = if property.price > 0.0 {
            (net_income / property.price) * 100.0
        } else {
            0.0
        };
        let cash_on_cash = if property.price > 0.0 {
            (net_income / (property.price * 0.20)) * 100.0 // assuming 20% down
        } else {
            0.0
        };
        let price_per_sqft = if property.sqft > 0 {
            property.price / property.sqft as f64
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "property_id": property.id,
            "address": property.address,
            "purchase_price": property.price,
            "annual_income": annual_income,
            "annual_expenses": annual_expenses,
            "net_operating_income": net_income,
            "cap_rate_pct": (cap_rate * 100.0).round() / 100.0,
            "cash_on_cash_pct": (cash_on_cash * 100.0).round() / 100.0,
            "price_per_sqft": (price_per_sqft * 100.0).round() / 100.0,
            "gross_rent_multiplier": if annual_income > 0.0 { property.price / annual_income } else { 0.0 },
        }))
    }

    /// Generate a listing description for a property.
    pub fn generate_listing(&self, property_id: &str) -> Result<serde_json::Value, String> {
        let property = self
            .properties
            .iter()
            .find(|p| p.id == property_id)
            .ok_or_else(|| format!("Property not found: {}", property_id))?;

        let description = format!(
            "Beautiful {} at {}. This stunning {}-bedroom, {}-bath home offers {} sq ft of living space. \
             Listed at ${:.0}. Don't miss this incredible opportunity!",
            property.property_type, property.address, property.bedrooms,
            property.bathrooms, property.sqft, property.price,
        );

        Ok(serde_json::json!({
            "property_id": property.id,
            "listing_title": format!("{} {} — {} bed / {} bath", property.property_type, property.address, property.bedrooms, property.bathrooms),
            "description": description,
            "highlights": [
                format!("{} bedrooms", property.bedrooms),
                format!("{} bathrooms", property.bathrooms),
                format!("{} sq ft", property.sqft),
                format!("${:.0}", property.price),
            ],
        }))
    }
}
