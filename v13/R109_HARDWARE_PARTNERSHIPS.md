# FASE R109 — HARDWARE PARTNERSHIPS: AgentOS pre-instalado en PCs

**Objetivo:** Negociar con fabricantes de PCs (Dell, Lenovo, HP, Asus) para que AgentOS venga pre-instalado como "AI assistant" incluido. Como los PCs vienen con McAfee o Office trial, pero con AgentOS.

---

## Tareas

### 1. OEM installer package
```
El fabricante necesita:
- Silent installer (instala sin interacción del usuario)
- Configurable branding (puede decir "Dell AI Assistant powered by AgentOS")
- Pre-configured: tier free activado, provider setup al primer uso
- Telemetry opt-in en primer inicio
- Desinstalable por el usuario (no bloatware intrusivo)

Crear: agentos-oem-installer.exe con flags:
  --silent
  --brand-name "Dell AI Assistant"
  --brand-logo assets/dell-logo.png
  --brand-color "#0076CE"
  --pre-configure free
  --first-run-wizard true
```

### 2. Revenue model para OEM
```
Opción A: License fee per PC ($1-3 per unit, volume discount)
Opción B: Revenue share (% de upgrades Free→Pro de esos users)
Opción C: Hybrid ($0.50 per unit + 20% revenue share)

Pitch: "Your PCs come with AI built-in. Differentiator vs competition.
Users who activate upgrade to Pro at 5% rate = $X revenue for you."
```

### 3. OEM dashboard
```
Para el fabricante:
- Installs por modelo de PC
- Activation rate (cuántos abren AgentOS después de instalarlo)
- Upgrade rate (Free → Pro)
- Revenue generated
- Usage por región/país
```

### 4. Partnership pitch deck
```
Slide 1: "Every PC should have an AI assistant"
Slide 2: AgentOS stats (500K downloads, 50K WAU, 5% conversion)
Slide 3: "Pre-install = free differentiator for your brand"
Slide 4: Revenue model (show projected revenue per 100K units)
Slide 5: Technical: silent install, brandable, light (18MB)
Slide 6: Case study: "Acme Corp deployed 5,000 units → 12% activation"
Slide 7: Next steps: pilot program with 10K units
```

---

## Demo
1. OEM installer: `agentos-oem-installer.exe --silent --brand "Dell AI"` → installs silently
2. First boot: "Welcome to Dell AI Assistant" (Dell branding) → wizard → works
3. OEM dashboard: 10K installs, 1.2K activations, 60 upgrades → revenue chart
4. Partnership deck: 7 slides listos para presentar a Dell/Lenovo/HP
