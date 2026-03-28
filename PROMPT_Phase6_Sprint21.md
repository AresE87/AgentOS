# PROMPT PARA CLAUDE CODE — PHASE 6, SPRINT 21

## Documentos: Phase6_Sprint_Plan.md + AOS-051_060_Architecture.md (PARTE 3) + código Phase 1-5 + Sprint 19-20

## Prompt:

Sos el DevOps Engineer + Backend Developer de AgentOS. Phase 6, Sprint 21. Hacés que AgentOS funcione en macOS, Linux, y abstraés las diferencias de plataforma.

### Ticket 1: AOS-055 — macOS Build
- Actualizar build scripts para macOS (cargo tauri build genera .dmg)
- Python embeddable para macOS (universal binary Intel + Apple Silicon)
- Menú bar (equivalente a system tray en macOS)
- Code signing y notarización (documentar pasos)

### Ticket 2: AOS-056 — Linux Build
- Actualizar build scripts para Linux (cargo tauri build genera AppImage/.deb)
- Python bundled para Linux x86_64
- System tray para GNOME y KDE
- Desktop entry (.desktop file)

### Ticket 3: AOS-057 — Platform Abstraction
- `agentos/utils/platform.py` → PlatformInfo
- Shell: cmd.exe (Windows) vs /bin/bash (Unix)
- Keychain backend por OS
- Blocklist de comandos OS-específicos (del /f /s en Windows)
- Screen control: pyautogui en X11, ydotool fallback en Wayland
- Tests platform-specific con @pytest.mark.skipif
