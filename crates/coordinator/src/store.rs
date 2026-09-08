use std::path::Path;

use rusqlite::{params, Connection};
use unveil_contracts::{DashboardOverview, NamedCount, SessionRow};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(workspace: &Path) -> rusqlite::Result<Self> {
        std::fs::create_dir_all(workspace).ok();
        let conn = Connection::open(workspace.join("metadata.sqlite"))?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS sessions (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              mode TEXT NOT NULL,
              created_at TEXT NOT NULL,
              saved INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS artifacts (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              sha256 TEXT NOT NULL,
              byte_length INTEGER NOT NULL,
              display_name TEXT NOT NULL,
              kind_hint TEXT NOT NULL,
              parent_id TEXT,
              created_by TEXT NOT NULL,
              FOREIGN KEY(session_id) REFERENCES sessions(id)
            );
            CREATE TABLE IF NOT EXISTS jobs (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              status TEXT NOT NULL,
              artifact_id TEXT,
              error TEXT
            );
            CREATE TABLE IF NOT EXISTS findings (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              artifact_id TEXT NOT NULL,
              technique_id TEXT NOT NULL,
              category TEXT NOT NULL,
              json TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn ensure_session(&self, id: &str, name: &str, mode: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions(id,name,mode,created_at,saved) VALUES(?1,?2,?3,datetime('now'),0)",
            params![id, name, mode],
        )?;
        Ok(())
    }

    pub fn mark_saved(&self, id: &str, name: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET saved=1, name=?2 WHERE id=?1",
            params![id, name],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM findings WHERE session_id=?1", [id])?;
        self.conn.execute("DELETE FROM jobs WHERE session_id=?1", [id])?;
        self.conn.execute("DELETE FROM artifacts WHERE session_id=?1", [id])?;
        self.conn.execute("DELETE FROM sessions WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn insert_artifact(
        &self,
        id: &str,
        session_id: &str,
        sha256: &str,
        len: u64,
        name: &str,
        kind: &str,
        parent: Option<&str>,
        created_by: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO artifacts(id,session_id,sha256,byte_length,display_name,kind_hint,parent_id,created_by)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, session_id, sha256, len as i64, name, kind, parent, created_by],
        )?;
        Ok(())
    }

    pub fn insert_job(
        &self,
        id: &str,
        session_id: &str,
        kind: &str,
        status: &str,
        artifact_id: Option<&str>,
        error: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO jobs(id,session_id,kind,status,artifact_id,error) VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, session_id, kind, status, artifact_id, error],
        )?;
        Ok(())
    }

    pub fn replace_findings(
        &self,
        session_id: &str,
        artifact_id: &str,
        findings: &[unveil_contracts::Finding],
    ) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM findings WHERE artifact_id=?1", [artifact_id])?;
        for finding in findings {
            let json = serde_json::to_string(finding).unwrap_or_else(|_| "{}".into());
            self.conn.execute(
                "INSERT INTO findings(id,session_id,artifact_id,technique_id,category,json) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    finding.id,
                    session_id,
                    artifact_id,
                    finding.technique_id,
                    finding.category,
                    json
                ],
            )?;
        }
        Ok(())
    }

    pub fn overview(&self, isolation_passed: u32) -> DashboardOverview {
        let mut overview = DashboardOverview::empty();
        overview.isolation_passed = isolation_passed;
        overview.saved_sessions = self.count("SELECT COUNT(*) FROM sessions WHERE saved=1");
        overview.jobs_total = self.count("SELECT COUNT(*) FROM jobs");
        overview.findings_total = self.count("SELECT COUNT(*) FROM findings");
        overview.categories = self.named("SELECT category, COUNT(*) FROM findings GROUP BY category ORDER BY category");
        overview.techniques = self.named("SELECT technique_id, COUNT(*) FROM findings GROUP BY technique_id ORDER BY technique_id");
        overview.job_outcomes = self.named("SELECT status, COUNT(*) FROM jobs GROUP BY status ORDER BY status");
        overview.module_jobs =
            self.named("SELECT kind, COUNT(*) FROM jobs GROUP BY kind ORDER BY kind");
        if let Ok(mut stmt) = self.conn.prepare(
            "SELECT s.name, COUNT(f.id), CASE s.saved WHEN 1 THEN 'saved' ELSE 'ephemeral' END
             FROM sessions s LEFT JOIN findings f ON f.session_id=s.id
             WHERE s.saved=1 GROUP BY s.id ORDER BY s.created_at",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(SessionRow {
                    session_id: String::new(),
                    name: row.get(0)?,
                    findings: row.get::<_, i64>(1)? as u32,
                    status: row.get(2)?,
                })
            }) {
                overview.sessions = rows.filter_map(|r| r.ok()).collect();
            }
        }
        overview
    }

    fn count(&self, sql: &str) -> u32 {
        self.conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as u32
    }

    fn named(&self, sql: &str) -> Vec<NamedCount> {
        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| {
            Ok(NamedCount {
                id: row.get(0)?,
                count: row.get::<_, i64>(1)? as u32,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }
}
