# FASE R115 — AUTONOMOUS QA: El agente testea software por vos

**Objetivo:** Darle al agente una app web o desktop y que la testee automáticamente: navega UIs, prueba flujos, encuentra bugs visuales, reporta issues. El QA engineer duerme mientras el agente testea toda la noche.

---

## Tareas

### 1. Test plan generation
```
Input: URL o app name + brief description
Output: Test plan generado por LLM:

"Testing Plan for: myapp.com
1. Smoke test: homepage loads, main navigation works
2. Auth flow: register, login, logout, forgot password
3. Core features: create item, edit, delete, search, filter
4. Edge cases: empty inputs, very long text, special characters
5. Visual: responsive (desktop, tablet, mobile viewports)
6. Performance: page load times, large data sets
7. Accessibility: tab navigation, screen reader, contrast"
```

### 2. Automated test execution (web)
```rust
// Usando headless browser (R19 chromiumoxide):
pub struct WebTester {
    pub async fn test_flow(&self, url: &str, flow: &TestFlow) -> TestReport {
        // 1. Navigate to URL
        // 2. For each step in flow:
        //    a. Take screenshot
        //    b. Vision: verify expected state
        //    c. Execute action (click, type, submit)
        //    d. Wait for response
        //    e. Vision: verify result
        //    f. Compare with expected
        // 3. Generate report with screenshots and findings
    }
}
```

### 3. Automated test execution (desktop — vision mode)
```
// Para apps desktop sin API de testing:
// 1. Abrir la app (vision R11)
// 2. Seguir el test plan step by step
// 3. En cada step: screenshot → verify → act → verify result
// 4. Si algo falla: screenshot + description → bug report

// Esto es ÚNICO: testear apps legacy con vision, sin Selenium/Playwright
```

### 4. Bug report generation
```
BUG REPORT #QA-001
──────────────────
Severity: 🔴 High
Feature: Login form
Steps to reproduce:
  1. Navigate to /login
  2. Enter email: "test@example.com"
  3. Enter password: (empty)
  4. Click "Login"
Expected: Error message "Password required"
Actual: Page crashes with 500 error
Screenshot: [attached]

Tested by: AgentOS QA Agent
Tested at: 2026-03-29 03:42 AM
Browser: Chrome 120 (headless)
```

### 5. Continuous testing
```
Trigger: after every deploy (webhook from CI/CD)
→ Run full test suite
→ Generate report
→ If bugs found → create Jira tickets automatically
→ Notify team on Slack
```

---

## Demo
1. "Test myapp.com" → agent generates test plan → executes 20 tests → report with 3 bugs found
2. Bug report with screenshot showing exactly what went wrong
3. Desktop app: agent opens Calculator → tests all buttons → reports "% button doesn't work"
4. Webhook: deploy triggers test suite → 2am → report ready at 6am
5. Integration: bug auto-created in Jira with screenshot and steps
