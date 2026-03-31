use serde::{Deserialize, Serialize};

/// R136 — HR vertical module.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: String,
    pub name: String,
    pub department: String,
    pub role: String,
    pub hire_date: String,
    pub status: EmployeeStatus,
    pub salary: Option<f64>,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeStatus {
    Active,
    OnLeave,
    Terminated,
    Probation,
}

pub struct HRManager {
    employees: Vec<Employee>,
    next_id: u64,
}

impl HRManager {
    pub fn new() -> Self {
        Self {
            employees: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new employee.
    pub fn add_employee(
        &mut self,
        name: String,
        department: String,
        role: String,
        hire_date: String,
        salary: Option<f64>,
        email: String,
    ) -> Employee {
        let employee = Employee {
            id: format!("emp_{}", self.next_id),
            name,
            department,
            role,
            hire_date,
            status: EmployeeStatus::Active,
            salary,
            email,
        };
        self.next_id += 1;
        self.employees.push(employee.clone());
        employee
    }

    /// List employees, optionally filtered by department or status.
    pub fn list_employees(&self, department: Option<&str>, status: Option<&str>) -> Vec<&Employee> {
        self.employees
            .iter()
            .filter(|e| {
                department.map_or(true, |d| e.department.to_lowercase() == d.to_lowercase())
                    && status.map_or(true, |s| {
                        let st = serde_json::to_string(&e.status)
                            .unwrap_or_default()
                            .trim_matches('"')
                            .to_string();
                        st == s
                    })
            })
            .collect()
    }

    /// Generate an offer letter for a candidate.
    pub fn generate_offer_letter(
        &self,
        candidate_name: &str,
        role: &str,
        department: &str,
        salary: f64,
        start_date: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "type": "offer_letter",
            "candidate": candidate_name,
            "role": role,
            "department": department,
            "salary": salary,
            "start_date": start_date,
            "content": format!(
                "Dear {},\n\nWe are pleased to offer you the position of {} in our {} department.\n\n\
                 Compensation: ${:.2}/year\nStart Date: {}\n\n\
                 This offer is contingent upon successful completion of background check.\n\n\
                 Please confirm your acceptance within 5 business days.\n\nBest regards,\nHR Department",
                candidate_name, role, department, salary, start_date
            ),
            "generated_at": chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Calculate benefits summary for an employee.
    pub fn calculate_benefits(&self, employee_id: &str) -> Result<serde_json::Value, String> {
        let employee = self
            .employees
            .iter()
            .find(|e| e.id == employee_id)
            .ok_or_else(|| format!("Employee not found: {}", employee_id))?;

        let salary = employee.salary.unwrap_or(50000.0);
        let health_insurance = salary * 0.08;
        let retirement_match = salary * 0.04;
        let pto_value = (salary / 260.0) * 20.0; // 20 PTO days
        let total_benefits = health_insurance + retirement_match + pto_value;

        Ok(serde_json::json!({
            "employee_id": employee.id,
            "employee_name": employee.name,
            "base_salary": salary,
            "benefits": {
                "health_insurance": (health_insurance * 100.0).round() / 100.0,
                "retirement_401k_match": (retirement_match * 100.0).round() / 100.0,
                "pto_value": (pto_value * 100.0).round() / 100.0,
                "dental": 1200.0,
                "vision": 600.0,
            },
            "total_benefits_value": (total_benefits * 100.0).round() / 100.0,
            "total_compensation": ((salary + total_benefits) * 100.0).round() / 100.0,
        }))
    }
}
