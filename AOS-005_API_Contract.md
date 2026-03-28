# API Contract: AOS-005 — Context Folder Protocol — Parser básico

**Ticket:** AOS-005
**Rol:** API Designer
**Input:** Especificación de producto (sección 4.1), AOS-001 Architecture
**Fecha:** Marzo 2026

---

## Objetivo

Definir el formato exacto de los archivos del Context Folder Protocol (v1) y la interfaz del parser que los lee y valida.

v1 soporta solo los dos archivos requeridos: `playbook.md` y `config.yaml`.
v2+ agregará: `steps/`, `templates/`, `triggers.yaml`, `metadata.yaml`.

---

## Formato de playbook.md

El archivo `playbook.md` es Markdown estándar con una estructura esperada:

```markdown
# [Nombre del playbook]

[Descripción opcional en texto libre — el primer párrafo después del título.]

## Behavior | Comportamiento

[Instrucciones en lenguaje natural sobre qué hacer y cómo actuar.]

## Constraints | Restricciones

[Reglas que el agente debe seguir. Formato libre o lista.]

## [Secciones opcionales adicionales]

[Cualquier sección adicional con ## es capturada como contexto extra.]
```

### Reglas de parsing

1. El **título** se extrae del primer `# heading` (H1). Es obligatorio.
2. La **descripción** es todo el texto entre el H1 y el primer H2. Opcional (puede estar vacío).
3. Las **instrucciones** son el contenido completo del archivo (incluido título y descripción) — se pasa al LLM como system context.
4. El parser NO interpreta el Markdown semánticamente — lo trata como texto plano para el LLM. La estructura es para legibilidad humana.
5. Si el archivo no tiene un H1, se usa el nombre del directorio como título.

---

## Formato de config.yaml

```yaml
# Requeridos
name: "string"              # Nombre del playbook (debe coincidir con H1 de playbook.md)
                             
# Opcionales con defaults
description: "string"        # Default: "" — Descripción corta para listings
tier: 1                      # Default: 1 — LLM tier (1=cheap, 2=standard, 3=premium)
timeout: 300                 # Default: 300 — Máximo segundos por acción
permissions:                 # Default: [] — Permisos requeridos
  - cli                      # Puede ejecutar comandos shell
  - screen                   # Puede controlar la pantalla (Phase 2)
  - files                    # Puede leer/escribir archivos
  - network                  # Puede hacer requests HTTP

allowed_commands: []         # Default: [] — Lista vacía = todos los no-bloqueados
blocked_commands: []         # Default: [] — Comandos adicionales a bloquear para este playbook
```

### Validación

| Campo | Tipo | Validación |
|-------|------|-----------|
| `name` | str | Requerido. No vacío. Max 100 chars. |
| `description` | str | Opcional. Max 500 chars. |
| `tier` | int | Debe ser 1, 2, o 3. |
| `timeout` | int | Debe ser > 0 y <= 600 (máximo del sistema). |
| `permissions` | list[str] | Cada item debe ser uno de: "cli", "screen", "files", "network". |
| `allowed_commands` | list[str] | Lista de strings. Sin validación de contenido en v1. |
| `blocked_commands` | list[str] | Lista de strings. Sin validación de contenido en v1. |

---

## Interface: ContextFolderParser

```python
class ContextFolderParser:
    """Parser de Context Folders (playbooks).

    Lee un directorio, valida que contenga los archivos requeridos,
    parsea playbook.md y config.yaml, y retorna un ContextFolder tipado.
    """

    async def parse(self, path: Path) -> ContextFolder:
        """Parsea un Context Folder.

        Args:
            path: Ruta al directorio del playbook.

        Returns:
            ContextFolder con toda la información parseada.

        Raises:
            ContextFolderError: Si falta un archivo requerido o tiene formato inválido.
        """
        ...

    async def parse_many(self, base_dir: Path) -> list[ContextFolder]:
        """Parsea todos los Context Folders en un directorio base.

        Cada subdirectorio de base_dir se intenta parsear como un Context Folder.
        Los inválidos se loguean como warning y se omiten (no rompen el lote).

        Args:
            base_dir: Directorio que contiene múltiples subdirectorios de playbooks.

        Returns:
            Lista de ContextFolders válidamente parseados.
        """
        ...

    def validate_config(self, config: dict) -> list[str]:
        """Valida un dict de config.yaml y retorna lista de errores.

        Returns:
            Lista vacía si todo ok. Lista de mensajes de error si hay problemas.
        """
        ...
```

