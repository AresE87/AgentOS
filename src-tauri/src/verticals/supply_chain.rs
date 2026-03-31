use serde::{Deserialize, Serialize};

/// R137 — Supply Chain vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shipment {
    pub id: String,
    pub origin: String,
    pub destination: String,
    pub status: ShipmentStatus,
    pub carrier: String,
    pub eta: String,
    pub weight_kg: f64,
    pub items: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShipmentStatus {
    Pending,
    PickedUp,
    InTransit,
    CustomsHold,
    OutForDelivery,
    Delivered,
    Returned,
}

pub struct SupplyChainManager {
    shipments: Vec<Shipment>,
    next_id: u64,
}

impl SupplyChainManager {
    pub fn new() -> Self {
        Self {
            shipments: Vec::new(),
            next_id: 1,
        }
    }

    /// Track a shipment by ID.
    pub fn track_shipment(&self, shipment_id: &str) -> Result<serde_json::Value, String> {
        let shipment = self
            .shipments
            .iter()
            .find(|s| s.id == shipment_id)
            .ok_or_else(|| format!("Shipment not found: {}", shipment_id))?;

        Ok(serde_json::json!({
            "shipment_id": shipment.id,
            "origin": shipment.origin,
            "destination": shipment.destination,
            "status": shipment.status,
            "carrier": shipment.carrier,
            "eta": shipment.eta,
            "weight_kg": shipment.weight_kg,
            "items": shipment.items,
        }))
    }

    /// Optimize route for a shipment (stub — returns recommendation).
    pub fn optimize_route(
        &self,
        origin: &str,
        destination: &str,
        weight_kg: f64,
    ) -> serde_json::Value {
        let distance_estimate = 500.0 + (weight_kg * 0.5); // simplified
        let recommended_carrier = if weight_kg > 1000.0 {
            "FreightCorp"
        } else if weight_kg > 100.0 {
            "LogiExpress"
        } else {
            "QuickShip"
        };
        let cost_estimate = distance_estimate * 0.15 + weight_kg * 0.50;

        serde_json::json!({
            "origin": origin,
            "destination": destination,
            "weight_kg": weight_kg,
            "recommended_carrier": recommended_carrier,
            "estimated_distance_km": distance_estimate,
            "estimated_cost": (cost_estimate * 100.0).round() / 100.0,
            "estimated_days": if weight_kg > 1000.0 { 7 } else { 3 },
            "route": [origin, "Distribution Hub", destination],
        })
    }

    /// Forecast demand based on historical pattern (stub).
    pub fn forecast_demand(&self, product: &str, period_months: u32) -> serde_json::Value {
        let base_demand = 100.0;
        let forecasts: Vec<serde_json::Value> = (1..=period_months)
            .map(|m| {
                let seasonal_factor = 1.0 + 0.15 * ((m as f64 * std::f64::consts::PI / 6.0).sin());
                let demand = base_demand * seasonal_factor * (1.0 + 0.02 * m as f64);
                serde_json::json!({
                    "month": m,
                    "forecasted_units": (demand).round(),
                    "confidence": 0.85 - (0.02 * m as f64),
                })
            })
            .collect();

        serde_json::json!({
            "product": product,
            "period_months": period_months,
            "forecast": forecasts,
            "model": "seasonal_trend_v1",
        })
    }

    /// List all shipments, optionally filtered by status.
    pub fn list_shipments(&self, status_filter: Option<&str>) -> Vec<&Shipment> {
        self.shipments
            .iter()
            .filter(|s| {
                status_filter.map_or(true, |sf| {
                    let st = serde_json::to_string(&s.status)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string();
                    st == sf
                })
            })
            .collect()
    }

    /// Add a shipment (used by IPC).
    pub fn add_shipment(
        &mut self,
        origin: String,
        destination: String,
        carrier: String,
        eta: String,
        weight_kg: f64,
        items: Vec<String>,
    ) -> Shipment {
        let shipment = Shipment {
            id: format!("ship_{}", self.next_id),
            origin,
            destination,
            status: ShipmentStatus::Pending,
            carrier,
            eta,
            weight_kg,
            items,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.shipments.push(shipment.clone());
        shipment
    }
}
