# Platform Support Matrix

Estado honesto de plataforma para AgentOS en esta rama:

## Cross-platform reales

- core chat / gateway
- billing y limits
- calendar / gmail
- memory + semantic retrieval
- swarm
- testing runner
- vertical packs `accounting` y `legal`

## Limitadas por plataforma

- `screen_capture`
  - Windows: real
  - macOS/Linux: deshabilitado por ahora
- `input_control`
  - Windows: real
  - macOS/Linux: deshabilitado por ahora
- `voice_tts`
  - depende del runtime del SO; no se marca como garantizado en macOS/Linux

## Windows-only hoy

- `pc_control`
- `ui_automation`
- `windows_ocr`
- algunas rutas de lectura Office basadas en PowerShell / COM
- ventanas secundarias de widgets con validación operativa solo en Windows

## Validación de build

- Windows: validado localmente durante esta ronda con `cargo test`.
- macOS y Linux: preparados para validación por CI en `.github/workflows/cross-platform-build.yml`.

## Cierre honesto

Esta rama no promete paridad funcional completa entre SO. Declara explícitamente qué funciona en todos, qué queda limitado y qué sigue siendo Windows-only.
