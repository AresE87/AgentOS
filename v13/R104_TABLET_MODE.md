# FASE R104 — TABLET MODE: UI para tablets con stylus

**Objetivo:** En tablets (iPad, Surface, Android tablets), la UI se adapta: más espacio para el workflow builder (R71), dibujar anotaciones en screenshots, firmar documentos, y un modo "whiteboard" donde el agente colabora visualmente.

---

## Tareas

### 1. Responsive layout para tablets (768-1200px)
- Sidebar colapsable por defecto (más espacio para contenido)
- Cards en grid de 2-3 columnas
- Touch targets: 48px mínimo
- Swipe gestures: swipe left to archive task, swipe right to retry

### 2. Stylus drawing en screenshots
- En playbook recorder: anotar screenshots con el stylus
- Círculos, flechas, texto handwritten
- "Marca dónde hay que hacer click" → el agente usa la anotación como guía

### 3. Workflow builder touch-optimized
- Nodos del visual builder (R71) draggable con dedo/stylus
- Pinch-to-zoom en canvas grande
- Double-tap node to edit
- Long-press for context menu

### 4. Document signing
- El agente genera un documento (template R58)
- El usuario firma con el stylus
- Firma se embebe en el PDF
- "Firmá este contrato" → agente genera → usuario firma → agente envía

### 5. Whiteboard mode
```
Pantalla completa: canvas blanco infinito
El usuario dibuja/escribe con stylus
El agente ve lo dibujado (vision) y responde:
- Diagrama → "I see a flowchart with 3 steps..."
- Ecuación → "The answer is 42"
- Wireframe → "I can create this UI for you"
```

---

## Demo
1. iPad: workflow builder funciona con finger/stylus → drag nodes, pinch to zoom
2. Anotar screenshot con círculo rojo → "click aquí" → agente entiende
3. Whiteboard: dibujar diagrama → agente describe y sugiere mejoras
4. Firmar PDF con stylus → firma embebida → enviar por email
