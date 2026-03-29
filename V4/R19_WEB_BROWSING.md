# FASE R19 — WEB BROWSING REAL: El agente navega sitios de verdad

**Objetivo:** El agente puede abrir un browser, navegar a un sitio, leer contenido, interactuar con formularios, y extraer información — incluyendo sitios SPA (JavaScript-rendered).

---

## El problema

Invoke-WebRequest solo trae HTML estático. Sitios como MercadoLibre, Gmail, LinkedIn son SPAs que requieren JavaScript. El vision mode puede "ver" el browser pero es lento e impreciso. Necesitamos una solución intermedia.

---

## Estrategia: 3 niveles de web access

### Nivel 1: HTTP simple (ya funciona)
Para sitios estáticos, APIs, y archivos:
```powershell
Invoke-WebRequest -Uri "https://example.com" -OutFile "page.html"
```
Uso: descargas directas, APIs REST, páginas simples.

### Nivel 2: Headless browser (NUEVO)
Para sitios SPA que necesitan JavaScript:
```rust
// Opción A: Usar el WebView2 de Tauri como browser controlable
// Opción B: Integrar chromiumoxide (headless Chrome en Rust)
// Opción C: Llamar a PowerShell con Selenium/Playwright

// Recomendación: chromiumoxide — Rust-native headless Chrome
// Crate: chromiumoxide

async fn browse_page(url: &str) -> Result<PageContent> {
    let browser = Browser::connect("http://127.0.0.1:9222").await?;
    let page = browser.new_page(url).await?;
    page.wait_for_navigation().await?;
    
    let html = page.content().await?;        // HTML renderizado con JS
    let text = page.evaluate("document.body.innerText").await?;  // Texto visible
    let title = page.title().await?;
    let screenshot = page.screenshot().await?; // Para vision si necesario
    
    Ok(PageContent { html, text, title, screenshot })
}
```

### Nivel 3: Vision browser (ya existe, mejorar)
Para interacciones complejas (login, formularios multi-step):
- Abrir browser normal
- Vision mode captura + analiza + click/type
- Ya existe en pipeline/engine.rs, mejorar con:
  - Wait for page load (detectar que la página terminó de cargar)
  - Scroll support (para contenido largo)
  - Tab management (no abrir infinitas pestañas)

---

## Tareas

### 1. Integrar headless browser

```toml
# Cargo.toml
[dependencies]
chromiumoxide = { version = "0.7", features = ["tokio-runtime"] }
```

```rust
// brain/ o pipeline/:
async fn web_search(query: &str, state: &AppState) -> Result<String> {
    // 1. Buscar en DuckDuckGo o Google
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let content = browse_page(&url).await?;
    
    // 2. Enviar el texto al LLM para que extraiga lo relevante
    let prompt = format!(
        "Extract the relevant information from these search results:\n{}\n\nOriginal question: {}",
        &content.text[..4000.min(content.text.len())],
        query
    );
    let answer = gateway.call(&prompt, Tier::Cheap).await?;
    Ok(answer)
}
```

### 2. Web browsing como acción del engine

```rust
// Agregar al pipeline engine:
AgentAction::Browse { url, task } => {
    let content = browse_page(&url).await?;
    // Enviar contenido al LLM para analizar
    let analysis = gateway.call(&format!("Analyze this page content: {}\n\nTask: {}", content.text, task)).await?;
    analysis
}
```

### 3. El LLM puede pedir browse como acción

```json
{"action": "browse", "url": "https://example.com", "task": "find the pricing page"}
```

El LLM recibe de vuelta el texto visible de la página y puede pedir otra acción.

### 4. Limitaciones y safety

```rust
// Blocked URLs (safety):
const BLOCKED_DOMAINS: &[&str] = &[
    "localhost", "127.0.0.1", "192.168.", "10.", "172.16.",  // Redes internas
    // No bloquear nada más — el usuario decide qué navegar
];

// Limits:
const MAX_PAGES_PER_TASK: usize = 10;  // No abrir 100 pestañas
const MAX_PAGE_TEXT_LENGTH: usize = 8000;  // Truncar para el LLM
const PAGE_LOAD_TIMEOUT: Duration = Duration::from_secs(15);
```

---

## Cómo verificar

1. "Buscá en internet cuánto cuesta un iPhone 16 en Argentina" → resultado real con precios
2. "Andá a weather.com y decime el clima de Montevideo" → datos reales del clima
3. "Buscá en Wikipedia qué es Rust programming language y haceme un resumen" → resumen real
4. Sitio SPA (ej: React app) → el headless browser renderiza el JS y extrae texto
