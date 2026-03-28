# Architecture: AOS-004 — CLI Executor con soporte PTY

**Ticket:** AOS-004
**Rol:** Software Architect
**Input:** Especificación de producto (sección 3.3), AOS-001 Architecture
**Fecha:** Marzo 2026

---

## Módulos involucrados

| Componente | Archivo | Responsabilidad |
|-----------|---------|-----------------|
| CLIExecutor | `executor/cli.py` | Ejecuta comandos shell con PTY, captura output, maneja timeouts |
| SafetyGuard | `executor/safety.py` | Valida comandos contra blocklist antes de ejecución. Separado para testabilidad. |

### Dependencias

- **Hacia arriba:** `core/agent.py` llama a `CLIExecutor.execute()`
- **Config:** Lee `config/cli_safety.yaml` para reglas del sandbox
- **Store:** Los resultados se registran en `store/task_store.py` (tabla `execution_log`)

---

## Diagrama

```
      AgentCore
          │
          │ execute(command, timeout, cwd, env)
          ▼
┌──────────────────────────────────────────────┐
│              CLIExecutor                      │
│                                              │
│  1. SafetyGuard.validate(command)            │
│     → Si bloqueado: raise CommandBlockedError │
│                                              │
│  2. Sanitizar environment vars               │
│     → Remover keys listadas en blocked_env   │
│                                              │
│  3. Spawn proceso con PTY                    │
│     → asyncio.create_subprocess_exec          │
│     → stdout + stderr capturados             │
│                                              │
│  4. Monitorear con timeout                   │
│     → asyncio.wait_for(timeout)              │
│     → Si timeout: SIGTERM → espera 5s → SIGKILL│
│                                              │
│  5. Truncar output si excede max_output_bytes│
│                                              │
│  6. Retornar ExecutionResult                 │
│     → exit_code, stdout, stderr, duration    │
└──────────────────────────────────────────────┘
```

---

## Interfaces

### CLIExecutor

```python
class CLIExecutor:
    """Ejecuta comandos shell en la PC del usuario.

    Features:
    - PTY support para programas interactivos
    - Safety sandbox que bloquea comandos peligrosos
    - Timeout configurable con graceful shutdown
    - Output truncado para prevenir memory overflow
    - Environment limpio (sin API keys)
    """

    def __init__(self, safety_guard: SafetyGuard, default_timeout: int = 300) -> None:
        ...

    async def execute(
        self,
        command: str,
        timeout: int | None = None,   # None = usa default
        cwd: str | None = None,       # None = home dir
        env: dict[str, str] | None = None,  # vars adicionales
    ) -> ExecutionResult:
        """Ejecuta un comando y retorna el resultado.

        Raises:
            CommandBlockedError: Comando rechazado por safety sandbox.
            CommandTimeoutError: Comando excedió el timeout.
        """
        ...
```

### SafetyGuard

```python
class SafetyGuard:
    """Valida comandos contra reglas de seguridad.

    Carga reglas de config/cli_safety.yaml.
    Separado del executor para testabilidad independiente.
    """

    def __init__(self, config_path: Path) -> None:
        ...

    def validate(self, command: str) -> tuple[bool, str]:
        """Valida si el comando es seguro.

        Returns:
            (is_safe, reason). Si is_safe=False, reason explica por qué.
        """
        ...

    def sanitize_env(self, env: dict[str, str] | None) -> dict[str, str]:
        """Remueve variables de entorno sensibles.

        Hereda el environment actual MENOS las vars bloqueadas en config.
        Si env extra es proporcionado, se agrega al environment limpio.
        """
        ...
```

---

## Proceso de terminación por timeout

```
1. Timeout se alcanza
2. Enviar SIGTERM al proceso
3. Esperar 5 segundos (grace period)
4. Si el proceso sigue vivo: enviar SIGKILL
5. Capturar output parcial generado antes del timeout
6. Raise CommandTimeoutError con el output parcial adjunto
```

---

## Truncado de output

Para prevenir que un comando con output infinito (ej: `yes`, `cat /dev/urandom`) llene la memoria:

- `max_output_bytes` se configura en `cli_safety.yaml` (default: 1 MB)
- stdout y stderr se leen en chunks
- Si el total excede el límite, se trunca y se agrega `"\n... [output truncated at 1MB]"`
- El `ExecutionResult` incluye el output truncado

---

## Design patterns

| Patrón | Aplicación | Justificación |
|--------|-----------|---------------|
| **Strategy** | SafetyGuard | Permite intercambiar reglas (YAML hoy, policy engine futuro) |
| **Dependency Injection** | CLIExecutor recibe SafetyGuard | Facilita testing: mock del guard para probar el executor |
| **Template Method** | Flujo execute: validate → spawn → monitor → collect | Pasos fijos, implementación variable |

---

## ADR: asyncio.create_subprocess_shell vs subprocess

- **Status:** Accepted
- **Context:** Necesitamos ejecutar comandos async con captura de output.
- **Decision:** Usar `asyncio.create_subprocess_shell()` para comandos simples. Para PTY interactivo, usar `asyncio.create_subprocess_exec` con `pty` fork.
- **Consequences:** Async nativo, compatible con el event loop del agente. PTY support requiere `os.openpty()` en Unix.

## ADR: SafetyGuard separado del Executor

- **Status:** Accepted
- **Context:** Las reglas de seguridad son complejas y necesitan tests propios.
- **Decision:** SafetyGuard es una clase independiente inyectada al Executor.
- **Consequences:** Se puede testear el guard sin ejecutar comandos reales. Se puede testear el executor con un guard permisivo.

---

## Constraints

- El executor NUNCA ejecuta un comando sin pasar por SafetyGuard primero.
- El environment del proceso hijo NUNCA contiene API keys.
- El output NUNCA excede `max_output_bytes` en memoria.
- Todos los procesos hijos se terminan al shutdown del agente (no dejar orphans).
- El executor funciona en Linux y Windows (Phase 3). v1 solo Linux.
