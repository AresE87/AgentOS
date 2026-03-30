# FASE R22 — MARKETPLACE: Compartir y descubrir playbooks

**Objetivo:** Los usuarios pueden navegar un catálogo de playbooks, instalarlos con un click, publicar los suyos, y dejar reviews. Todo funcional, probado con 10+ playbooks reales.

---

## Arquitectura

Para v1, el marketplace es LOCAL — un directorio de paquetes .aosp que se distribuye con la app y se actualiza vía GitHub. No hay servidor central todavía.

```
marketplace/
├── index.json          ← catálogo con metadata de todos los playbooks
├── packages/
│   ├── system-monitor-1.0.0.aosp
│   ├── git-assistant-1.0.0.aosp
│   └── ...
```

Un .aosp es un ZIP renombrado:
```
my-playbook-1.0.0.aosp (ZIP)
├── playbook.json       ← instrucciones del agente
├── metadata.json       ← nombre, autor, tags, precio, versión
├── README.md           ← descripción para el listing
├── icon.png            ← thumbnail (256x256)
└── steps/              ← screenshots del playbook (si visual)
```

---

## Tareas

### 1. Formato .aosp y PackageManager

```rust
// Nuevo: src-tauri/src/marketplace/packaging.rs

pub struct PackageManager;

impl PackageManager {
    /// Empaqueta un directorio de playbook en .aosp
    pub fn pack(playbook_dir: &Path, output: &Path) -> Result<PathBuf>;
    
    /// Valida un .aosp (estructura correcta, metadata válida)
    pub fn validate(aosp_path: &Path) -> Result<Vec<String>>;  // errores
    
    /// Instala un .aosp en el directorio de playbooks
    pub fn install(aosp_path: &Path, playbooks_dir: &Path) -> Result<String>;
    
    /// Desinstala un playbook
    pub fn uninstall(name: &str, playbooks_dir: &Path) -> Result<()>;
}
```

### 2. Catálogo y búsqueda

```rust
// Nuevo: src-tauri/src/marketplace/catalog.rs

pub struct MarketplaceCatalog {
    listings: Vec<PlaybookListing>,
}

pub struct PlaybookListing {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub category: String,      // dev, business, productivity, data, marketing, sysadmin
    pub tags: Vec<String>,
    pub price: f64,            // 0.0 = free
    pub rating: f64,           // 0.0-5.0
    pub review_count: u32,
    pub download_count: u32,
    pub permissions: Vec<String>,  // cli, screen, files, network
    pub icon_path: Option<String>,
}

impl MarketplaceCatalog {
    pub fn load(index_path: &Path) -> Result<Self>;
    pub fn search(&self, query: &str) -> Vec<&PlaybookListing>;
    pub fn filter(&self, category: Option<&str>, price: Option<&str>) -> Vec<&PlaybookListing>;
    pub fn get(&self, id: &str) -> Option<&PlaybookListing>;
}
```

### 3. Reviews en SQLite

```sql
CREATE TABLE IF NOT EXISTS marketplace_reviews (
    id          TEXT PRIMARY KEY,
    playbook_id TEXT NOT NULL,
    rating      INTEGER NOT NULL,  -- 1-5
    comment     TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS marketplace_installs (
    id          TEXT PRIMARY KEY,
    playbook_id TEXT NOT NULL,
    version     TEXT NOT NULL,
    installed_at TEXT NOT NULL
);
```

### 4. IPC commands

```rust
#[tauri::command] async fn marketplace_list(category: Option<String>, price: Option<String>, sort: Option<String>) -> Result<Vec<PlaybookListing>, String>
#[tauri::command] async fn marketplace_search(query: String) -> Result<Vec<PlaybookListing>, String>
#[tauri::command] async fn marketplace_detail(id: String) -> Result<PlaybookDetail, String>
#[tauri::command] async fn marketplace_install(id: String) -> Result<(), String>
#[tauri::command] async fn marketplace_uninstall(id: String) -> Result<(), String>
#[tauri::command] async fn marketplace_review(id: String, rating: u32, comment: String) -> Result<(), String>
#[tauri::command] async fn marketplace_get_reviews(id: String) -> Result<Vec<Review>, String>
#[tauri::command] async fn marketplace_publish(playbook_dir: String) -> Result<String, String>
```

### 5. Frontend: Marketplace en Playbooks page

Reemplazar el placeholder actual con:

```
MARKETPLACE                    [Search: ________] [Category ▾] [Sort ▾]

┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ [icon]       │ │ [icon]       │ │ [icon]       │ │ [icon]       │
│ System       │ │ Git          │ │ File         │ │ Code         │
│ Monitor      │ │ Assistant    │ │ Organizer    │ │ Reviewer     │
│ ★★★★☆ (12)  │ │ ★★★★★ (8)   │ │ ★★★★☆ (5)   │ │ ★★★★★ (3)   │
│ FREE         │ │ FREE         │ │ FREE         │ │ FREE         │
│ [Install]    │ │ [Install]    │ │ [Installed ✓]│ │ [Install]    │
└──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘
```

Click en un playbook → Detail page:
```
← Back

📘 System Monitor                              [Install]
by AgentOS Team · v1.0.0 · FREE

Monitors your PC health: CPU usage, memory, disk space, network.
Runs a comprehensive system check and reports any issues.

Permissions: [CLI] [FILES]
Tier: 1 (Cheap)

REVIEWS (12)
★★★★★ — "Works perfectly, runs every morning"      3 days ago
★★★★☆ — "Good but could check GPU too"              1 week ago
★★★☆☆ — "Sometimes slow on large disks"             2 weeks ago

[Write a review]
```

### 6. Crear 10 playbooks seed

Crear playbooks REALES (no mocks) que funcionen con el engine actual:

1. **system-monitor** — Checkea CPU, RAM, disco, red vía PowerShell
2. **git-assistant** — Git status, diff, log formateado
3. **file-organizer** — Organiza Downloads/ por extensión
4. **code-reviewer** — Revisa código y da feedback detallado
5. **daily-standup** — Resumen de actividad del día
6. **log-analyzer** — Analiza un archivo de log por errores
7. **dependency-checker** — Verifica deps desactualizadas (npm/pip)
8. **markdown-to-html** — Convierte .md a .html formateado
9. **disk-cleanup** — Encuentra archivos grandes y temp files
10. **network-check** — Ping, DNS, traceroute, velocidad

Cada uno empaquetado como .aosp, incluido en el catálogo.

---

## Demo

1. Abrir Playbooks → Marketplace → ver grid de 10 playbooks
2. Click "System Monitor" → ver detalle con descripción y reviews
3. Click "Install" → se instala → aparece en "Installed"
4. Click "Activate" → ejecutar "check my system" → funciona con el playbook
5. Escribir review → aparece en la lista de reviews
