# Implementation Spec: AOS-008 — Integración con bot de Telegram

**Ticket:** AOS-008
**Rol:** Backend Developer (con notas de seguridad)
**Input:** Especificación de producto (sección 3.1), AOS-001 Architecture, AOS-009 Architecture (AgentCore interface)
**Fecha:** Marzo 2026

---

## Objetivo

Implementar un bot de Telegram que recibe mensajes del usuario, los pasa al AgentCore para procesamiento, y devuelve los resultados formateados. Es la primera interfaz de comunicación del agente.

---

## Arquitectura del messaging layer

El bot de Telegram implementa una interfaz genérica `BaseMessagingAdapter` para que futuros canales (WhatsApp, Discord) sean plug-and-play.

```
Usuario Telegram
    │
    │ mensaje de texto
    ▼
┌────────────────────────────────────────────┐
│         TelegramAdapter                     │
│                                            │
│  1. Recibe Update de Telegram API          │
│  2. Convierte a TaskInput                  │
│  3. Envía "typing..." indicator            │
│  4. Llama a AgentCore.process(task_input)  │
│  5. Formatea TaskResult para Telegram      │
│  6. Envía respuesta (split si > 4096 chars)│
└────────────────────────────────────────────┘
```

---

## Interfaces

### BaseMessagingAdapter (interfaz genérica)

```python
from abc import ABC, abstractmethod
from typing import Callable, Awaitable

class BaseMessagingAdapter(ABC):
    """Interfaz base para todos los canales de mensajería.

    Cada adaptador (Telegram, WhatsApp, Discord) implementa esta interfaz.
    El AgentCore no sabe qué canal está usando — solo recibe TaskInputs.
    """

    def __init__(self, on_message: Callable[[TaskInput], Awaitable[TaskResult]]) -> None:
        """
        Args:
            on_message: Callback async que procesa un TaskInput y retorna TaskResult.
                        Típicamente es AgentCore.process().
        """
        ...

    @abstractmethod
    async def start(self) -> None:
        """Inicia la escucha de mensajes."""
        ...

    @abstractmethod
    async def stop(self) -> None:
        """Para la escucha y limpia recursos."""
        ...

    @abstractmethod
    async def send_message(self, chat_id: str, text: str) -> None:
        """Envía un mensaje de texto al usuario."""
        ...
```

### TelegramAdapter

```python
class TelegramAdapter(BaseMessagingAdapter):
    """Adaptador de Telegram usando python-telegram-bot.

    Uso:
        adapter = TelegramAdapter(
            token=settings.telegram_bot_token,
            on_message=agent_core.process,
        )
        await adapter.start()
    """

    def __init__(self, token: str, on_message: Callable) -> None: ...

    async def start(self) -> None:
        """Inicia el bot con polling. No bloquea — corre en el event loop."""
        ...

    async def stop(self) -> None:
        """Para el polling y cierra conexiones."""
        ...

    async def send_message(self, chat_id: str, text: str) -> None:
        """Envía un mensaje formateado con Markdown de Telegram.

        Si el texto excede 4096 chars, lo splitea en múltiples mensajes.
        """
        ...
```

---

## Comandos del bot

| Comando | Descripción | Respuesta |
|---------|-------------|-----------|
| `/start` | Primer mensaje del usuario | Bienvenida con instrucciones básicas de uso |
| `/status` | Estado del agente | Providers disponibles, tareas hoy, costo de sesión |
| `/history` | Últimas tareas | Últimas 5 tareas con: input truncado, estado, costo |
| `/help` | Ayuda | Lista de comandos disponibles |
| (mensaje de texto) | Tarea para el agente | Procesa con AgentCore, devuelve resultado |

---

## Formato de respuestas

### Respuesta exitosa
```
✅ *Done*

[output del agente]

_Model: gpt-4o-mini · Cost: $0.0003 · 1.2s_
```

### Respuesta con error
```
❌ *Error*

[mensaje de error legible]

_If this keeps happening, check /status for provider health._
```

### Mensaje de bienvenida (/start)
```
👋 *Welcome to AgentOS!*

I'm your AI agent running on your PC. Send me any message and I'll help you.

*What I can do:*
• Answer questions using AI
• Run commands on your PC
• Execute playbook automations

*Commands:*
/status — Check agent health
/history — Recent tasks
/help — Show this help

_Send me a message to get started!_
```

---

## Comportamiento del indicador "typing..."

1. Al recibir un mensaje, enviar `ChatAction.TYPING` inmediatamente.
2. Si el procesamiento tarda > 5 segundos, re-enviar `TYPING` cada 5 segundos (Telegram lo cancela automáticamente después de 5s).
3. Parar de enviar typing cuando la respuesta está lista.

---

## Split de mensajes largos

Telegram tiene un límite de 4096 caracteres por mensaje.

```python
def split_message(text: str, max_length: int = 4096) -> list[str]:
    """Splitea un mensaje largo en partes.

    Reglas:
    1. Preferir split en newlines (no cortar mid-párrafo).
    2. Si no hay newlines, split en espacios.
    3. Si una sola palabra excede el límite... truncar (muy raro).
    4. Agregar "... (1/N)" al final de cada parte excepto la última.
    """
```

---

## Seguridad

- **SEC-050**: El token de Telegram se lee de `Settings.telegram_bot_token`. NUNCA se loguea.
- **SEC-051**: El token se pasa una vez al constructor de python-telegram-bot y nunca se expone después.
- **SEC-052**: Los mensajes del usuario NO se loguean en nivel INFO (solo el task_id y resultado status). En DEBUG se puede loguear el texto truncado a 100 chars.
- **SEC-053**: El bot NO responde a mensajes de usuarios no autorizados si se configura una allowlist de user_ids (configuración futura, no en v1 — en v1 cualquiera puede hablarle al bot).

---

## Error handling

| Escenario | Comportamiento |
|-----------|---------------|
| Token de Telegram inválido | Log error al iniciar, no crashear el agente. Deshabilitar el adaptador. |
| Network error durante polling | python-telegram-bot tiene retry automático. Log warning. |
| AgentCore.process() falla | Enviar mensaje de error al usuario. No crashear el bot. |
| Mensaje vacío del usuario | Responder: "Send me a message and I'll help!" |
| Telegram API rate limit | Backoff automático de python-telegram-bot. |

---

## Test cases

| # | Test | Expected |
|---|------|----------|
| 1 | /start command | Envía mensaje de bienvenida |
| 2 | /status command | Muestra providers y stats |
| 3 | /history command (sin tareas) | "No tasks yet" |
| 4 | /history command (con tareas) | Lista las últimas 5 |
| 5 | Mensaje de texto normal | Llama on_message callback, envía respuesta |
| 6 | Respuesta > 4096 chars | Se splitea en múltiples mensajes |
| 7 | AgentCore falla | Envía mensaje de error, bot sigue funcionando |
| 8 | Mensaje vacío | Respuesta helpful, no crash |
| 9 | Múltiples mensajes rápidos | Se procesan en orden (queue), no se pierden |
| 10 | Token inválido | Log error, adaptador se deshabilita, agente sigue |

**Nota:** Todos los tests usan mocks del API de Telegram. No dependen de un token real.
