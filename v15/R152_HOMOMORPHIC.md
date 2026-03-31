# FASE R152 — HOMOMORPHIC OPERATIONS: Procesar datos sin desencriptarlos

**Objetivo:** Para datos ultra-sensibles (medical, financial, legal), el agente puede operar sobre datos ENCRIPTADOS. El LLM nunca ve los datos en claro — solo trabaja con datos cifrados y retorna resultados que el usuario desencripta localmente.

---

## Tareas

### 1. Homomorphic encryption layer (simplificado)

```rust
// Full Homomorphic Encryption (FHE) es muy lento para uso general
// Approach pragmático: Partial Homomorphic + Secure Enclaves

// Opción A: Tokenización + processing
// Los datos sensibles se tokenizan antes de enviar al LLM:
// "Juan García, DNI 12345678" → "PERSON_1, ID_TOKEN_1"
// El LLM procesa con tokens → resultado con tokens
// Detokenización local: "PERSON_1" → "Juan García"

pub struct DataTokenizer {
    token_map: HashMap<String, String>,  // original → token
    reverse_map: HashMap<String, String>, // token → original
}

impl DataTokenizer {
    pub fn tokenize(&mut self, text: &str) -> String {
        // Detectar PII: nombres, IDs, emails, teléfonos, direcciones
        // Reemplazar cada uno con un token único
        // Guardar mapeo localmente (NUNCA sale de la PC)
    }
    
    pub fn detokenize(&self, text: &str) -> String {
        // Reemplazar tokens con valores originales
    }
}
```

### 2. PII detection engine

```rust
pub fn detect_pii(text: &str) -> Vec<PIIEntity> {
    // Detectar:
    // - Nombres de personas (NER: Named Entity Recognition)
    // - Documentos de identidad (regex: CI, DNI, SSN patterns)
    // - Emails (regex)
    // - Teléfonos (regex + libphonenumber)
    // - Direcciones (NER)
    // - Números de cuenta bancaria (regex patterns)
    // - Números de tarjeta de crédito (Luhn algorithm)
    
    // Usar modelo NER local (ONNX, R81) para nombres/direcciones
    // Usar regex para patterns conocidos
}
```

### 3. Secure processing modes

```
Mode 1: STANDARD (default)
  Data → LLM → Response
  Fast, full capability, data visible to LLM provider

Mode 2: TOKENIZED
  Data → Tokenize PII → LLM → Response with tokens → Detokenize
  Medium speed, full capability, PII never leaves PC

Mode 3: LOCAL ONLY
  Data → Local model (R81) → Response
  Slower, limited capability, NOTHING leaves PC

Mode 4: ENCLAVE (future, requires hardware)
  Data → Intel SGX/ARM TrustZone enclave → Process → Result
  Hardware-level isolation, data encrypted even in RAM
```

### 4. Frontend: Privacy mode selector

```
PRIVACY MODE                              [Learn more]
──────────────────────────────────────
Task: "Analyze patient records for trends"

⚠️ This task contains sensitive data (medical records detected)

Select processing mode:
○ Standard — Fastest, data sent to AI provider
● Tokenized — Patient names replaced with tokens before sending ← recommended
○ Local only — Processed entirely on your PC (slower, less accurate)

PII detected and will be tokenized:
  - 34 patient names → PATIENT_1 through PATIENT_34
  - 12 medical record numbers → MRN_TOKEN_1 through MRN_TOKEN_12
  - 8 phone numbers → PHONE_TOKEN_1 through PHONE_TOKEN_8

[Process with tokenization]
```

### 5. Auto-detect sensitive data

```rust
// Antes de CADA request al LLM:
// 1. Scan el texto por PII
// 2. Si PII encontrado + privacy mode no es "standard":
//    → Auto-tokenize → enviar tokenizado → detokenize resultado
// 3. Si el usuario tiene "always tokenize" en Settings:
//    → Todo se tokeniza automáticamente
```

---

## Demo

1. "Analyze these patient records" → PII detected → auto-tokenized → LLM sees "PATIENT_1 has condition X" → result detokenized locally → "Juan García has condition X"
2. Verificar: en los logs del LLM call → CERO nombres reales, solo tokens
3. Local only mode: proceso completo sin internet → slower pero NADA sale de la PC
4. Settings: "Always tokenize" → todo el PII se reemplaza automáticamente en cada request
