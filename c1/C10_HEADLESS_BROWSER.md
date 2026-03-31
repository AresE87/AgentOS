# CONSOLIDACIÓN C10 — HEADLESS BROWSER REAL

**Estado actual:** ⚠️ Web browsing usa `reqwest` + HTML strip. NO puede navegar SPAs (JavaScript no se ejecuta). Inútil para sitios modernos.
**Objetivo:** Integrar chromiumoxide (headless Chrome en Rust) para navegar sitios JavaScript-rendered.

---

## Qué YA existe

```
src-tauri/src/pipeline/engine.rs:
- Modo web browsing usa reqwest::get(url) → response.text() → strip HTML tags
- Funciona para sitios estáticos pero NO para React/Vue/Angular/SPAs
- El LLM recibe HTML crudo sin JavaScript ejecutado
```

## Qué REEMPLAZAR

### 1. Agregar chromiumoxide

```toml
[dependencies]
chromiumoxide = { version = "0.7", features = ["tokio-runtime"] }
```

### 2. Browser manager

```rust
// Nuevo o reemplazar en pipeline/:
pub struct BrowserManager {
    browser: Option<Browser>,
}

impl BrowserManager {
    pub async fn init() -> Result<Self> {
        // Buscar Chrome/Edge instalado:
        // Windows: chrome.exe en Program Files o msedge.exe
        let browser = Browser::launch(
            BrowserConfig::builder()
                .chrome_executable(find_chrome()?)
                .arg("--headless=new")
                .arg("--disable-gpu")
                .arg("--no-sandbox")
                .build()?
        ).await?;
        Ok(Self { browser: Some(browser) })
    }
    
    pub async fn get_page_content(&self, url: &str) -> Result<PageContent> {
        let browser = self.browser.as_ref().ok_or("Browser not initialized")?;
        let page = browser.new_page(url).await?;
        
        // Esperar a que la página cargue (SPAs necesitan esto)
        page.wait_for_navigation().await?;
        tokio::time::sleep(Duration::from_secs(2)).await;  // Extra wait for JS
        
        let title = page.title().await?.unwrap_or_default();
        let text = page.evaluate("document.body.innerText").await?
            .into_value::<String>()?;
        let html = page.content().await?;
        
        // Opcional: screenshot para vision
        let screenshot = page.screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Jpeg)
                .quality(60)
                .build()
        ).await?;
        
        page.close().await?;
        
        Ok(PageContent { title, text, html, screenshot: Some(screenshot) })
    }
}

fn find_chrome() -> Result<PathBuf> {
    // Buscar en orden: Chrome, Edge, Chromium
    let paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ];
    for path in paths {
        if Path::new(path).exists() { return Ok(PathBuf::from(path)); }
    }
    Err("No Chrome/Edge browser found".into())
}
```

### 3. Integrar en el pipeline

```rust
// REEMPLAZAR el reqwest fetch con:
async fn browse_url(url: &str, task: &str, state: &AppState) -> Result<String> {
    let content = state.browser.get_page_content(url).await?;
    
    // Truncar para el LLM (max 6000 chars de texto visible)
    let truncated = &content.text[..6000.min(content.text.len())];
    
    let prompt = format!(
        "Page title: {}\nVisible text:\n{}\n\nTask: {}",
        content.title, truncated, task
    );
    
    gateway.call(&prompt, Tier::Standard).await
}
```

### 4. Fallback si no hay Chrome instalado

```rust
// Si chromiumoxide falla (no hay Chrome/Edge):
// → Fallback a reqwest (como antes)
// → Log: "Headless browser not available, using basic HTTP fetch"
// → El usuario verá: "Note: JavaScript-heavy sites may not render correctly"
```

---

## Verificación

1. ✅ "Buscá el clima de Montevideo en weather.com" → datos REALES (weather.com es SPA)
2. ✅ "¿Cuánto cuesta un iPhone en MercadoLibre?" → precios reales (MercadoLibre es SPA)
3. ✅ Sitio estático: funciona como antes
4. ✅ Sin Chrome instalado: fallback a reqwest con warning
5. ✅ El LLM recibe texto RENDERIZADO (JavaScript ejecutado) no HTML crudo
