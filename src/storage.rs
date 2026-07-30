use crate::domain::{KnowledgeObject, ObjectType, Relationship, SourceInfo};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_objects: usize,
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
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS knowledge_objects (
                id TEXT PRIMARY KEY NOT NULL,
                object_type TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                relationships TEXT NOT NULL,
                provider TEXT NOT NULL,
                instance_url TEXT NOT NULL,
                original_id TEXT NOT NULL,
                web_url TEXT NOT NULL,
                source_metadata TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                checksum TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_ko_provider_orig ON knowledge_objects(provider, original_id);
            CREATE INDEX IF NOT EXISTS idx_ko_object_type ON knowledge_objects(object_type);
            CREATE INDEX IF NOT EXISTS idx_ko_updated_at ON knowledge_objects(updated_at);

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
                content,
                tags,
                tokenize = 'porter unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS ko_ai AFTER INSERT ON knowledge_objects BEGIN
                INSERT INTO knowledge_fts(id, title, summary, content, tags)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.content, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS ko_ad AFTER DELETE ON knowledge_objects BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, id, title, summary, content, tags)
                VALUES ('delete', old.id, old.title, COALESCE(old.summary, ''), old.content, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS ko_au AFTER UPDATE ON knowledge_objects BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, id, title, summary, content, tags)
                VALUES ('delete', old.id, old.title, COALESCE(old.summary, ''), old.content, old.tags);
                INSERT INTO knowledge_fts(id, title, summary, content, tags)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.content, new.tags);
            END;
            ",
        )?;
        Ok(())
    }

    pub fn get_existing_checksum(&self, id: &str) -> Result<Option<String>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT checksum FROM knowledge_objects WHERE id = ?1")?;
        let checksum: Option<String> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()?;
        Ok(checksum)
    }

    pub fn upsert_object(&self, obj: &KnowledgeObject) -> Result<()> {
        let conn = self.get_connection()?;
        let tags_json = serde_json::to_string(&obj.tags)?;
        let rels_json = serde_json::to_string(&obj.relationships)?;
        let meta_json = serde_json::to_string(&obj.source_metadata)?;

        conn.execute(
            "INSERT INTO knowledge_objects (
                id, object_type, title, summary, content, tags, relationships,
                provider, instance_url, original_id, web_url, source_metadata,
                updated_at, synced_at, checksum
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(id) DO UPDATE SET
                object_type = excluded.object_type,
                title = excluded.title,
                summary = excluded.summary,
                content = excluded.content,
                tags = excluded.tags,
                relationships = excluded.relationships,
                provider = excluded.provider,
                instance_url = excluded.instance_url,
                original_id = excluded.original_id,
                web_url = excluded.web_url,
                source_metadata = excluded.source_metadata,
                updated_at = excluded.updated_at,
                synced_at = excluded.synced_at,
                checksum = excluded.checksum
            ",
            params![
                obj.id,
                obj.object_type.to_string(),
                obj.title,
                obj.summary,
                obj.content,
                tags_json,
                rels_json,
                obj.source.provider,
                obj.source.instance_url,
                obj.source.original_id,
                obj.source.web_url,
                meta_json,
                obj.updated_at.to_rfc3339(),
                obj.synced_at.to_rfc3339(),
                obj.checksum,
            ],
        )?;

        Ok(())
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

    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeObject>> {
        let conn = self.get_connection()?;
        let clean_query = query.trim().replace('"', "");
        let formatted_query = if clean_query.is_empty() {
            "*".to_string()
        } else if clean_query.ends_with('*') {
            clean_query
        } else {
            format!("{}*", clean_query)
        };
        let mut stmt = conn.prepare(
            "SELECT ko.id, ko.object_type, ko.title, ko.summary, ko.content,
                    ko.tags, ko.relationships, ko.provider, ko.instance_url,
                    ko.original_id, ko.web_url, ko.source_metadata,
                    ko.updated_at, ko.synced_at, ko.checksum
             FROM knowledge_fts fts
             JOIN knowledge_objects ko ON fts.id = ko.id
             WHERE knowledge_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![formatted_query, limit as i64], Self::row_to_object)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn query_structured(
        &self,
        object_type: Option<&str>,
        tag: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KnowledgeObject>> {
        let conn = self.get_connection()?;
        let mut sql = "SELECT id, object_type, title, summary, content,
                              tags, relationships, provider, instance_url,
                              original_id, web_url, source_metadata,
                              updated_at, synced_at, checksum
                       FROM knowledge_objects WHERE 1=1".to_string();

        let mut param_index = 1;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ot) = object_type {
            sql.push_str(&format!(" AND object_type = ?{}", param_index));
            params_vec.push(Box::new(ot.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            sql.push_str(&format!(" AND tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT ?{}", param_index));
        params_vec.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(rusqlite_params.as_slice(), Self::row_to_object)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_stats(&self) -> Result<StorageStats> {
        let conn = self.get_connection()?;

        let total_objects: usize = conn.query_row(
            "SELECT COUNT(*) FROM knowledge_objects",
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
            total_objects,
            connectors_count,
            db_size_bytes,
        })
    }

    fn row_to_object(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeObject> {
        let id: String = row.get(0)?;
        let ot_str: String = row.get(1)?;
        let title: String = row.get(2)?;
        let summary: Option<String> = row.get(3)?;
        let content: String = row.get(4)?;
        let tags_json: String = row.get(5)?;
        let rels_json: String = row.get(6)?;
        let provider: String = row.get(7)?;
        let instance_url: String = row.get(8)?;
        let original_id: String = row.get(9)?;
        let web_url: String = row.get(10)?;
        let meta_json: String = row.get(11)?;
        let updated_str: String = row.get(12)?;
        let synced_str: String = row.get(13)?;
        let checksum: String = row.get(14)?;

        let object_type = ot_str.parse().unwrap_or(ObjectType::Document);
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let relationships: Vec<Relationship> =
            serde_json::from_str(&rels_json).unwrap_or_default();
        let source_metadata: serde_json::Value =
            serde_json::from_str(&meta_json).unwrap_or(serde_json::Value::Null);

        let updated_at = DateTime::parse_from_rfc3339(&updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let synced_at = DateTime::parse_from_rfc3339(&synced_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(KnowledgeObject {
            id,
            object_type,
            title,
            summary,
            content,
            tags,
            relationships,
            source: SourceInfo {
                provider,
                instance_url,
                original_id,
                web_url,
            },
            source_metadata,
            updated_at,
            synced_at,
            checksum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_storage_init_and_fts_search() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let storage = Storage::new(temp_file.path())?;

        let now = Utc::now();
        let obj = KnowledgeObject {
            id: "test-id-1".to_string(),
            object_type: ObjectType::Ticket,
            title: "Implement Payment Gateway Gateway API".to_string(),
            summary: Some("Status: In Progress".to_string()),
            content: "Integration with Stripe and PayPal APIs for processing checkout".to_string(),
            tags: vec!["payment".to_string(), "backend".to_string()],
            relationships: vec![],
            source: SourceInfo {
                provider: "jira".to_string(),
                instance_url: "https://acme.atlassian.net".to_string(),
                original_id: "PAY-101".to_string(),
                web_url: "https://acme.atlassian.net/browse/PAY-101".to_string(),
            },
            source_metadata: serde_json::json!({"priority": "High"}),
            updated_at: now,
            synced_at: now,
            checksum: "sha256-checksum-1".to_string(),
        };

        storage.upsert_object(&obj)?;

        // Verify checksum lookup
        let cs = storage.get_existing_checksum("test-id-1")?;
        assert_eq!(cs, Some("sha256-checksum-1".to_string()));

        // Verify FTS5 search
        let search_results = storage.search_fts("Stripe", 10)?;
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].title, "Implement Payment Gateway Gateway API");

        // Verify structured query
        let query_results = storage.query_structured(Some("ticket"), Some("payment"), 10)?;
        assert_eq!(query_results.len(), 1);
        assert_eq!(query_results[0].source.original_id, "PAY-101");

        // Verify stats
        let stats = storage.get_stats()?;
        assert_eq!(stats.total_objects, 1);

        Ok(())
    }
}

