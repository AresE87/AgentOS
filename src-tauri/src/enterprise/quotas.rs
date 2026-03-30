use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentQuota {
    pub department: String,
    pub monthly_budget: f64,
    pub max_tasks_per_day: u32,
    pub allowed_models: Vec<String>,
}

pub struct QuotaManager {
    quotas: Mutex<HashMap<String, DepartmentQuota>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_quota(&self, quota: DepartmentQuota) -> Result<(), String> {
        let mut store = self.quotas.lock().map_err(|e| e.to_string())?;
        store.insert(quota.department.clone(), quota);
        Ok(())
    }

    pub fn get_quota(&self, department: &str) -> Result<Option<DepartmentQuota>, String> {
        let store = self.quotas.lock().map_err(|e| e.to_string())?;
        Ok(store.get(department).cloned())
    }

    /// Check whether the department is within its quota limits.
    /// Returns Ok(()) if within limits, Err with reason if exceeded.
    pub fn check_quota(&self, department: &str) -> Result<(), String> {
        let store = self.quotas.lock().map_err(|e| e.to_string())?;
        match store.get(department) {
            Some(quota) => {
                if quota.monthly_budget <= 0.0 {
                    return Err(format!(
                        "Department '{}' has exhausted its monthly budget",
                        department
                    ));
                }
                if quota.max_tasks_per_day == 0 {
                    return Err(format!(
                        "Department '{}' has zero daily task allowance",
                        department
                    ));
                }
                Ok(())
            }
            None => {
                // No quota set — allow by default
                Ok(())
            }
        }
    }

    pub fn list_quotas(&self) -> Result<Vec<DepartmentQuota>, String> {
        let store = self.quotas.lock().map_err(|e| e.to_string())?;
        Ok(store.values().cloned().collect())
    }
}
