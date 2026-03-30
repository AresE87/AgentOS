# FASE R26 — CROSS-PLATFORM: macOS y Linux

**Objetivo:** AgentOS se instala y funciona en macOS (.dmg) y Linux (AppImage/.deb). Misma funcionalidad que Windows excepto las APIs nativas del OS (screen capture, input simulation).

---

## Tareas

### 1. Platform abstraction layer

```rust
// Nuevo: src-tauri/src/platform.rs

pub trait PlatformOps {
    fn capture_screen() -> Result<Screenshot>;
    fn click(x: i32, y: i32) -> Result<()>;
    fn type_text(text: &str) -> Result<()>;
    fn key_combo(keys: &[&str]) -> Result<()>;
    fn run_command(cmd: &str) -> Result<CommandOutput>;
    fn shell_name() -> &'static str;
    fn blocked_commands() -> Vec<String>;
}

#[cfg(target_os = "windows")]
mod windows_ops; // GDI, SendInput, PowerShell

#[cfg(target_os = "macos")]  
mod macos_ops;   // CoreGraphics, CGEvent, zsh

#[cfg(target_os = "linux")]
mod linux_ops;   // X11/Wayland, xdotool/ydotool, bash
```

### 2. macOS implementation

```rust
// macos_ops.rs:
// Screen capture: CoreGraphics CGWindowListCreateImage
// Input: CGEventCreateKeyboardEvent, CGEventCreateMouseEvent
// CLI: /bin/zsh -c "command"
// Blocked: rm -rf /, sudo rm, etc.
```

Build:
```bash
# En macOS:
cargo tauri build
# Genera: .dmg y .app bundle
```

### 3. Linux implementation

```rust
// linux_ops.rs:
// Screen capture: X11 XGetImage o PipeWire (Wayland)
// Input: xdotool (X11) o ydotool (Wayland)
// CLI: /bin/bash -c "command"
// Keychain: Secret Service (libsecret / GNOME Keyring)
```

Build:
```bash
# En Linux:
cargo tauri build
# Genera: AppImage y .deb
```

### 4. CI/CD para 3 plataformas

```yaml
# .github/workflows/build.yml
jobs:
  build:
    strategy:
      matrix:
        os: [windows-latest, macos-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo tauri build
      - uses: actions/upload-artifact@v4
```

### 5. Safety guard por plataforma

```rust
// Windows: block "format C:", "del /f /s", "shutdown /s"
// macOS: block "rm -rf /", "diskutil eraseDisk", "shutdown -h"  
// Linux: block "rm -rf /", "dd if=/dev/zero", "mkfs.", "shutdown"
```

---

## Demo

1. Instalar en macOS → .dmg → drag to Applications → abre → wizard → chat funciona
2. Instalar en Linux (Ubuntu) → .deb → abre → wizard → chat funciona
3. Vision mode en macOS: abre Calculator → hace una suma
4. Vision mode en Linux: abre terminal → ejecuta comando
