# FASE R30 — ECOSYSTEM: Docs, seed content, onboarding, comunidad

**Objetivo:** Todo lo que rodea al producto para que otras personas lo adopten: documentación completa, 30 playbooks de calidad, onboarding optimizado, y las bases de la comunidad.

---

## Tareas

### 1. Docs site

Crear documentación en Markdown (hostear en GitHub Pages o similar):

```
docs/
├── getting-started.md      — Install → setup → first task (5 min)
├── features/
│   ├── chat.md             — Cómo usar el chat
│   ├── vision.md           — Cómo funciona el vision mode
│   ├── playbooks.md        — Crear, grabar, compartir playbooks
│   ├── board.md            — Entender el task board
│   ├── mesh.md             — Conectar múltiples PCs
│   ├── triggers.md         — Automatizar tareas
│   └── analytics.md        — Entender las métricas
├── api/
│   ├── reference.md        — Todos los endpoints con examples
│   ├── sdk.md              — Python SDK quickstart
│   └── webhooks.md         — Configurar webhooks
├── guides/
│   ├── create-playbook.md  — Tutorial paso a paso
│   ├── publish-playbook.md — Publicar en marketplace
│   ├── enterprise.md       — SSO + audit + multi-tenant
│   └── troubleshooting.md  — Problemas comunes
└── contributing.md         — Cómo contribuir playbooks
```

### 2. 30 playbooks seed (agregar 20 más a los 10 de R22)

```
Los 10 existentes (R22): system-monitor, git-assistant, file-organizer,
code-reviewer, daily-standup, log-analyzer, dependency-checker,
markdown-to-html, disk-cleanup, network-check

20 nuevos:
11. email-drafter — Redacta emails basado en contexto
12. invoice-reader — Lee facturas PDF y extrae datos
13. backup-manager — Backup de directorios configurables
14. api-tester — Ejecuta requests HTTP y valida responses
15. csv-analyzer — Analiza un CSV y da estadísticas
16. screenshot-annotator — Toma screenshot y anota elementos
17. password-auditor — Busca secrets en archivos de config
18. docker-helper — Gestión básica de containers
19. database-query — Ejecuta queries SQLite/PostgreSQL
20. blog-post-writer — Genera borradores de blog posts
21. meeting-notes — Resume texto de reuniones
22. seo-checker — Analiza una URL para SEO básico
23. social-media-poster — Genera posts para redes
24. project-scaffolder — Crea estructura de proyecto (Python/Node/Rust)
25. regex-helper — Genera y testea expresiones regulares
26. json-formatter — Formatea y valida JSON/YAML
27. port-scanner — Escanea puertos abiertos en un host
28. ssl-checker — Verifica certificados SSL de un dominio
29. cron-explainer — Explica expresiones cron en lenguaje humano
30. changelog-generator — Genera changelog desde git log
```

Cada uno: playbook.json + metadata.json + README.md + empaquetado como .aosp.

### 3. Onboarding optimizado

```
Primera ejecución mejorada:
1. Wizard (3 pasos como R3)
2. Después del wizard: "Quick Tour" interactivo
   - Highlight Chat → "Send your first message here"
   - Highlight Playbooks → "Browse pre-built automations"
   - Highlight Board → "Watch your agents work in real-time"
3. 3 tareas sugeridas para probar:
   - "Check my disk space" (simple, funciona seguro)
   - "Review this Python code: [example]" (muestra specialist)
   - "Organize my Downloads folder" (muestra acción real en el filesystem)
```

### 4. Landing page v2

```html
<!-- agentos.app -->
Hero: "Your AI team, running on your PC"
[Video demo: 30s de Chat → Board → Result]

3 features destacadas:
1. "Tell it what to do" — chat screenshot
2. "Watch it work" — board screenshot
3. "It learns your workflow" — playbook screenshot

Stats: "18MB installer · 40 specialists · 30 playbooks · 3 platforms"

[Download for Windows] [Download for macOS] [Download for Linux]

Pricing: Free / Pro $29/mo / Team $79/mo

"Built for acquisition. Open protocol, closed engine."
```

### 5. Creator program (bases)

```
¿Tenés expertise en un dominio? Creá un playbook y ganá dinero.

1. Creá un playbook con el recorder
2. Empaquetalo como .aosp
3. Publicalo en el marketplace
4. Poné el precio que quieras
5. Ganá el 70% de cada venta

[Become a Creator]
```

Documentación de cómo crear y publicar un playbook premium.

---

## Demo

1. Docs site navegable con todos los guides
2. Marketplace tiene 30 playbooks de calidad
3. Nuevo usuario: instala → wizard → quick tour → primera tarea en < 3 minutos
4. Landing page con video demo y botones de descarga
5. Creator docs explican cómo publicar un playbook
