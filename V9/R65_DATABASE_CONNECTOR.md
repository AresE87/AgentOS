# FASE R65 — DATABASE CONNECTOR: El agente consulta tus bases de datos

**Objetivo:** El usuario le dice "cuántas ventas hubo en marzo" y el agente genera una query SQL, la ejecuta contra la base de datos real, y presenta los resultados formateados.

---

## Tareas

### 1. Database provider abstraction

```rust
pub trait DatabaseProvider: Send + Sync {
    async fn test_connection(&self) -> Result<()>;
    async fn list_tables(&self) -> Result<Vec<TableInfo>>;
    async fn describe_table(&self, table: &str) -> Result<Vec<ColumnInfo>>;
    async fn execute_query(&self, sql: &str) -> Result<QueryResult>;
    fn provider_name(&self) -> &str;
}

pub struct PostgresProvider { connection_string: String }
pub struct MySQLProvider { connection_string: String }
pub struct SQLiteProvider { path: PathBuf }
// Futuro: MongoDB, Redis, etc.
```

### 2. Natural language → SQL

```rust
// El agente genera SQL a partir de lenguaje natural:
async fn nl_to_sql(question: &str, schema: &[TableInfo]) -> Result<String> {
    let prompt = format!(
        "Given this database schema:\n{}\n\nGenerate a SQL query to answer: '{}'\n\nRespond with ONLY the SQL query, no explanation.",
        format_schema(schema), question
    );
    gateway.call(&prompt, Tier::Standard).await
}

// SAFETY: el query se ejecuta en READ ONLY mode
// NUNCA permitir INSERT, UPDATE, DELETE, DROP, ALTER desde NL
```

### 3. Safety: read-only por defecto

```rust
fn validate_query(sql: &str) -> Result<(), String> {
    let forbidden = ["INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE", "GRANT"];
    let upper = sql.to_uppercase();
    for keyword in forbidden {
        if upper.contains(keyword) {
            return Err(format!("Write operations not allowed: found {}", keyword));
        }
    }
    Ok(())
}
// El usuario puede habilitar write mode en Settings con warning prominente
```

### 4. Frontend: Database panel en Settings + resultados en Chat

```
Settings → Databases:
┌──────────────────────────────────────────────────┐
│ 📊 DATABASES                      [+ Add Database]│
│                                                    │
│ PostgreSQL — Sales DB     ● Connected              │
│ host: db.company.com:5432                          │
│ Tables: 23 · Last query: 5 min ago                 │
│ [Test] [Edit] [Remove]                             │
│                                                    │
│ SQLite — Local analytics  ● Connected              │
│ path: ~/data/analytics.db                          │
│ Tables: 8                                          │
│ [Test] [Edit] [Remove]                             │
└──────────────────────────────────────────────────┘

En Chat:
User: "Cuántas ventas hubo en marzo"
Agent: 
  📊 Query: SELECT COUNT(*) FROM sales WHERE date >= '2026-03-01' AND date < '2026-04-01'
  
  Result: 1,247 ventas en marzo 2026
  
  ┌──────────┬──────────┬──────────┐
  │ Semana   │ Ventas   │ Total $  │
  ├──────────┼──────────┼──────────┤
  │ Sem 1    │ 287      │ $34,500  │
  │ Sem 2    │ 312      │ $41,200  │
  │ Sem 3    │ 298      │ $37,800  │
  │ Sem 4    │ 350      │ $45,100  │
  └──────────┴──────────┴──────────┘
```

### 5. IPC commands

```rust
#[tauri::command] async fn db_add(config: DatabaseConfig) -> Result<String, String>
#[tauri::command] async fn db_test(id: String) -> Result<(), String>
#[tauri::command] async fn db_list() -> Result<Vec<DatabaseInfo>, String>
#[tauri::command] async fn db_tables(id: String) -> Result<Vec<TableInfo>, String>
#[tauri::command] async fn db_query(id: String, question: String) -> Result<QueryResult, String>
#[tauri::command] async fn db_raw_query(id: String, sql: String) -> Result<QueryResult, String>
#[tauri::command] async fn db_remove(id: String) -> Result<(), String>
```

---

## Demo

1. Agregar PostgreSQL en Settings → "Connected ✅ — 23 tables"
2. "Cuántas ventas hubo en marzo" → SQL generado → resultado con tabla formateada
3. "Mostrá los top 10 clientes por monto" → query + tabla + datos reales
4. Intentar "borrá la tabla users" → BLOCKED: "Write operations not allowed"
5. "Describí la tabla sales" → columnas, tipos, ejemplo de datos
