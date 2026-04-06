use crate::{App, Extension};
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open_or_create() -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        path.push("file-type-groups.db");
        path
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS apps (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS extensions (
                ext            TEXT PRIMARY KEY,
                description    TEXT NOT NULL DEFAULT '',
                default_app_id INTEGER REFERENCES apps(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS ext_apps (
                ext    TEXT NOT NULL REFERENCES extensions(ext) ON DELETE CASCADE,
                app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
                PRIMARY KEY (ext, app_id)
            );",
        )
    }

    // --- Scanner operations ---

    pub fn upsert_app(&self, name: &str, path: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO apps (name, path) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET path = excluded.path",
            params![name, path],
        )?;
        let id: i64 =
            self.conn
                .query_row("SELECT id FROM apps WHERE name = ?1", params![name], |r| {
                    r.get(0)
                })?;
        Ok(id)
    }

    pub fn upsert_ext_app(&self, ext: &str, app_id: i64, description: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO extensions (ext, description) VALUES (?1, ?2)",
            params![ext, description],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO ext_apps (ext, app_id) VALUES (?1, ?2)",
            params![ext, app_id],
        )?;
        Ok(())
    }

    pub fn set_default_app(&self, ext: &str, app_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE extensions SET default_app_id = ?1 WHERE ext = ?2",
            params![app_id, ext],
        )?;
        Ok(())
    }

    pub fn remove_stale_ext_apps(&self, app_id: i64, current_exts: &[String]) -> Result<()> {
        if current_exts.is_empty() {
            self.conn
                .execute("DELETE FROM ext_apps WHERE app_id = ?1", params![app_id])?;
            return Ok(());
        }
        let placeholders: Vec<String> = current_exts
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect();
        let sql = format!(
            "DELETE FROM ext_apps WHERE app_id = ?1 AND ext NOT IN ({})",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(app_id));
        for ext in current_exts {
            param_values.push(Box::new(ext.clone()));
        }
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        stmt.execute(refs.as_slice())?;
        Ok(())
    }

    pub fn all_extensions(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT ext FROM extensions")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn cleanup_orphan_extensions(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM extensions WHERE ext NOT IN (SELECT DISTINCT ext FROM ext_apps)",
            [],
        )?;
        Ok(())
    }

    // --- Query operations ---

    /// Get all apps, with ext_count = number of extensions this app is the default for.
    pub fn get_apps(&self, filter: Option<&str>) -> Result<Vec<App>> {
        let sql = "SELECT a.id, a.name, a.path,
                          (SELECT COUNT(*) FROM extensions WHERE default_app_id = a.id) as ext_count
                   FROM apps a
                   WHERE (?1 IS NULL OR a.name LIKE '%' || ?1 || '%')
                   ORDER BY a.name COLLATE NOCASE";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![filter], |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Get extensions for a given app (by default_app_id), or all extensions if app_id is None.
    pub fn get_extensions_for_app(&self, app_id: Option<i64>) -> Result<Vec<Extension>> {
        let sql = if app_id.is_some() {
            "SELECT e.ext, e.description, e.default_app_id, a.name
             FROM extensions e
             LEFT JOIN apps a ON a.id = e.default_app_id
             WHERE e.default_app_id = ?1
             ORDER BY e.ext"
        } else {
            "SELECT e.ext, e.description, e.default_app_id, a.name
             FROM extensions e
             LEFT JOIN apps a ON a.id = e.default_app_id
             WHERE ?1 IS NULL
             ORDER BY e.ext"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![app_id], |r| {
            Ok(Extension {
                ext: r.get(0)?,
                description: r.get(1)?,
                default_app_id: r.get(2)?,
                default_app_name: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Get apps that can open at least one extension currently defaulted to source_app_id.
    /// ext_count = number of overlapping extensions. Excludes the source app.
    pub fn get_candidate_targets(&self, source_app_id: i64) -> Result<Vec<App>> {
        let sql = "SELECT a.id, a.name, a.path, COUNT(DISTINCT ea.ext) as overlap_count
                   FROM ext_apps ea
                   JOIN apps a ON a.id = ea.app_id
                   WHERE ea.ext IN (SELECT ext FROM extensions WHERE default_app_id = ?1)
                     AND ea.app_id != ?1
                   GROUP BY a.id
                   ORDER BY overlap_count DESC, a.name COLLATE NOCASE";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![source_app_id], |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Get extension names that are defaulted to source and that target can open.
    pub fn get_eligible_extensions(
        &self,
        source_app_id: i64,
        target_app_id: i64,
    ) -> Result<Vec<String>> {
        let sql = "SELECT e.ext
                   FROM extensions e
                   JOIN ext_apps ea ON ea.ext = e.ext AND ea.app_id = ?2
                   WHERE e.default_app_id = ?1
                   ORDER BY e.ext";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![source_app_id, target_app_id], |r| {
            r.get::<_, String>(0)
        })?;
        rows.collect()
    }

    /// Reassign extensions to a new default app.
    pub fn reassign_extensions(&self, exts: &[String], target_app_id: i64) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("UPDATE extensions SET default_app_id = ?1 WHERE ext = ?2")?;
        for ext in exts {
            stmt.execute(params![target_app_id, ext])?;
        }
        Ok(())
    }

    /// Get all apps that can open a given extension.
    pub fn get_apps_for_extension(&self, ext: &str) -> Result<Vec<App>> {
        let sql = "SELECT a.id, a.name, a.path,
                          (SELECT COUNT(*) FROM extensions WHERE default_app_id = a.id) as ext_count
                   FROM apps a
                   JOIN ext_apps ea ON ea.app_id = a.id
                   WHERE ea.ext = ?1
                   ORDER BY a.name COLLATE NOCASE";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![ext], |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Get apps that can open ALL of the given extensions, excluding source_app_id.
    pub fn get_apps_for_extensions(
        &self,
        exts: &[String],
        exclude_app_id: Option<i64>,
    ) -> Result<Vec<App>> {
        if exts.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = exts
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT a.id, a.name, a.path,
                    (SELECT COUNT(DISTINCT ea2.ext) FROM ext_apps ea2
                     WHERE ea2.app_id = a.id
                       AND ea2.ext IN (SELECT ext FROM extensions WHERE default_app_id = ?{})) as ext_count
             FROM apps a
             JOIN ext_apps ea ON ea.app_id = a.id
             WHERE ea.ext IN ({})
               AND (?{} IS NULL OR a.id != ?{})
             GROUP BY a.id
             ORDER BY ext_count DESC, a.name COLLATE NOCASE",
            exts.len() + 1,
            placeholders.join(","),
            exts.len() + 1,
            exts.len() + 1,
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for ext in exts {
            param_values.push(Box::new(ext.clone()));
        }
        param_values.push(Box::new(exclude_app_id));
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// For each extension defaulted to source_app_id, count how many other apps can open it.
    pub fn get_extension_target_counts(&self, source_app_id: i64) -> Result<Vec<(String, i64)>> {
        let sql = "SELECT e.ext, COUNT(ea.app_id) as target_count
                   FROM extensions e
                   LEFT JOIN ext_apps ea ON ea.ext = e.ext AND ea.app_id != ?1
                   WHERE e.default_app_id = ?1
                   GROUP BY e.ext
                   ORDER BY e.ext";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![source_app_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        rows.collect()
    }

    pub fn get_summary(&self) -> Result<(i64, i64)> {
        let app_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM apps", [], |r| r.get(0))?;
        let ext_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM extensions", [], |r| r.get(0))?;
        Ok((app_count, ext_count))
    }
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("com.devbox.filetypegroups")
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".file-type-groups"))
    }
}