### Data types (ya en types.py)

```python
@dataclass
class PlaybookConfig:
    name: str
    description: str = ""
    tier: LLMTier = LLMTier.CHEAP
    timeout: int = 300
    permissions: list[str] = field(default_factory=list)
    allowed_commands: list[str] = field(default_factory=list)
    blocked_commands: list[str] = field(default_factory=list)

@dataclass
class ContextFolder:
    path: str
    config: PlaybookConfig
    instructions: str       # Contenido completo de playbook.md
    steps: list[str] = field(default_factory=list)      # v2
    templates: dict[str, str] = field(default_factory=dict)  # v2
```

### Errors

```python
class ContextFolderError(Exception):
    """Error base para problemas con Context Folders."""
    def __init__(self, path: str, message: str) -> None:
        self.path = path
        super().__init__(f"Invalid Context Folder at {path}: {message}")

class PlaybookNotFoundError(ContextFolderError):
    """playbook.md no encontrado en el directorio."""
    def __init__(self, path: str) -> None:
        super().__init__(path, "playbook.md not found")

class ConfigNotFoundError(ContextFolderError):
    """config.yaml no encontrado en el directorio."""
    def __init__(self, path: str) -> None:
        super().__init__(path, "config.yaml not found")

class ConfigValidationError(ContextFolderError):
    """config.yaml tiene valores inválidos."""
    def __init__(self, path: str, errors: list[str]) -> None:
        self.errors = errors
        super().__init__(path, f"Invalid config: {'; '.join(errors)}")
```

---

## Playbooks de ejemplo requeridos (5)

### 1. hello_world/ (válido, minimal)
```yaml
# config.yaml
name: "Hello World"
description: "A simple assistant"
tier: 1
timeout: 60
permissions:
  - cli
```

### 2. system_monitor/ (válido, con allowed_commands)
```yaml
name: "System Monitor"
description: "PC health monitoring"
tier: 1
timeout: 30
permissions:
  - cli
allowed_commands: ["top", "free", "df", "ps", "uptime"]
```

### 3. code_reviewer/ (válido, tier alto)
```yaml
name: "Code Reviewer"
description: "Reviews code for quality and bugs"
tier: 3
timeout: 120
permissions:
  - cli
  - files
```

### 4. invalid_missing_name/ (inválido — falta name)
```yaml
description: "This config is missing the required name field"
tier: 1
```

### 5. invalid_bad_tier/ (inválido — tier fuera de rango)
```yaml
name: "Bad Config"
tier: 7
```

---

## Test cases

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | Parse valid hello_world | hello_world/ | ContextFolder con name="Hello World", tier=1 |
| 2 | Parse valid system_monitor | system_monitor/ | allowed_commands tiene 5 items |
| 3 | Parse valid code_reviewer | code_reviewer/ | tier=PREMIUM, permissions=["cli","files"] |
| 4 | Missing playbook.md | dir sin playbook.md | PlaybookNotFoundError |
| 5 | Missing config.yaml | dir sin config.yaml | ConfigNotFoundError |
| 6 | Missing name in config | invalid_missing_name/ | ConfigValidationError |
| 7 | Invalid tier | invalid_bad_tier/ | ConfigValidationError |
| 8 | parse_many con mix | directorio con 3 válidos + 2 inválidos | Retorna 3, loguea 2 warnings |
| 9 | Empty playbook.md | archivo vacío | ContextFolder con instructions="" y title del dirname |
| 10 | Config with unknown fields | config con campo "extra: true" | No falla (ignora campos extra) |
