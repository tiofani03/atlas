use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    pub total_artifacts: usize,
    pub connectors_count: usize,
    pub db_size_bytes: u64,
}

#[derive(Clone)]
pub struct Storage {
    pub path: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create storage directory {:?}", parent))?;
        }
        let storage = Storage { path };
        storage.init_schema()?;
        Ok(storage)
    }

    pub fn get_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", self.path))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;
        Ok(conn)
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Perform migration from old knowledge_objects schema if present
        let has_old_table: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='knowledge_objects'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
            > 0;

        if has_old_table {
            let has_new_table: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='knowledge_artifacts'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0)
                > 0;

            if !has_new_table {
                conn.execute_batch("DROP TABLE IF EXISTS knowledge_fts;")?;
                conn.execute_batch("DROP TRIGGER IF EXISTS ko_ai; DROP TRIGGER IF EXISTS ko_ad; DROP TRIGGER IF EXISTS ko_au;")?;
            }
        }

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS knowledge_artifacts (
                id TEXT PRIMARY KEY NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT,
                body TEXT NOT NULL,
                provider TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_url TEXT NOT NULL,
                repository TEXT,
                tags TEXT NOT NULL,
                relationships TEXT NOT NULL,
                created_at TEXT,
                updated_at TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                checksum TEXT NOT NULL,
                metadata TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_ka_provider_source ON knowledge_artifacts(provider, source_id);
            CREATE INDEX IF NOT EXISTS idx_ka_kind ON knowledge_artifacts(kind);
            CREATE INDEX IF NOT EXISTS idx_ka_repository ON knowledge_artifacts(repository);
            CREATE INDEX IF NOT EXISTS idx_ka_updated_at ON knowledge_artifacts(updated_at);

            CREATE TABLE IF NOT EXISTS artifact_relationships (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id, relationship_type)
            );

            CREATE INDEX IF NOT EXISTS idx_rel_source ON artifact_relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_rel_target ON artifact_relationships(target_id);

            CREATE TABLE IF NOT EXISTS connectors_state (
                connector_id TEXT PRIMARY KEY NOT NULL,
                provider TEXT NOT NULL,
                last_synced_at TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
                id UNINDEXED,
                title,
                summary,
                body,
                tags,
                repository,
                kind,
                provider,
                source_id,
                tokenize = 'porter unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS ka_ai AFTER INSERT ON knowledge_artifacts BEGIN
                INSERT INTO knowledge_fts(id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.body, new.tags, COALESCE(new.repository, ''), new.kind, new.provider, new.source_id);
            END;

            CREATE TRIGGER IF NOT EXISTS ka_ad AFTER DELETE ON knowledge_artifacts BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES ('delete', old.id, old.title, COALESCE(old.summary, ''), old.body, old.tags, COALESCE(old.repository, ''), old.kind, old.provider, old.source_id);
            END;

            CREATE TRIGGER IF NOT EXISTS ka_au AFTER UPDATE ON knowledge_artifacts BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES ('delete', old.id, old.title, COALESCE(old.summary, ''), old.body, old.tags, COALESCE(old.repository, ''), old.kind, old.provider, old.source_id);
                INSERT INTO knowledge_fts(id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.body, new.tags, COALESCE(new.repository, ''), new.kind, new.provider, new.source_id);
            END;
            ",
        )?;

        Ok(())
    }

    pub fn get_existing_checksum(&self, id: &str) -> Result<Option<String>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT checksum FROM knowledge_artifacts WHERE id = ?1")?;
        let checksum: Option<String> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()?;
        Ok(checksum)
    }

    pub fn upsert_artifact(&self, artifact: &KnowledgeArtifact) -> Result<()> {
        let conn = self.get_connection()?;
        let tags_json = serde_json::to_string(&artifact.tags)?;
        let rels_json = serde_json::to_string(&artifact.relationships)?;
        let meta_json = serde_json::to_string(&artifact.metadata)?;
        let created_at_str = artifact.created_at.map(|dt| dt.to_rfc3339());

        conn.execute(
            "INSERT INTO knowledge_artifacts (
                id, kind, title, summary, body, provider, source_id, source_url,
                repository, tags, relationships, created_at, updated_at, synced_at,
                checksum, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                title = excluded.title,
                summary = excluded.summary,
                body = excluded.body,
                provider = excluded.provider,
                source_id = excluded.source_id,
                source_url = excluded.source_url,
                repository = excluded.repository,
                tags = excluded.tags,
                relationships = excluded.relationships,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                synced_at = excluded.synced_at,
                checksum = excluded.checksum,
                metadata = excluded.metadata
            ",
            params![
                artifact.id,
                artifact.kind.to_string(),
                artifact.title,
                artifact.summary,
                artifact.body,
                artifact.provider,
                artifact.source_id,
                artifact.source_url,
                artifact.repository,
                tags_json,
                rels_json,
                created_at_str,
                artifact.updated_at.to_rfc3339(),
                artifact.synced_at.to_rfc3339(),
                artifact.checksum,
                meta_json,
            ],
        )?;

        // Update artifact_relationships graph table
        conn.execute(
            "DELETE FROM artifact_relationships WHERE source_id = ?1 OR source_id = ?2",
            params![artifact.id, artifact.source_id],
        )?;

        for rel in &artifact.relationships {
            conn.execute(
                "INSERT OR REPLACE INTO artifact_relationships (source_id, target_id, relationship_type)
                 VALUES (?1, ?2, ?3)",
                params![rel.source_id, rel.target_id, rel.relationship_type],
            )?;
        }

        Ok(())
    }

    pub fn get_artifact_by_id(&self, id_or_source_id: &str) -> Result<Option<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts WHERE id = ?1 OR source_id = ?1",
        )?;
        let obj = stmt.query_row(params![id_or_source_id], Self::row_to_artifact).optional()?;
        Ok(obj)
    }

    pub fn get_related_artifacts(
        &self,
        id_or_source_id: &str,
    ) -> Result<Vec<(ArtifactRelationship, KnowledgeArtifact)>> {
        let conn = self.get_connection()?;

        // Resolve canonical ID and source_id of the key artifact if available
        let key_artifact = self.get_artifact_by_id(id_or_source_id)?;
        let (id_key, source_key) = match key_artifact {
            Some(ref a) => (a.id.clone(), a.source_id.clone()),
            None => (id_or_source_id.to_string(), id_or_source_id.to_string()),
        };

        let mut stmt = conn.prepare(
            "SELECT source_id, target_id, relationship_type
             FROM artifact_relationships
             WHERE source_id = ?1 OR source_id = ?2 OR target_id = ?1 OR target_id = ?2",
        )?;

        let rel_rows = stmt.query_map(params![id_key, source_key], |row| {
            Ok(ArtifactRelationship {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relationship_type: row.get(2)?,
            })
        })?;

        let mut results = Vec::new();
        for rel in rel_rows {
            let rel = rel?;
            let other_key = if rel.source_id == id_key || rel.source_id == source_key {
                &rel.target_id
            } else {
                &rel.source_id
            };

            if let Some(other_artifact) = self.get_artifact_by_id(other_key)? {
                results.push((rel, other_artifact));
            }
        }

        Ok(results)
    }

    pub fn query_by_repository(
        &self,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts
             WHERE repository = ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![repo, limit as i64], Self::row_to_artifact)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_last_sync(&self, connector_id: &str) -> Result<Option<DateTime<Utc>>> {
        let conn = self.get_connection()?;
        let mut stmt =
            conn.prepare("SELECT last_synced_at FROM connectors_state WHERE connector_id = ?1")?;
        let time_str: Option<String> = stmt
            .query_row(params![connector_id], |row| row.get(0))
            .optional()?;

        match time_str {
            Some(s) => {
                let dt = DateTime::parse_from_rfc3339(&s)
                    .map(|d| d.with_timezone(&Utc))
                    .context("Invalid RFC3339 timestamp in connectors_state")?;
                Ok(Some(dt))
            }
            None => Ok(None),
        }
    }

    pub fn update_last_sync(
        &self,
        connector_id: &str,
        provider: &str,
        timestamp: DateTime<Utc>,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO connectors_state (connector_id, provider, last_synced_at, status, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(connector_id) DO UPDATE SET
                last_synced_at = excluded.last_synced_at,
                status = excluded.status,
                error_message = excluded.error_message",
            params![
                connector_id,
                provider,
                timestamp.to_rfc3339(),
                status,
                error_message
            ],
        )?;
        Ok(())
    }

    pub fn search_fts(
        &self,
        query: &str,
        kind: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let clean_query = query.trim().replace('"', "");
        let formatted_query = if clean_query.is_empty() {
            "*".to_string()
        } else if clean_query.contains('-') || clean_query.contains(':') || clean_query.contains(' ') {
            format!("\"{}\"*", clean_query)
        } else if clean_query.ends_with('*') {
            clean_query.to_string()
        } else {
            format!("{}*", clean_query)
        };

        let like_query = format!("%{}%", clean_query);

        let mut sql = "SELECT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider,
                              ka.source_id, ka.source_url, ka.repository, ka.tags,
                              ka.relationships, ka.created_at, ka.updated_at, ka.synced_at,
                              ka.checksum, ka.metadata
                       FROM knowledge_artifacts ka
                       WHERE (ka.source_id LIKE ?1
                          OR ka.id IN (SELECT id FROM knowledge_fts WHERE knowledge_fts MATCH ?2))".to_string();

        let mut param_index = 3;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(like_query));
        params_vec.push(Box::new(formatted_query));

        if let Some(k) = kind {
            sql.push_str(&format!(" AND ka.kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            sql.push_str(&format!(" AND ka.tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        if let Some(repo) = repository {
            sql.push_str(&format!(" AND ka.repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        sql.push_str(&format!(" ORDER BY ka.updated_at DESC LIMIT ?{}", param_index));
        params_vec.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(rusqlite_params.as_slice(), Self::row_to_artifact)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }


    pub fn query_structured(
        &self,
        kind: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut sql = "SELECT id, kind, title, summary, body, provider, source_id,
                              source_url, repository, tags, relationships, created_at,
                              updated_at, synced_at, checksum, metadata
                       FROM knowledge_artifacts WHERE 1=1".to_string();

        let mut param_index = 1;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(k) = kind {
            sql.push_str(&format!(" AND kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            sql.push_str(&format!(" AND tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        if let Some(repo) = repository {
            sql.push_str(&format!(" AND repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT ?{}", param_index));
        params_vec.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(rusqlite_params.as_slice(), Self::row_to_artifact)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_stats(&self) -> Result<StorageStats> {
        let conn = self.get_connection()?;

        let total_artifacts: usize = conn.query_row(
            "SELECT COUNT(*) FROM knowledge_artifacts",
            [],
            |row| row.get(0),
        )?;

        let connectors_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM connectors_state",
            [],
            |row| row.get(0),
        )?;

        let db_size_bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);

        Ok(StorageStats {
            total_artifacts,
            connectors_count,
            db_size_bytes,
        })
    }

    fn row_to_artifact(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeArtifact> {
        let id: String = row.get(0)?;
        let kind_str: String = row.get(1)?;
        let title: String = row.get(2)?;
        let summary: Option<String> = row.get(3)?;
        let body: String = row.get(4)?;
        let provider: String = row.get(5)?;
        let source_id: String = row.get(6)?;
        let source_url: String = row.get(7)?;
        let repository: Option<String> = row.get(8)?;
        let tags_json: String = row.get(9)?;
        let rels_json: String = row.get(10)?;
        let created_str: Option<String> = row.get(11)?;
        let updated_str: String = row.get(12)?;
        let synced_str: String = row.get(13)?;
        let checksum: String = row.get(14)?;
        let meta_json: String = row.get(15)?;

        let kind = ArtifactKind::from_str(&kind_str).unwrap_or(ArtifactKind::Document);
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let relationships: Vec<ArtifactRelationship> =
            serde_json::from_str(&rels_json).unwrap_or_default();
        let metadata: serde_json::Value =
            serde_json::from_str(&meta_json).unwrap_or(serde_json::Value::Null);

        let created_at = created_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc))
                .ok()
        });

        let updated_at = DateTime::parse_from_rfc3339(&updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let synced_at = DateTime::parse_from_rfc3339(&synced_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(KnowledgeArtifact {
            id,
            kind,
            title,
            summary,
            body,
            provider,
            source_id,
            source_url,
            repository,
            tags,
            relationships,
            created_at,
            updated_at,
            synced_at,
            checksum,
            metadata,
        })
    }
}

