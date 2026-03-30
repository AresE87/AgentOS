# FASE R92 — FEDERATED LEARNING: Agentes mejoran sin compartir datos

**Objetivo:** Miles de instancias de AgentOS contribuyen a mejorar el modelo colectivamente SIN enviar datos del usuario. Cada instancia entrena localmente, solo comparte los GRADIENTES (no los datos). El modelo central mejora para todos.

---

## Tareas

### 1. Federated training protocol

```rust
// Federated Averaging (FedAvg) simplificado:
//
// Ronda de training:
// 1. Server envía modelo global actual a N clientes
// 2. Cada cliente entrena localmente con SUS datos (nunca salen de la PC)
// 3. Cada cliente envía solo los WEIGHT UPDATES (gradientes) al server
// 4. Server promedia los updates → nuevo modelo global
// 5. Repetir
//
// Privacy: los datos NUNCA salen de la PC del usuario
// Solo viajan: gradientes (números, no texto)

pub struct FederatedClient {
    local_model: EmbeddedLLM,
    training_data: Vec<TrainingPair>,  // Del historial local
}

impl FederatedClient {
    /// Entrenar localmente y generar weight updates
    pub fn local_train(&mut self, global_weights: &Weights, epochs: usize) -> WeightDelta {
        // 1. Cargar global weights
        // 2. Entrenar con datos locales (3-5 epochs)
        // 3. Calcular delta: new_weights - global_weights
        // 4. Retornar delta (NOT the weights, NOT the data)
    }
    
    /// Aplicar nuevo modelo global
    pub fn apply_global_update(&mut self, new_weights: &Weights) {
        self.local_model.load_weights(new_weights);
    }
}
```

### 2. Differential privacy (protección extra)

```rust
// Agregar ruido a los gradientes antes de enviar:
// Esto garantiza matemáticamente que no se pueden reconstruir los datos originales

fn add_differential_privacy(delta: &mut WeightDelta, epsilon: f64, noise_scale: f64) {
    // Clip gradients (bound sensitivity)
    delta.clip(max_norm: 1.0);
    
    // Add Gaussian noise
    for weight in delta.iter_mut() {
        *weight += gaussian_noise(0.0, noise_scale);
    }
}

// epsilon controla el tradeoff privacidad/utilidad
// epsilon = 1.0: muy privado, aprendizaje lento
// epsilon = 10.0: menos privado, aprendizaje rápido
```

### 3. Server de agregación (mínimo)

```rust
// Servicio simple en la nube:
// POST /federated/register    — registrar cliente
// GET  /federated/model       — descargar modelo global actual
// POST /federated/contribute  — enviar weight delta
// GET  /federated/stats       — cuántos clientes, rondas completadas

// El server NO almacena datos de usuarios
// Solo almacena: modelo global + deltas recibidos + promedio
```

### 4. Qué se mejora con federated learning

```
1. Clasificador de tareas (R81 DistilBERT ONNX):
   - Cada usuario tiene clasificaciones correctas/incorrectas
   - El clasificador mejora con datos de todos los usuarios
   - Resultado: clasificación más precisa para TODOS

2. Routing table:
   - Qué modelo funciona mejor para qué tipo de tarea
   - Promediado de miles de usuarios
   - Resultado: routing más inteligente para TODOS

3. Agent selection:
   - Qué specialist funciona mejor para qué keyword
   - Resultado: agent matching más preciso
```

### 5. Frontend: Federated settings

```
FEDERATED LEARNING                              [Learn more]
──────────────────────────────────────────────────
Help improve AgentOS for everyone, privately.

How it works:
Your AgentOS learns from YOUR usage locally.
Only mathematical gradients are shared — never your data.
The global model improves for all users.

Status: Contributing (1,247 clients active)
Last contribution: 2 hours ago
Rounds completed: 34
Your local accuracy improvement: +4.2% since joining

[x] Participate in federated learning (opt-in)
[ ] Use extra-private mode (more noise, slower improvement)

Privacy guarantee:
✅ Your data never leaves your PC
✅ Differential privacy applied (ε=2.0)
✅ Verified by [independent auditor]
```

---

## Demo

1. Opt-in → "Contributing to round 35 with 1,246 other users"
2. Clasificador accuracy: 89% → 93% después de 10 rondas de federated training
3. Settings muestra: "Your data NEVER leaves your PC" con explicación visual
4. Desactivar → "Stopped contributing. You still benefit from the global model."
5. Extra-private mode: más ruido en gradientes → contribución más lenta pero más segura
