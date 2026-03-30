# FASE R45 — WHITE-LABEL / OEM: Empresas usan el engine con su marca

**Objetivo:** Una empresa puede tomar el engine de AgentOS, ponerle su logo, sus colores, sus specialists, y deployarlo como "su" producto de automatización interna. Licencia OEM con revenue recurrente.

---

## Tareas

### 1. Branding configurable

```json
// config/branding.json — reemplazable por el OEM
{
  "app_name": "AutomatePro",           // en vez de "AgentOS"
  "company": "Acme Corp",
  "logo_path": "assets/acme-logo.png",
  "icon_path": "assets/acme-icon.ico",
  "primary_color": "#FF6B00",          // en vez de cyan
  "tagline": "Your AI assistant by Acme",
  "support_url": "https://support.acme.com",
  "docs_url": "https://docs.acme.com",
  "hide_agentos_branding": true,       // ocultar "Powered by AgentOS"
  "custom_specialists": "specialists/acme/"  // specialists propios
}
```

### 2. Frontend lee branding dinámicamente

```typescript
// En App.tsx:
const branding = await invoke("get_branding");

// CSS variables se actualizan:
document.documentElement.style.setProperty('--brand-color', branding.primary_color);

// Logo, nombre, tagline se inyectan dinámicamente
// Sidebar: "AutomatePro" en vez de "AgentOS"
// About page: "AutomatePro by Acme Corp"
```

### 3. Build script con branding custom

```bash
# scripts/build-oem.sh
# Input: branding.json + assets/
# Output: AutomatePro-Setup.exe con logo, colores, y nombre del OEM

# 1. Copiar branding.json a config/
# 2. Copiar assets (logo, icon) a resources/
# 3. Actualizar tauri.conf.json con el nombre y el ícono
# 4. cargo tauri build
# 5. El installer dice "AutomatePro" no "AgentOS"
```

### 4. Custom specialists para OEM

```
// El OEM puede agregar specialists específicos a su industria:
// Ej: una firma de contabilidad agrega:
// - Tax Specialist (Uruguay)
// - Payroll Processor
// - Invoice Reconciler
// - BPS Report Generator

// Cada specialist es un JSON con system prompt + keywords
// Se cargan desde el directorio de branding
```

### 5. Licencia OEM

```markdown
# OEM License Agreement

Tiers:
- Startup:    $500/month — up to 50 users, 1 custom specialist
- Business: $2,000/month — up to 500 users, unlimited specialists
- Enterprise: custom    — unlimited, dedicated support, source access

Includes:
- White-label build (your brand, your colors, your icon)
- Custom specialists support
- Priority bug fixes
- Quarterly feature alignment calls

Does NOT include:
- Source code access (except Enterprise)
- Resale rights
- Modification of core engine
```

---

## Demo

1. Cambiar branding.json → rebuild → app dice "AutomatePro" con logo naranja de Acme
2. Custom specialist "Tax Analyst Uruguay" → responde preguntas de DGI/BPS
3. Installer dice "AutomatePro-Setup.exe" — cero mención de AgentOS
4. About page: "AutomatePro v1.0 by Acme Corp. Powered by AgentOS Engine."
