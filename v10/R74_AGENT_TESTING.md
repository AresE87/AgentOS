# FASE R74 — AGENT TESTING FRAMEWORK: TDD para agentes

**Objetivo:** El usuario puede escribir tests para sus playbooks y specialists: "dado este input, el agente debería producir output que contenga X". Los tests corren automáticamente antes de publicar en marketplace, y como CI para workflows.

---

## Tareas

### 1. Test format

```yaml
# tests/system-monitor.test.yaml
name: "System Monitor Playbook Tests"
playbook: "system-monitor"

tests:
  - name: "Should report disk usage"
    input: "check disk space"
    expect:
      output_contains: ["disk", "usage", "%"]
      status: "completed"
      max_cost: 0.01
      max_latency_ms: 5000

  - name: "Should report CPU info"
    input: "check CPU usage"
    expect:
      output_contains: ["CPU", "cores"]
      status: "completed"

  - name: "Should handle no-permission gracefully"
    input: "check disk space"
    mock:
      cli_blocked: true
    expect:
      status: "failed"
      output_contains: ["permission", "denied"]
```

### 2. Test runner

```rust
pub struct AgentTestRunner;

impl AgentTestRunner {
    pub async fn run_suite(&self, test_file: &Path) -> Result<TestReport> {
        let suite: TestSuite = serde_yaml::from_str(&fs::read_to_string(test_file)?)?;
        let mut results = Vec::new();
        
        for test in &suite.tests {
            let result = self.run_single_test(test, &suite.playbook).await;
            results.push(result);
        }
        
        Ok(TestReport { suite: suite.name, results, passed: results.iter().filter(|r| r.passed).count() })
    }
    
    async fn run_single_test(&self, test: &TestCase, playbook: &str) -> TestResult {
        // 1. Activar playbook
        // 2. Aplicar mocks si hay
        // 3. Enviar input al engine
        // 4. Capturar output
        // 5. Verificar expectations
        // 6. Retornar passed/failed con detalles
    }
}
```

### 3. Mocking layer

```rust
// Para tests, poder mockear:
// - LLM responses (respuesta fija en vez de llamar API)
// - CLI output (simular output de PowerShell)
// - Screen capture (screenshot fijo)
// - File system (archivos virtuales)
// - Network (sin internet)

pub struct TestMocks {
    pub llm_response: Option<String>,
    pub cli_output: Option<String>,
    pub cli_blocked: bool,
    pub offline: bool,
}
```

### 4. Frontend: Test runner UI

```
TESTS                                    [Run All] [+ New Test]
──────────────────────────────────────────────────────────

system-monitor.test.yaml                 3/3 passed ✅
├── ✅ Should report disk usage          0.8s  $0.001
├── ✅ Should report CPU info            1.2s  $0.002
└── ✅ Should handle no-permission       0.1s  $0.000

code-reviewer.test.yaml                  2/3 passed ⚠️
├── ✅ Should find SQL injection         2.1s  $0.012
├── ✅ Should approve clean code         1.8s  $0.010
└── ❌ Should detect XSS                 3.5s  $0.015
       Expected: output contains "XSS"
       Got: "The code looks clean..."

[View Details] [Re-run Failed] [Export Report]
```

### 5. Integration con marketplace

```
// Antes de publicar un playbook/agent en marketplace:
// 1. Verificar que tiene test file
// 2. Ejecutar tests
// 3. Si alguno falla → bloquear publicación con mensaje
// 4. Badge en marketplace: "✅ 5/5 tests passing"
```

### 6. IPC commands

```rust
#[tauri::command] async fn test_run_suite(path: String) -> Result<TestReport, String>
#[tauri::command] async fn test_run_single(path: String, test_name: String) -> Result<TestResult, String>
#[tauri::command] async fn test_list_suites() -> Result<Vec<TestSuiteSummary>, String>
#[tauri::command] async fn test_create_template(playbook: String) -> Result<String, String>  // genera test template
```

---

## Demo

1. Crear test para system-monitor → Run → 3/3 passed ✅
2. Test que falla → ver detalle: expected vs got → fix playbook → re-run → passes
3. Intentar publicar playbook sin tests → warning "Add tests before publishing"
4. Marketplace badge: "✅ Tests passing"
5. "Generate test template" → auto-genera tests básicos para un playbook existente
