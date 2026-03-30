# FASE R103 — IOT CONTROLLER: El agente controla tu casa/oficina

**Objetivo:** "Apagá las luces de la oficina" → el agente controla dispositivos smart vía APIs de IoT (Home Assistant, Philips Hue, TP-Link, Tuya, Google Home, Alexa).

---

## Tareas

### 1. IoT hub integration
```rust
pub trait IoTProvider: Send + Sync {
    async fn list_devices(&self) -> Result<Vec<IoTDevice>>;
    async fn get_state(&self, device_id: &str) -> Result<DeviceState>;
    async fn set_state(&self, device_id: &str, state: DeviceState) -> Result<()>;
}

// Providers:
struct HomeAssistantProvider { base_url: String, token: String }
struct PhilipsHueProvider { bridge_ip: String, user: String }
struct TuyaProvider { access_id: String, access_secret: String }
```

### 2. Natural language IoT control
```
"Apagá las luces" → find lights → set_state(off)
"Poné el aire en 22 grados" → find AC → set_state(temp: 22)
"¿Está cerrada la puerta?" → find lock → get_state() → "Yes, locked"
"Cuando llegue a casa, prendé las luces" → geofence trigger + IoT action
```

### 3. IoT dashboard widget
```
SMART HOME                              [Refresh]
┌──────────┐ ┌──────────┐ ┌──────────┐
│ 💡 Lights│ │ 🌡 AC    │ │ 🔒 Door  │
│   ON     │ │  22°C    │ │  Locked  │
│ [Toggle] │ │ [± Temp] │ │ [Unlock] │
└──────────┘ └──────────┘ └──────────┘
```

### 4. IoT + Triggers (R18)
```
"Si la temperatura sube de 30°C, prendé el aire"
"Si nadie está en la oficina a las 8pm, apagá todo"
"Cuando empiece una reunión (calendar), poné modo 'no molestar' en las luces"
```

---

## Demo
1. "Apagá las luces" → luces se apagan (Home Assistant)
2. "¿Cuánto marca el termostato?" → "23.5°C"
3. Trigger: temperatura > 28 → AC se prende automáticamente
4. Dashboard widget muestra estado de 5 dispositivos en tiempo real
