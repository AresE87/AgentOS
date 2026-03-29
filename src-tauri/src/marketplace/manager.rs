use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub install_path: String,
    pub installed_at: String,
}

pub struct PackageManager {
    db_path: PathBuf,
    playbooks_dir: PathBuf,
}

impl PackageManager {
    pub fn new(db_path: PathBuf, playbooks_dir: PathBuf) -> Self {
        Self {
            db_path,
            playbooks_dir,
        }
    }

    fn open_db(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| format!("DB open error: {}", e))
    }

    /// Ensure the marketplace tables exist.
    pub fn ensure_tables(&self) -> Result<(), String> {
        let conn = self.open_db()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS marketplace_installs (
                id           TEXT PRIMARY KEY,
                name         TEXT NOT NULL,
                version      TEXT NOT NULL,
                install_path TEXT NOT NULL,
                installed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS marketplace_reviews (
                id         TEXT PRIMARY KEY,
                package_id TEXT NOT NULL,
                rating     INTEGER NOT NULL CHECK(rating >= 1 AND rating <= 5),
                comment    TEXT,
                created_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Migration error: {}", e))
    }

    /// Install a .aosp package from disk.
    /// .aosp files are ZIP archives containing at minimum a `metadata.json`.
    pub fn install(&self, aosp_path: &Path, package_id: &str) -> Result<InstalledPackage, String> {
        use std::io::Read;

        let file = std::fs::File::open(aosp_path)
            .map_err(|e| format!("Cannot open .aosp file: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {}", e))?;

        // Read metadata.json from the archive
        let (name, version) = {
            let mut meta_file = archive
                .by_name("metadata.json")
                .map_err(|_| "metadata.json not found in package".to_string())?;
            let mut contents = String::new();
            meta_file
                .read_to_string(&mut contents)
                .map_err(|e| format!("Cannot read metadata.json: {}", e))?;
            let meta: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| format!("Invalid metadata.json: {}", e))?;
            let name = meta["name"]
                .as_str()
                .unwrap_or(package_id)
                .to_string();
            let version = meta["version"]
                .as_str()
                .unwrap_or("1.0.0")
                .to_string();
            (name, version)
        };

        // Extract to playbooks_dir/package_id/
        let dest = self.playbooks_dir.join(package_id);
        std::fs::create_dir_all(&dest)
            .map_err(|e| format!("Cannot create install dir: {}", e))?;

        // Re-open archive since we consumed the first borrow
        let file2 = std::fs::File::open(aosp_path)
            .map_err(|e| format!("Cannot re-open .aosp file: {}", e))?;
        let mut archive2 =
            zip::ZipArchive::new(file2).map_err(|e| format!("Re-open ZIP error: {}", e))?;

        for i in 0..archive2.len() {
            let mut entry = archive2
                .by_index(i)
                .map_err(|e| format!("ZIP entry error: {}", e))?;
            let out_path = dest.join(entry.name());
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)
                    .map_err(|e| format!("Dir create error: {}", e))?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Dir create error: {}", e))?;
                }
                let mut out_file = std::fs::File::create(&out_path)
                    .map_err(|e| format!("File create error: {}", e))?;
                std::io::copy(&mut entry, &mut out_file)
                    .map_err(|e| format!("File write error: {}", e))?;
            }
        }

        let install_path = dest.to_string_lossy().to_string();
        let installed_at = Utc::now().to_rfc3339();

        let pkg = InstalledPackage {
            id: package_id.to_string(),
            name,
            version,
            install_path: install_path.clone(),
            installed_at: installed_at.clone(),
        };

        let conn = self.open_db()?;
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_installs (id, name, version, install_path, installed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pkg.id, pkg.name, pkg.version, pkg.install_path, pkg.installed_at],
        )
        .map_err(|e| format!("DB insert error: {}", e))?;

        Ok(pkg)
    }

    /// Simulate an install without a real .aosp file.
    /// Used when the user presses "Install" from the UI catalog.
    pub fn simulate_install(
        &self,
        package_id: &str,
        name: &str,
        version: &str,
    ) -> Result<InstalledPackage, String> {
        let install_path = self
            .playbooks_dir
            .join(package_id)
            .to_string_lossy()
            .to_string();
        let installed_at = Utc::now().to_rfc3339();

        let pkg = InstalledPackage {
            id: package_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            install_path,
            installed_at,
        };

        let conn = self.open_db()?;
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_installs (id, name, version, install_path, installed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pkg.id, pkg.name, pkg.version, pkg.install_path, pkg.installed_at],
        )
        .map_err(|e| format!("DB insert error: {}", e))?;

        tracing::info!(package_id = %package_id, name = %pkg.name, "Marketplace package installed (simulated)");
        Ok(pkg)
    }

    /// Uninstall a package by ID: removes directory and DB record.
    pub fn uninstall(&self, package_id: &str) -> Result<(), String> {
        // Try to remove directory if it exists
        let dest = self.playbooks_dir.join(package_id);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| format!("Failed to remove package dir: {}", e))?;
        }

        let conn = self.open_db()?;
        conn.execute(
            "DELETE FROM marketplace_installs WHERE id = ?1",
            params![package_id],
        )
        .map_err(|e| format!("DB delete error: {}", e))?;

        tracing::info!(package_id = %package_id, "Marketplace package uninstalled");
        Ok(())
    }

    /// List all installed packages.
    pub fn list_installed(&self) -> Result<Vec<InstalledPackage>, String> {
        let conn = self.open_db()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, install_path, installed_at
                 FROM marketplace_installs
                 ORDER BY installed_at DESC",
            )
            .map_err(|e| format!("DB prepare error: {}", e))?;

        let packages = stmt
            .query_map([], |row| {
                Ok(InstalledPackage {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    install_path: row.get(3)?,
                    installed_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("DB query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(packages)
    }

    /// Check whether a package is currently installed.
    pub fn is_installed(&self, package_id: &str) -> bool {
        let conn = match self.open_db() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM marketplace_installs WHERE id = ?1",
                params![package_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        count > 0
    }

    /// Add a review for a package.
    pub fn add_review(
        &self,
        package_id: &str,
        rating: i32,
        comment: Option<&str>,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO marketplace_reviews (id, package_id, rating, comment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, package_id, rating, comment, created_at],
        )
        .map_err(|e| format!("DB insert review error: {}", e))?;
        Ok(id)
    }

    /// Get reviews for a package.
    pub fn get_reviews(&self, package_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.open_db()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rating, comment, created_at
                 FROM marketplace_reviews
                 WHERE package_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("DB prepare error: {}", e))?;

        let reviews = stmt
            .query_map(params![package_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "rating": row.get::<_, i32>(1)?,
                    "comment": row.get::<_, Option<String>>(2)?,
                    "created_at": row.get::<_, String>(3)?,
                }))
            })
            .map_err(|e| format!("DB query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(reviews)
    }
}
