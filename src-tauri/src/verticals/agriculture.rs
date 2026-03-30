use serde::{Deserialize, Serialize};

/// R139 — Agriculture vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropPlan {
    pub id: String,
    pub crop: String,
    pub field: String,
    pub field_acres: f64,
    pub planted_date: String,
    pub expected_harvest: String,
    pub status: CropStatus,
    pub notes: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CropStatus {
    Planned,
    Planted,
    Growing,
    Flowering,
    Harvesting,
    Harvested,
    Failed,
}

pub struct AgricultureAssistant {
    plans: Vec<CropPlan>,
    next_id: u64,
}

impl AgricultureAssistant {
    pub fn new() -> Self {
        Self {
            plans: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new crop plan.
    pub fn create_plan(
        &mut self,
        crop: String,
        field: String,
        field_acres: f64,
        planted_date: String,
        expected_harvest: String,
    ) -> CropPlan {
        let plan = CropPlan {
            id: format!("crop_{}", self.next_id),
            crop,
            field,
            field_acres,
            planted_date,
            expected_harvest,
            status: CropStatus::Planned,
            notes: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.next_id += 1;
        self.plans.push(plan.clone());
        plan
    }

    /// Assess weather impact on crops.
    pub fn weather_impact(
        &self,
        crop_id: &str,
        temperature_c: f64,
        rainfall_mm: f64,
        humidity_pct: f64,
    ) -> Result<serde_json::Value, String> {
        let plan = self
            .plans
            .iter()
            .find(|p| p.id == crop_id)
            .ok_or_else(|| format!("Crop plan not found: {}", crop_id))?;

        let temp_risk = if temperature_c < 0.0 {
            "critical — frost damage likely"
        } else if temperature_c > 40.0 {
            "critical — heat stress"
        } else if temperature_c < 5.0 || temperature_c > 35.0 {
            "moderate — suboptimal growing conditions"
        } else {
            "low — optimal range"
        };

        let rain_risk = if rainfall_mm > 150.0 {
            "high — flooding risk"
        } else if rainfall_mm < 10.0 {
            "high — drought conditions"
        } else {
            "low — adequate moisture"
        };

        let humidity_risk = if humidity_pct > 90.0 {
            "high — fungal disease risk"
        } else if humidity_pct < 20.0 {
            "moderate — drying stress"
        } else {
            "low"
        };

        Ok(serde_json::json!({
            "crop_id": plan.id,
            "crop": plan.crop,
            "field": plan.field,
            "conditions": {
                "temperature_c": temperature_c,
                "rainfall_mm": rainfall_mm,
                "humidity_pct": humidity_pct,
            },
            "risk_assessment": {
                "temperature": temp_risk,
                "rainfall": rain_risk,
                "humidity": humidity_risk,
            },
            "recommendations": [
                if temperature_c < 5.0 { "Consider frost protection covers" } else { "Temperature is adequate" },
                if rainfall_mm < 10.0 { "Increase irrigation immediately" } else if rainfall_mm > 150.0 { "Ensure drainage is clear" } else { "Rainfall is sufficient" },
                if humidity_pct > 90.0 { "Apply preventive fungicide" } else { "Humidity levels are acceptable" },
            ],
        }))
    }

    /// Generate an irrigation schedule for a crop.
    pub fn irrigation_schedule(
        &self,
        crop_id: &str,
        soil_moisture_pct: f64,
    ) -> Result<serde_json::Value, String> {
        let plan = self
            .plans
            .iter()
            .find(|p| p.id == crop_id)
            .ok_or_else(|| format!("Crop plan not found: {}", crop_id))?;

        let water_need = match plan.crop.to_lowercase().as_str() {
            "corn" | "rice" => "high",
            "wheat" | "soybean" => "medium",
            "sorghum" | "millet" => "low",
            _ => "medium",
        };

        let daily_gallons_per_acre = match water_need {
            "high" => 3000.0,
            "medium" => 2000.0,
            _ => 1200.0,
        };

        let adjustment = if soil_moisture_pct < 30.0 {
            1.5
        } else if soil_moisture_pct > 70.0 {
            0.5
        } else {
            1.0
        };

        let daily_total = daily_gallons_per_acre * plan.field_acres * adjustment;

        Ok(serde_json::json!({
            "crop_id": plan.id,
            "crop": plan.crop,
            "field": plan.field,
            "field_acres": plan.field_acres,
            "soil_moisture_pct": soil_moisture_pct,
            "water_need_category": water_need,
            "schedule": {
                "daily_gallons": daily_total.round(),
                "sessions_per_day": if soil_moisture_pct < 30.0 { 3 } else { 2 },
                "duration_minutes": 45,
                "best_times": ["06:00", "18:00"],
            },
            "adjustment_factor": adjustment,
        }))
    }

    /// Forecast yield for a crop.
    pub fn yield_forecast(
        &self,
        crop_id: &str,
        soil_quality: f64,   // 0.0 - 1.0
        pest_pressure: f64,  // 0.0 - 1.0
    ) -> Result<serde_json::Value, String> {
        let plan = self
            .plans
            .iter()
            .find(|p| p.id == crop_id)
            .ok_or_else(|| format!("Crop plan not found: {}", crop_id))?;

        let base_yield_per_acre = match plan.crop.to_lowercase().as_str() {
            "corn" => 180.0,   // bushels
            "wheat" => 50.0,
            "soybean" => 50.0,
            "rice" => 7500.0,  // lbs
            "cotton" => 800.0, // lbs
            _ => 100.0,
        };

        let adjusted_yield = base_yield_per_acre * soil_quality * (1.0 - pest_pressure * 0.4);
        let total_yield = adjusted_yield * plan.field_acres;

        Ok(serde_json::json!({
            "crop_id": plan.id,
            "crop": plan.crop,
            "field": plan.field,
            "field_acres": plan.field_acres,
            "base_yield_per_acre": base_yield_per_acre,
            "adjusted_yield_per_acre": (adjusted_yield * 100.0).round() / 100.0,
            "total_estimated_yield": (total_yield * 100.0).round() / 100.0,
            "factors": {
                "soil_quality": soil_quality,
                "pest_pressure": pest_pressure,
            },
            "confidence": 0.75 - (pest_pressure * 0.2),
        }))
    }
}
