# FASE R158 — PRIVACY-PRESERVING ANALYTICS: Datos sin identificar a nadie

**Objetivo:** Los analytics de AgentOS (R76, R148) nunca pueden ser usados para identificar a un usuario individual. Differential privacy matemáticamente garantiza que no se puede re-identificar a nadie desde los datos agregados.

---

## Tareas

### 1. Differential privacy implementation

```rust
use rand_distr::{Laplace, Distribution};

pub struct DifferentialPrivacy {
    pub epsilon: f64,  // Privacy budget (lower = more private)
}

impl DifferentialPrivacy {
    /// Agregar ruido a un valor numérico
    pub fn add_noise(&self, value: f64, sensitivity: f64) -> f64 {
        let scale = sensitivity / self.epsilon;
        let noise = Laplace::new(0.0, scale).unwrap();
        value + noise.sample(&mut rand::thread_rng())
    }
    
    /// Agregar ruido a un conteo
    pub fn noisy_count(&self, count: usize) -> usize {
        let noisy = self.add_noise(count as f64, 1.0);
        noisy.max(0.0).round() as usize
    }
    
    /// Agregar ruido a un promedio
    pub fn noisy_average(&self, values: &[f64], bounds: (f64, f64)) -> f64 {
        let sensitivity = (bounds.1 - bounds.0) / values.len() as f64;
        let avg = values.iter().sum::<f64>() / values.len() as f64;
        self.add_noise(avg, sensitivity)
    }
}
```

### 2. K-anonymity para datos de usuarios

```rust
// Antes de exportar o mostrar datos agregados:
// Verificar que cada grupo tiene al menos K usuarios (K=5 default)
// Si un grupo tiene < K usuarios → suprimir o generalizar

pub fn enforce_k_anonymity(data: &mut GroupedData, k: usize) {
    data.groups.retain(|group| group.user_count >= k);
    // "Users from Uruguay with medical vertical" → si < 5 → no mostrar
}
```

### 3. Aplicar a todos los analytics dashboards

```
// Creator analytics (R148):
// "3 users from Chile" → suppressed (< K=5)
// "45 users from Uruguay" → shown (≥ K=5)

// Revenue analytics (R97):
// Per-user revenue → never shown (individual data)
// Revenue by country → only if ≥ 5 users per country

// Federated learning (R92):
// Gradients already have noise (DP applied in R92)
// Additional verification: gradients can't reconstruct individual data
```

### 4. Privacy audit tool

```rust
pub fn audit_privacy(analytics_output: &AnalyticsReport) -> PrivacyAuditResult {
    // Verify:
    // 1. No individual user data in any chart/table
    // 2. All groups have ≥ K users
    // 3. Differential privacy noise applied to all counts/averages
    // 4. No cross-referencing possible between datasets
    // 5. Export doesn't contain user IDs, emails, or names
}
```

---

## Demo

1. Analytics dashboard: todos los números tienen DP noise → "~1,247 tasks" (not exact)
2. Small group suppression: "Chile: data suppressed (< 5 users)" → privacy protected
3. Privacy audit: "✅ All analytics pass privacy verification"
4. Creator dashboard: revenue by country only shows countries with 5+ users
