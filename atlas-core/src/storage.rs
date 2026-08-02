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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ArtifactHeader {
    pub id: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub provider: String,
    pub source_id: String,
    pub source_url: String,
    pub repository: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
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

            CREATE INDEX IF NOT EXISTS idx_ka_source_id ON knowledge_artifacts(source_id);
            CREATE INDEX IF NOT EXISTS idx_ka_provider_source ON knowledge_artifacts(provider, source_id);
            CREATE INDEX IF NOT EXISTS idx_ka_kind ON knowledge_artifacts(kind);
            CREATE INDEX IF NOT EXISTS idx_ka_repository ON knowledge_artifacts(repository);
            CREATE INDEX IF NOT EXISTS idx_ka_updated_at ON knowledge_artifacts(updated_at);
            CREATE INDEX IF NOT EXISTS idx_ka_kind_updated ON knowledge_artifacts(kind, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_ka_repo_kind ON knowledge_artifacts(repository, kind);
            CREATE INDEX IF NOT EXISTS idx_ka_repo_updated ON knowledge_artifacts(repository, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_ka_provider_updated ON knowledge_artifacts(provider, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_ka_kind_source ON knowledge_artifacts(kind, source_id);

            CREATE TABLE IF NOT EXISTS artifact_relationships (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id, relationship_type)
            );

            CREATE INDEX IF NOT EXISTS idx_rel_source ON artifact_relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_rel_target ON artifact_relationships(target_id);
            CREATE INDEX IF NOT EXISTS idx_rel_src_type ON artifact_relationships(source_id, relationship_type);
            CREATE INDEX IF NOT EXISTS idx_rel_tgt_type ON artifact_relationships(target_id, relationship_type);

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

            CREATE TABLE IF NOT EXISTS git_index_commits (
                sha TEXT PRIMARY KEY NOT NULL,
                repository TEXT NOT NULL,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                authored_at TEXT NOT NULL,
                message TEXT NOT NULL,
                is_merge INTEGER NOT NULL DEFAULT 0,
                parents TEXT NOT NULL,
                patch_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_gic_repo_authored ON git_index_commits(repository, authored_at DESC);
            CREATE INDEX IF NOT EXISTS idx_gic_author_email ON git_index_commits(author_email);

            CREATE TABLE IF NOT EXISTS commit_files (
                commit_sha TEXT NOT NULL,
                repository TEXT NOT NULL,
                file_path TEXT NOT NULL,
                change_type TEXT NOT NULL,
                insertions INTEGER NOT NULL DEFAULT 0,
                deletions INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (commit_sha, file_path)
            );

            CREATE INDEX IF NOT EXISTS idx_cf_file_repo ON commit_files(repository, file_path);
            CREATE INDEX IF NOT EXISTS idx_cf_commit ON commit_files(commit_sha);

            CREATE TABLE IF NOT EXISTS git_ref_watermarks (
                connector_id TEXT NOT NULL,
                ref_name TEXT NOT NULL,
                commit_sha TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (connector_id, ref_name)
            );

            DROP TRIGGER IF EXISTS ka_ad;
            DROP TRIGGER IF EXISTS ka_au;

            CREATE TRIGGER IF NOT EXISTS ka_ai AFTER INSERT ON knowledge_artifacts BEGIN
                INSERT INTO knowledge_fts(id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.body, new.tags, COALESCE(new.repository, ''), new.kind, new.provider, new.source_id);
            END;

            CREATE TRIGGER ka_ad AFTER DELETE ON knowledge_artifacts BEGIN
                DELETE FROM knowledge_fts WHERE id = old.id;
            END;

            CREATE TRIGGER ka_au AFTER UPDATE ON knowledge_artifacts BEGIN
                DELETE FROM knowledge_fts WHERE id = old.id;
                INSERT INTO knowledge_fts(id, title, summary, body, tags, repository, kind, provider, source_id)
                VALUES (new.id, new.title, COALESCE(new.summary, ''), new.body, new.tags, COALESCE(new.repository, ''), new.kind, new.provider, new.source_id);
            END;
            ",
        )?;
        Ok(())
    }

    pub fn clear_all_data(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute_batch(
            "DROP TABLE IF EXISTS knowledge_objects;
             DELETE FROM knowledge_artifacts;
             DELETE FROM artifact_relationships;
             DELETE FROM connectors_state;
             DELETE FROM knowledge_fts;
             DELETE FROM git_ref_watermarks;
             DELETE FROM git_index_commits;
             DELETE FROM commit_files;
             PRAGMA wal_checkpoint(TRUNCATE);
             VACUUM;
             PRAGMA wal_checkpoint(TRUNCATE);",
        )?;
        Ok(())
    }

    /// Delete all synchronized knowledge artifacts, relationships, watermarks, and state for a specific connector ID
    pub fn clear_connector_data(
        &self,
        connector_id: &str,
        provider: Option<&str>,
        repos: &[String],
    ) -> Result<usize> {
        let conn = self.get_connection()?;

        conn.execute(
            "DELETE FROM connectors_state WHERE connector_id = ?1",
            params![connector_id],
        )?;
        conn.execute(
            "DELETE FROM git_ref_watermarks WHERE connector_id = ?1",
            params![connector_id],
        )?;

        let prov_str = provider.unwrap_or(connector_id);
        let prefix = format!("{}:", connector_id);

        let mut query = String::from(
            "DELETE FROM knowledge_artifacts WHERE provider = ?1 OR provider = ?2 OR source_id LIKE ?3 || '%'"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(connector_id.to_string()),
            Box::new(prov_str.to_string()),
            Box::new(prefix),
        ];

        if !repos.is_empty() {
            query.push_str(" OR repository IN (");
            for (idx, r) in repos.iter().enumerate() {
                if idx > 0 {
                    query.push_str(", ");
                }
                query.push_str(&format!("?{}", params_vec.len() + 1));
                params_vec.push(Box::new(r.clone()));
            }
            query.push(')');
        }

        let deleted_count = conn.execute(&query, rusqlite::params_from_iter(params_vec))?;

        // Cleanup orphaned relationships, commits, and commit_files
        let _ = conn.execute(
            "DELETE FROM artifact_relationships 
             WHERE source_id NOT IN (SELECT id FROM knowledge_artifacts) 
               AND source_id NOT IN (SELECT source_id FROM knowledge_artifacts)",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM artifact_relationships 
             WHERE target_id NOT IN (SELECT id FROM knowledge_artifacts) 
               AND target_id NOT IN (SELECT source_id FROM knowledge_artifacts)",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM git_index_commits 
             WHERE sha NOT IN (SELECT source_id FROM knowledge_artifacts WHERE kind = 'commit')",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM commit_files 
             WHERE commit_sha NOT IN (SELECT source_id FROM knowledge_artifacts WHERE kind = 'commit')",
            [],
        );

        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        Ok(deleted_count)
    }

    pub fn get_ref_watermark(&self, connector_id: &str, ref_name: &str) -> Result<Option<String>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT commit_sha FROM git_ref_watermarks WHERE connector_id = ?1 AND ref_name = ?2")?;
        let sha: Option<String> = stmt.query_row(params![connector_id, ref_name], |row| row.get(0)).optional()?;
        Ok(sha)
    }

    pub fn update_ref_watermark(&self, connector_id: &str, ref_name: &str, commit_sha: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO git_ref_watermarks (connector_id, ref_name, commit_sha, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(connector_id, ref_name) DO UPDATE SET
                 commit_sha = excluded.commit_sha,
                 updated_at = excluded.updated_at",
            params![connector_id, ref_name, commit_sha, Utc::now().to_rfc3339()],
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

    pub fn upsert_artifacts_batch(&self, artifacts: &[KnowledgeArtifact]) -> Result<(usize, usize, usize)> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let mut inserted = 0;
        let mut updated = 0;
        let mut skipped = 0;

        {
            let mut check_stmt = tx.prepare_cached("SELECT checksum FROM knowledge_artifacts WHERE id = ?1")?;
            let mut insert_ka_stmt = tx.prepare_cached(
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
                    metadata = excluded.metadata",
            )?;

            let mut del_rel_stmt = tx.prepare_cached(
                "DELETE FROM artifact_relationships WHERE (source_id = ?1 OR source_id = ?2) AND relationship_type != 'released_in'",
            )?;
            let mut del_rel_release_stmt = tx.prepare_cached(
                "DELETE FROM artifact_relationships WHERE source_id = ?1 OR source_id = ?2",
            )?;
            let mut ins_rel_stmt = tx.prepare_cached(
                "INSERT INTO artifact_relationships (source_id, target_id, relationship_type)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(source_id, target_id, relationship_type) DO NOTHING",
            )?;
            let mut ins_commit_stmt = tx.prepare_cached(
                "INSERT INTO git_index_commits (
                    sha, repository, author_name, author_email, authored_at, message, is_merge, parents, patch_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(sha) DO UPDATE SET
                    repository = excluded.repository,
                    author_name = excluded.author_name,
                    author_email = excluded.author_email,
                    authored_at = excluded.authored_at,
                    message = excluded.message,
                    is_merge = excluded.is_merge,
                    parents = excluded.parents,
                    patch_id = excluded.patch_id",
            )?;
            let mut ins_commit_file_stmt = tx.prepare_cached(
                "INSERT INTO commit_files (
                    commit_sha, repository, file_path, change_type, insertions, deletions
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(commit_sha, file_path) DO UPDATE SET
                    change_type = excluded.change_type,
                    insertions = excluded.insertions,
                    deletions = excluded.deletions",
            )?;

            for artifact in artifacts {
                let existing_checksum: Option<String> = check_stmt
                    .query_row(params![&artifact.id], |row| row.get(0))
                    .optional()?;

                if let Some(ref cs) = existing_checksum {
                    if cs == &artifact.checksum {
                        skipped += 1;
                        continue;
                    }
                    updated += 1;
                } else {
                    inserted += 1;
                }

                let tags_json = serde_json::to_string(&artifact.tags)?;
                let rels_json = serde_json::to_string(&artifact.relationships)?;
                let meta_json = serde_json::to_string(&artifact.metadata)?;
                let created_at_str = artifact.created_at.map(|dt| dt.to_rfc3339());

                insert_ka_stmt.execute(params![
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
                ])?;

                if artifact.kind != ArtifactKind::Release {
                    del_rel_stmt.execute(params![artifact.id, artifact.source_id])?;
                } else {
                    del_rel_release_stmt.execute(params![artifact.id, artifact.source_id])?;
                }

                let auto_rels = Self::extract_automatic_linking_relationships(artifact);

                for rel in artifact.relationships.iter().chain(auto_rels.iter()) {
                    if rel.source_id != rel.target_id {
                        ins_rel_stmt.execute(params![rel.source_id, rel.target_id, rel.relationship_type])?;
                    }
                }

                if artifact.kind == ArtifactKind::Commit {
                    let sha = &artifact.source_id;
                    let repo = artifact.repository.as_deref().unwrap_or("");
                    let author_name = artifact.metadata.get("author_name").and_then(|v| v.as_str()).unwrap_or("");
                    let author_email = artifact.metadata.get("author_email").and_then(|v| v.as_str()).unwrap_or("");
                    let authored_at = artifact.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
                    let message = &artifact.title;
                    let is_merge = if artifact.metadata.get("is_merge").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 };
                    let parents_str = artifact.metadata.get("parents").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
                    let patch_id = artifact.metadata.get("patch_id").and_then(|v| v.as_str());

                    ins_commit_stmt.execute(params![sha, repo, author_name, author_email, authored_at, message, is_merge, parents_str, patch_id])?;

                    if let Some(files) = artifact.metadata.get("files").and_then(|v| v.as_array()) {
                        for f in files {
                            let file_path = f.get("filename").or_else(|| f.get("path")).and_then(|v| v.as_str()).unwrap_or("");
                            if !file_path.is_empty() {
                                let change_type = f.get("status").and_then(|v| v.as_str()).unwrap_or("MODIFIED");
                                let additions = f.get("additions").and_then(|v| v.as_i64()).unwrap_or(0);
                                let deletions = f.get("deletions").and_then(|v| v.as_i64()).unwrap_or(0);

                                ins_commit_file_stmt.execute(params![sha, repo, file_path, change_type, additions, deletions])?;
                            }
                        }
                    }
                }
            }
        }

        tx.commit()?;
        Ok((inserted, updated, skipped))
    }

    pub fn upsert_artifact(&self, artifact: &KnowledgeArtifact) -> Result<()> {
        let _ = self.upsert_artifacts_batch(std::slice::from_ref(artifact))?;
        Ok(())
    }

    pub fn extract_automatic_linking_relationships(artifact: &KnowledgeArtifact) -> Vec<ArtifactRelationship> {
        use std::collections::HashSet;
        let mut rels = Vec::new();
        let mut seen = HashSet::new();

        let full_text = format!("{} {}", artifact.title, artifact.body);

        let mut add_edge = |src: String, tgt: String, r_type: String, rev_type: String| {
            if !src.is_empty() && !tgt.is_empty() && src != tgt {
                if seen.insert((src.clone(), tgt.clone(), r_type.clone())) {
                    rels.push(ArtifactRelationship {
                        source_id: src.clone(),
                        target_id: tgt.clone(),
                        relationship_type: r_type,
                    });
                }
                if seen.insert((tgt.clone(), src.clone(), rev_type.clone())) {
                    rels.push(ArtifactRelationship {
                        source_id: tgt,
                        target_id: src,
                        relationship_type: rev_type,
                    });
                }
            }
        };

        // 1. Ticket Key Regex (e.g. INIT-488, PAY-123)
        if let Ok(ticket_re) = regex::Regex::new("([A-Z][A-Z0-9]{1,9}-[0-9]+)") {
            for cap in ticket_re.captures_iter(&full_text) {
                if let Some(m) = cap.get(1) {
                    let ticket_key = m.as_str().to_string();
                    if ticket_key != artifact.source_id && ticket_key != artifact.id {
                        let (rel_type, rev_type) = if artifact.kind == ArtifactKind::Commit
                            || artifact.kind == ArtifactKind::PullRequest
                        {
                            ("implements".to_string(), "implemented_by".to_string())
                        } else {
                            ("references".to_string(), "referenced_by".to_string())
                        };

                        add_edge(artifact.source_id.clone(), ticket_key, rel_type, rev_type);
                    }
                }
            }
        }

        // 1b. ClickUp Task Key Regex (e.g. CU-123, ClickUp#92, ClickUp #91)
        if let Ok(cu_re) = regex::Regex::new(r"(?i)\b(?:CU-|ClickUp\s*#?)([A-Za-z0-9]+)\b") {
            for cap in cu_re.captures_iter(&full_text) {
                if let Some(m) = cap.get(1) {
                    let task_key = format!("CU-{}", m.as_str());
                    let raw_key = m.as_str().to_string();
                    if task_key != artifact.source_id && raw_key != artifact.source_id && task_key != artifact.id {
                        let (rel_type, rev_type) = if artifact.kind == ArtifactKind::Commit
                            || artifact.kind == ArtifactKind::PullRequest
                        {
                            ("implements".to_string(), "implemented_by".to_string())
                        } else {
                            ("references".to_string(), "referenced_by".to_string())
                        };

                        add_edge(artifact.source_id.clone(), task_key.clone(), rel_type.clone(), rev_type.clone());
                        add_edge(artifact.source_id.clone(), raw_key, rel_type, rev_type);
                    }
                }
            }
        }

        // 2. PR Number Regex (e.g. #307 or Merge pull request #148 or /pull/23 - exact integer boundaries)
        if let Ok(pr_re) = regex::Regex::new(r"(?:\B#|\b#|/pull/)([0-9]+)\b") {
            if let Some(repo) = &artifact.repository {
                for cap in pr_re.captures_iter(&full_text) {
                    if let Some(m) = cap.get(1) {
                        if let Ok(pr_num) = m.as_str().parse::<u64>() {
                            let target_pr_id = format!("{}#{}", repo, pr_num);
                            if target_pr_id != artifact.source_id && target_pr_id != artifact.id {
                                let (rel_type, rev_type) = if artifact.kind == ArtifactKind::Commit {
                                    ("merged_into".to_string(), "contains".to_string())
                                } else if artifact.kind == ArtifactKind::Release {
                                    ("contains".to_string(), "released_in".to_string())
                                } else {
                                    ("references".to_string(), "referenced_by".to_string())
                                };

                                add_edge(artifact.source_id.clone(), target_pr_id, rel_type, rev_type);
                            }
                        }
                    }
                }
            }
        }

        // 3. Release Tag & Release Target Linking
        if artifact.kind == ArtifactKind::Release {
            if let Some(target_sha) = artifact
                .metadata
                .get("target_commitish")
                .or_else(|| artifact.metadata.get("commit_sha"))
                .and_then(|v| v.as_str())
            {
                if !target_sha.is_empty() {
                    add_edge(
                        artifact.source_id.clone(),
                        target_sha.to_string(),
                        "contains".to_string(),
                        "released_in".to_string(),
                    );
                    if let Some(repo) = &artifact.repository {
                        if !target_sha.contains('@') {
                            let repo_commit_id = format!("{}@{}", repo, target_sha);
                            add_edge(
                                artifact.source_id.clone(),
                                repo_commit_id,
                                "contains".to_string(),
                                "released_in".to_string(),
                            );
                        }
                    }
                }
            }
        }

        rels
    }

    pub fn get_commits_for_ticket(&self, ticket_id: &str) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider, ka.source_id,
                    ka.source_url, ka.repository, ka.tags, ka.relationships, ka.created_at,
                    ka.updated_at, ka.synced_at, ka.checksum, ka.metadata
             FROM knowledge_artifacts ka
             JOIN artifact_relationships rel ON rel.source_id = ka.id OR rel.source_id = ka.source_id
             WHERE (rel.target_id = ?1) AND (rel.relationship_type = 'implements' OR rel.relationship_type = 'references')
               AND ka.kind = 'commit'
             ORDER BY ka.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![ticket_id], Self::row_to_artifact)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_commits_for_pr(&self, pr_id: &str) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider, ka.source_id,
                    ka.source_url, ka.repository, ka.tags, ka.relationships, ka.created_at,
                    ka.updated_at, ka.synced_at, ka.checksum, ka.metadata
             FROM knowledge_artifacts ka
             JOIN artifact_relationships rel ON rel.source_id = ka.id OR rel.source_id = ka.source_id
             WHERE (rel.target_id = ?1) AND (rel.relationship_type = 'merged_into' OR rel.relationship_type = 'part_of')
               AND ka.kind = 'commit'
             ORDER BY ka.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![pr_id], Self::row_to_artifact)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_commits_for_release(&self, release_id: &str) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider, ka.source_id,
                    ka.source_url, ka.repository, ka.tags, ka.relationships, ka.created_at,
                    ka.updated_at, ka.synced_at, ka.checksum, ka.metadata
             FROM knowledge_artifacts ka
             JOIN artifact_relationships rel ON rel.source_id = ka.id OR rel.source_id = ka.source_id
             WHERE (rel.target_id = ?1) AND rel.relationship_type = 'released_in'
               AND ka.kind = 'commit'
             ORDER BY ka.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![release_id], Self::row_to_artifact)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_releases_for_commit(&self, commit_sha: &str) -> Result<Vec<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider, ka.source_id,
                    ka.source_url, ka.repository, ka.tags, ka.relationships, ka.created_at,
                    ka.updated_at, ka.synced_at, ka.checksum, ka.metadata
             FROM knowledge_artifacts ka
             JOIN artifact_relationships rel ON rel.target_id = ka.id OR rel.target_id = ka.source_id
             WHERE (rel.source_id = ?1) AND rel.relationship_type = 'released_in'
               AND ka.kind = 'release'
             ORDER BY ka.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![commit_sha], Self::row_to_artifact)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_file_hotspots(&self, repo: &str, limit: usize) -> Result<Vec<(String, usize, usize, usize)>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT file_path, COUNT(commit_sha) as mod_count, SUM(insertions) as total_add, SUM(deletions) as total_del
             FROM commit_files
             WHERE repository = ?1
             GROUP BY file_path
             ORDER BY mod_count DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![repo, limit as i64], |row| {
            let path: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let add: i64 = row.get(2)?;
            let del: i64 = row.get(3)?;
            Ok((path, count as usize, add as usize, del as usize))
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_commit_file_count(&self, commit_sha: &str) -> Result<usize> {
        let conn = self.get_connection()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM commit_files WHERE commit_sha = ?1 OR commit_sha LIKE '%' || ?1",
                params![commit_sha],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok(count as usize)
    }

    pub fn get_artifact_by_id(&self, id_or_source_id: &str) -> Result<Option<KnowledgeArtifact>> {
        let conn = self.get_connection()?;

        // Fast path 1: exact match on primary key (id)
        let mut stmt_id = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts
             WHERE id = ?1",
        )?;
        if let Some(a) = stmt_id.query_row(params![id_or_source_id], Self::row_to_artifact).optional()? {
            return Ok(Some(a));
        }

        // Fast path 2: exact match on source_id (indexed)
        let mut stmt_src = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts
             WHERE source_id = ?1",
        )?;
        if let Some(a) = stmt_src.query_row(params![id_or_source_id], Self::row_to_artifact).optional()? {
            return Ok(Some(a));
        }

        // Case-insensitive fallback only for short, plausible identifiers
        if !id_or_source_id.contains(' ') && id_or_source_id.len() <= 128 {
            let lower_key = id_or_source_id.to_lowercase();
            let mut stmt_lower = conn.prepare(
                "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                        repository, tags, relationships, created_at, updated_at, synced_at,
                        checksum, metadata
                 FROM knowledge_artifacts
                 WHERE source_id = ?1 LIMIT 1",
            )?;
            if let Some(a) = stmt_lower.query_row(params![lower_key], Self::row_to_artifact).optional()? {
                return Ok(Some(a));
            }
        }

        Ok(None)
    }

    /// Resolve a Pull Request by repository name and PR number by exact equality
    pub fn resolve_pr(&self, repository: &str, number: u64) -> Result<Option<KnowledgeArtifact>> {
        let conn = self.get_connection()?;
        let exact_source_id = format!("{}#{}", repository, number);

        let mut stmt = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts
             WHERE kind = 'pull_request'
               AND repository = ?1
               AND (source_id = ?2 OR json_extract(metadata, '$.number') = ?3)",
        )?;
        let obj = stmt
            .query_row(
                params![repository, exact_source_id, number],
                Self::row_to_artifact,
            )
            .optional()?;

        Ok(obj)
    }

    pub fn parse_pr_number_from_source_id(source_id: &str) -> Option<u64> {
        let s = if let Some(pos) = source_id.rfind('#') {
            &source_id[pos + 1..]
        } else {
            source_id
        };
        let num_str = s.split('/').next().unwrap_or(s);
        num_str.parse::<u64>().ok()
    }

    pub fn parse_pr_alias(alias: &str) -> Option<(Option<String>, u64)> {
        let clean = alias.trim();
        if clean.is_empty() {
            return None;
        }

        if let Some(hash_pos) = clean.rfind('#') {
            let prefix = clean[..hash_pos].trim();
            let suffix = clean[hash_pos + 1..].trim();
            if let Ok(num) = suffix.parse::<u64>() {
                let repo_opt = if prefix.is_empty() || prefix.eq_ignore_ascii_case("pr") {
                    None
                } else if prefix.contains('/') {
                    Some(prefix.to_string())
                } else {
                    None
                };
                return Some((repo_opt, num));
            }
        }

        if let Some(pull_pos) = clean.find("/pull/") {
            let repo = &clean[..pull_pos];
            let suffix = &clean[pull_pos + "/pull/".len()..];
            if let Ok(num) = suffix.parse::<u64>() {
                if repo.contains('/') {
                    return Some((Some(repo.to_string()), num));
                }
            }
        }

        let lower = clean.to_lowercase();
        let num_str = if lower.starts_with("pr") {
            lower
                .trim_start_matches("pr")
                .trim_start_matches('#')
                .trim_start_matches('-')
                .trim_start_matches(':')
                .trim_start_matches('/')
        } else if lower.starts_with('#') {
            lower.trim_start_matches('#')
        } else if lower.chars().all(|c| c.is_ascii_digit()) {
            &lower
        } else {
            ""
        };

        if !num_str.is_empty() {
            if let Ok(num) = num_str.parse::<u64>() {
                return Some((None, num));
            }
        }

        None
    }

    pub fn detect_current_repository_context() -> Option<String> {
        if let Ok(repo) = crate::connectors::local_git::LocalGitRepository::open(".") {
            if let Some(ref origin_url) = repo.git_origin_url {
                if let Some(pos) = origin_url.rfind(':') {
                    let path = &origin_url[pos + 1..];
                    let clean = path.trim_end_matches(".git");
                    if clean.contains('/') {
                        return Some(clean.to_string());
                    }
                } else if let Some(pos) = origin_url.rfind('/') {
                    let repo_part = origin_url[pos + 1..].trim_end_matches(".git");
                    if let Some(prev_pos) = origin_url[..pos].rfind('/') {
                        let owner_part = &origin_url[prev_pos + 1..pos];
                        return Some(format!("{}/{}", owner_part, repo_part));
                    }
                }
            }
            return Some(repo.name);
        }
        None
    }

    pub fn resolve_artifact_by_alias(&self, alias: &str) -> Result<Vec<KnowledgeArtifact>> {
        let clean = alias.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Direct exact lookup by id or source_id
        if let Some(art) = self.get_artifact_by_id(clean)? {
            return Ok(vec![art]);
        }

        // 2. PR Resolution: Fully-qualified repo + PR number, or alias with repo context / unique PR
        if let Some((repo_opt, pr_num)) = Self::parse_pr_alias(clean) {
            if let Some(ref repo) = repo_opt {
                if let Some(pr_art) = self.resolve_pr(repo, pr_num)? {
                    return Ok(vec![pr_art]);
                }
            } else {
                // Try current repo context first
                if let Some(ctx_repo) = Self::detect_current_repository_context() {
                    if let Some(pr_art) = self.resolve_pr(&ctx_repo, pr_num)? {
                        return Ok(vec![pr_art]);
                    }
                }

                // Query DB for exact PR number across all repositories
                let conn = self.get_connection()?;
                let exact_suffix = format!("#{}", pr_num);
                let mut stmt = conn.prepare(
                    "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                            repository, tags, relationships, created_at, updated_at, synced_at,
                            checksum, metadata
                     FROM knowledge_artifacts
                     WHERE kind = 'pull_request'
                       AND (source_id LIKE '%' || ?1 OR source_id = ?1 OR json_extract(metadata, '$.number') = ?2)",
                )?;
                let rows = stmt.query_map(params![exact_suffix, pr_num], Self::row_to_artifact)?;
                let mut pr_matches = Vec::new();
                for r in rows {
                    let art = r?;
                    if let Some(meta) = art.pull_request_metadata() {
                        if meta.number == pr_num {
                            pr_matches.push(art);
                        }
                    } else if Self::parse_pr_number_from_source_id(&art.source_id) == Some(pr_num) {
                        pr_matches.push(art);
                    }
                }

                if !pr_matches.is_empty() {
                    return Ok(pr_matches);
                }
            }
        }

        let conn = self.get_connection()?;
        let mut matches = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // 3. Commit Alias Matching (SHA hex like d18bdfb or owner/repo@sha)
        let sha_target = if clean.contains('@') {
            clean.split('@').nth(1).unwrap_or(clean)
        } else {
            clean
        };

        if sha_target.len() >= 6 && sha_target.chars().all(|c| c.is_ascii_hexdigit()) {
            let mut stmt = conn.prepare(
                "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                        repository, tags, relationships, created_at, updated_at, synced_at,
                        checksum, metadata
                 FROM knowledge_artifacts
                 WHERE kind = 'commit'
                   AND (source_id LIKE '%@' || ?1 || '%' OR source_id LIKE '%' || ?1 || '%' OR id LIKE ?1 || '%')",
            )?;
            let rows = stmt.query_map(params![sha_target], Self::row_to_artifact)?;
            for r in rows {
                let art = r?;
                if seen_ids.insert(art.id.clone()) {
                    matches.push(art);
                }
            }
        }


        // 4. Exact ticket/source_id fallback lookup (NO loose substring matching for numbers)
        if matches.is_empty() && !clean.chars().all(|c| c.is_ascii_digit()) {
            let mut stmt = conn.prepare(
                "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                        repository, tags, relationships, created_at, updated_at, synced_at,
                        checksum, metadata
                 FROM knowledge_artifacts
                 WHERE source_id = ?1 OR LOWER(source_id) = LOWER(?1) OR title = ?1",
            )?;
            let rows = stmt.query_map(params![clean], Self::row_to_artifact)?;
            for r in rows {
                let art = r?;
                if seen_ids.insert(art.id.clone()) {
                    matches.push(art);
                }
            }
        }

        // 5. ClickUp Task Alias Fallback Resolution
        if matches.is_empty() {
            let cu_query = if clean.to_lowercase().starts_with("cu-") {
                clean[3..].to_string()
            } else if clean.to_lowercase().starts_with("clickup#") {
                clean[8..].to_string()
            } else {
                clean.to_string()
            };

            let formatted_cu = format!("CU-{}", cu_query);

            let mut stmt = conn.prepare(
                "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                        repository, tags, relationships, created_at, updated_at, synced_at,
                        checksum, metadata
                 FROM knowledge_artifacts
                 WHERE provider = 'clickup'
                   AND (source_id = ?1 OR source_id = ?2 OR LOWER(source_id) = LOWER(?1) OR json_extract(metadata, '$.id') = ?1 OR json_extract(metadata, '$.custom_id') = ?1)",
            )?;
            let rows = stmt.query_map(params![cu_query, formatted_cu], Self::row_to_artifact)?;
            for r in rows {
                let art = r?;
                if seen_ids.insert(art.id.clone()) {
                    matches.push(art);
                }
            }
        }

        Ok(matches)
    }

    pub fn rebuild_all_relationships(&self) -> Result<(usize, usize)> {
        let conn = self.get_connection()?;

        // Clear stale automatic and manual relationships before rebuilding
        conn.execute("DELETE FROM artifact_relationships", [])?;

        let mut stmt = conn.prepare(
            "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                    repository, tags, relationships, created_at, updated_at, synced_at,
                    checksum, metadata
             FROM knowledge_artifacts",
        )?;

        let rows = stmt.query_map([], Self::row_to_artifact)?;
        let mut artifacts = Vec::new();
        for r in rows {
            artifacts.push(r?);
        }

        let total_artifacts = artifacts.len();
        let mut total_relationships = 0;

        for art in &artifacts {
            let auto_rels = Self::extract_automatic_linking_relationships(art);
            for rel in art.relationships.iter().chain(auto_rels.iter()) {
                if rel.source_id != rel.target_id {
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO artifact_relationships (source_id, target_id, relationship_type)
                         VALUES (?1, ?2, ?3)",
                        params![rel.source_id, rel.target_id, rel.relationship_type],
                    );
                    total_relationships += 1;
                }
            }

            if art.kind == ArtifactKind::Commit {
                let sha = &art.source_id;
                let repo = art.repository.as_deref().unwrap_or("");
                let author_name = art.metadata.get("author_name").and_then(|v| v.as_str()).unwrap_or("");
                let author_email = art.metadata.get("author_email").and_then(|v| v.as_str()).unwrap_or("");
                let authored_at = art.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
                let message = &art.title;
                let is_merge = if art.metadata.get("is_merge").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 };
                let parents_str = art.metadata.get("parents").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
                let patch_id = art.metadata.get("patch_id").and_then(|v| v.as_str());

                let _ = conn.execute(
                    "INSERT INTO git_index_commits (
                        sha, repository, author_name, author_email, authored_at, message, is_merge, parents, patch_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(sha) DO UPDATE SET
                        repository = excluded.repository,
                        author_name = excluded.author_name,
                        author_email = excluded.author_email,
                        authored_at = excluded.authored_at,
                        message = excluded.message,
                        is_merge = excluded.is_merge,
                        parents = excluded.parents,
                        patch_id = excluded.patch_id
                    ",
                    params![sha, repo, author_name, author_email, authored_at, message, is_merge, parents_str, patch_id],
                );

                if let Some(files) = art.metadata.get("files").and_then(|v| v.as_array()) {
                    for f in files {
                        let file_path = f.get("filename").or_else(|| f.get("path")).and_then(|v| v.as_str()).unwrap_or("");
                        if !file_path.is_empty() {
                            let change_type = f.get("status").and_then(|v| v.as_str()).unwrap_or("MODIFIED");
                            let additions = f.get("additions").and_then(|v| v.as_i64()).unwrap_or(0);
                            let deletions = f.get("deletions").and_then(|v| v.as_i64()).unwrap_or(0);

                            let _ = conn.execute(
                                "INSERT INTO commit_files (
                                    commit_sha, repository, file_path, change_type, insertions, deletions
                                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                                ON CONFLICT(commit_sha, file_path) DO UPDATE SET
                                    change_type = excluded.change_type,
                                    insertions = excluded.insertions,
                                    deletions = excluded.deletions
                                ",
                                params![sha, repo, file_path, change_type, additions, deletions],
                            );
                        }
                    }
                }
            }
        }

        Ok((total_artifacts, total_relationships))
    }

    pub fn inverse_relationship_type(rel_type: &str) -> String {
        match rel_type.to_lowercase().as_str() {
            "parent_commit" => "child_commit".to_string(),
            "child_commit" => "parent_commit".to_string(),
            "implements" => "implemented_by".to_string(),
            "implemented_by" => "implements".to_string(),
            "merged_into" => "contains".to_string(),
            "contains" => "merged_into".to_string(),
            "belongs_to" => "owns".to_string(),
            "owns" => "belongs_to".to_string(),
            "references" => "referenced_by".to_string(),
            "referenced_by" => "references".to_string(),
            "released_in" => "contains_release".to_string(),
            "contains_release" => "released_in".to_string(),
            other => {
                if let Some(base) = other.strip_suffix("_by") {
                    base.to_string()
                } else {
                    format!("{}_by", other)
                }
            }
        }
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
             WHERE (source_id = ?1 OR source_id = ?2 OR target_id = ?1 OR target_id = ?2)
               AND source_id != target_id",
        )?;

        let rel_rows = stmt.query_map(params![id_key, source_key], |row| {
            Ok(ArtifactRelationship {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relationship_type: row.get(2)?,
            })
        })?;

        let mut results = Vec::new();
        let mut seen_targets = std::collections::HashSet::new();

        for rel in rel_rows {
            let rel = rel?;
            let is_outgoing = rel.source_id == id_key || rel.source_id == source_key;
            let other_key = if is_outgoing {
                &rel.target_id
            } else {
                &rel.source_id
            };

            if other_key == &id_key || other_key == &source_key {
                continue;
            }

            if let Some(other_artifact) = self.get_artifact_by_id(other_key)? {
                let target_key = if !other_artifact.source_id.is_empty() {
                    other_artifact.source_id.clone()
                } else {
                    other_artifact.id.clone()
                };

                if seen_targets.insert(target_key) {
                    let effective_rel = if is_outgoing {
                        rel
                    } else {
                        ArtifactRelationship {
                            source_id: source_key.clone(),
                            target_id: other_artifact.source_id.clone(),
                            relationship_type: Self::inverse_relationship_type(&rel.relationship_type),
                        }
                    };

                    results.push((effective_rel, other_artifact));
                }
            }
        }

        Ok(results)
    }

    pub fn validate_graph_integrity(&self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;
        let mut issues = Vec::new();

        // 1. Check for self-parent or self-target edges
        let mut stmt = conn.prepare(
            "SELECT source_id, relationship_type FROM artifact_relationships WHERE source_id = target_id",
        )?;
        let self_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for r in self_rows {
            let (src, r_type) = r?;
            issues.push(format!("Self-relationship detected: {} -> {} ({})", src, src, r_type));
        }

        // 2. Check for duplicate commit artifacts with conflicting titles
        let mut stmt = conn.prepare(
            "SELECT source_id, COUNT(*) FROM knowledge_artifacts WHERE kind = 'commit' GROUP BY source_id HAVING COUNT(*) > 1",
        )?;
        let dup_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut dups = Vec::new();
        for r in dup_rows {
            dups.push(r?);
        }

        for (src, count) in dups {
            let mut title_stmt = conn.prepare(
                "SELECT title FROM knowledge_artifacts WHERE kind = 'commit' AND source_id = ?1",
            )?;
            let titles_vec: Vec<String> = title_stmt
                .query_map(params![src], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
            issues.push(format!("Duplicate commit source_id (count {}): {} with titles: {}", count, src, titles_vec.join(" || ")));
        }

        Ok(issues)
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
        let (items, _) = self.search_fts_paginated(query, kind, None, tag, repository, limit, 0)?;
        Ok(items)
    }

    pub fn search_fts_paginated(
        &self,
        query: &str,
        kind: Option<&str>,
        provider: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<KnowledgeArtifact>, usize)> {
        let conn = self.get_connection()?;
        let clean_query = query.trim().replace('"', "");
        let formatted_query = if clean_query.is_empty() {
            "*".to_string()
        } else if clean_query.contains('-')
            || clean_query.contains(':')
            || clean_query.contains('/')
            || clean_query.contains('\\')
            || clean_query.contains(' ')
        {
            format!("\"{}\"", clean_query)
        } else if clean_query.ends_with('*') {
            clean_query.to_string()
        } else {
            format!("{}*", clean_query)
        };

        let mut where_sql = "FROM knowledge_fts fts
                             JOIN knowledge_artifacts ka ON ka.id = fts.id
                             WHERE fts.knowledge_fts MATCH ?1".to_string();

        let mut param_index = 2;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(formatted_query));

        if let Some(k) = kind {
            where_sql.push_str(&format!(" AND ka.kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(p) = provider {
            where_sql.push_str(&format!(" AND ka.provider = ?{}", param_index));
            params_vec.push(Box::new(p.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            where_sql.push_str(&format!(" AND ka.tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        if let Some(repo) = repository {
            where_sql.push_str(&format!(" AND ka.repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        // Get total count matching criteria
        let count_sql = format!("SELECT COUNT(*) {}", where_sql);
        let mut count_stmt = match conn.prepare(&count_sql) {
            Ok(s) => s,
            Err(_) => return self.search_like_fallback(query, kind, provider, tag, repository, limit, offset),
        };

        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let total: usize = match count_stmt.query_row(rusqlite_params.as_slice(), |row| row.get(0)) {
            Ok(t) => t,
            Err(_) => return self.search_like_fallback(query, kind, provider, tag, repository, limit, offset),
        };

        // Data query
        let select_sql = format!(
            "SELECT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider,
                    ka.source_id, ka.source_url, ka.repository, ka.tags,
                    ka.relationships, ka.created_at, ka.updated_at, ka.synced_at,
                    ka.checksum, ka.metadata
             {} ORDER BY ka.updated_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, param_index, param_index + 1
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let mut stmt = match conn.prepare(&select_sql) {
            Ok(s) => s,
            Err(_) => return self.search_like_fallback(query, kind, provider, tag, repository, limit, offset),
        };
        let rusqlite_params_data: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = match stmt.query_map(rusqlite_params_data.as_slice(), Self::row_to_artifact) {
            Ok(r) => r,
            Err(_) => return self.search_like_fallback(query, kind, provider, tag, repository, limit, offset),
        };

        let mut results = Vec::new();
        for r in rows {
            if let Ok(art) = r {
                results.push(art);
            }
        }
        Ok((results, total))
    }

    /// Fallback search using pure SQL LIKE queries when FTS5 query syntax fails
    pub fn search_like_fallback(
        &self,
        query: &str,
        kind: Option<&str>,
        provider: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<KnowledgeArtifact>, usize)> {
        let conn = self.get_connection()?;
        let clean = query.trim().replace('"', "");
        let like_query = format!("%{}%", clean);

        let mut where_sql = "FROM knowledge_artifacts ka
                             WHERE (ka.title LIKE ?1 OR ka.source_id LIKE ?1 OR ka.body LIKE ?1)".to_string();

        let mut param_index = 2;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(like_query));

        if let Some(k) = kind {
            where_sql.push_str(&format!(" AND ka.kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(p) = provider {
            where_sql.push_str(&format!(" AND ka.provider = ?{}", param_index));
            params_vec.push(Box::new(p.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            where_sql.push_str(&format!(" AND ka.tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        if let Some(repo) = repository {
            where_sql.push_str(&format!(" AND ka.repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        let count_sql = format!("SELECT COUNT(*) {}", where_sql);
        let mut count_stmt = conn.prepare(&count_sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let total: usize = count_stmt.query_row(rusqlite_params.as_slice(), |row| row.get(0)).unwrap_or(0);

        let select_sql = format!(
            "SELECT ka.id, ka.kind, ka.title, ka.summary, ka.body, ka.provider,
                    ka.source_id, ka.source_url, ka.repository, ka.tags,
                    ka.relationships, ka.created_at, ka.updated_at, ka.synced_at,
                    ka.checksum, ka.metadata
             {} ORDER BY ka.updated_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, param_index, param_index + 1
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let mut stmt = conn.prepare(&select_sql)?;
        let rusqlite_params_data: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(rusqlite_params_data.as_slice(), Self::row_to_artifact)?;

        let mut results = Vec::new();
        for r in rows {
            if let Ok(art) = r {
                results.push(art);
            }
        }
        Ok((results, total))
    }

    pub fn query_structured(
        &self,
        kind: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KnowledgeArtifact>> {
        let (items, _) = self.query_structured_paginated(kind, None, tag, repository, limit, 0)?;
        Ok(items)
    }

    pub fn query_structured_paginated(
        &self,
        kind: Option<&str>,
        provider: Option<&str>,
        tag: Option<&str>,
        repository: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<KnowledgeArtifact>, usize)> {
        let conn = self.get_connection()?;
        let mut where_sql = "FROM knowledge_artifacts WHERE 1=1".to_string();

        let mut param_index = 1;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(k) = kind {
            where_sql.push_str(&format!(" AND kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(p) = provider {
            where_sql.push_str(&format!(" AND provider = ?{}", param_index));
            params_vec.push(Box::new(p.to_string()));
            param_index += 1;
        }

        if let Some(t) = tag {
            where_sql.push_str(&format!(" AND tags LIKE ?{}", param_index));
            params_vec.push(Box::new(format!("%\"{}\"%", t)));
            param_index += 1;
        }

        if let Some(repo) = repository {
            where_sql.push_str(&format!(" AND repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        // Get total count matching criteria
        let count_sql = format!("SELECT COUNT(*) {}", where_sql);
        let mut count_stmt = conn.prepare(&count_sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();
        let total: usize = count_stmt.query_row(rusqlite_params.as_slice(), |row| row.get(0))?;

        // Data query
        let select_sql = format!(
            "SELECT id, kind, title, summary, body, provider, source_id,
                    source_url, repository, tags, relationships, created_at,
                    updated_at, synced_at, checksum, metadata
             {} ORDER BY updated_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, param_index, param_index + 1
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let mut stmt = conn.prepare(&select_sql)?;
        let rusqlite_params_data: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(rusqlite_params_data.as_slice(), Self::row_to_artifact)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok((results, total))
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

        let mut db_size_bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        let wal_path = self.path.with_extension("db-wal");
        if let Ok(m) = fs::metadata(&wal_path) {
            db_size_bytes += m.len();
        }
        let shm_path = self.path.with_extension("db-shm");
        if let Ok(m) = fs::metadata(&shm_path) {
            db_size_bytes += m.len();
        }

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

    pub fn get_artifact_header_by_id(&self, id_or_source_id: &str) -> Result<Option<ArtifactHeader>> {
        let conn = self.get_connection()?;

        // Fast path 1: exact match on primary key (id)
        let mut stmt_id = conn.prepare(
            "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
             FROM knowledge_artifacts
             WHERE id = ?1",
        )?;
        if let Some(h) = stmt_id.query_row(params![id_or_source_id], Self::row_to_header).optional()? {
            return Ok(Some(h));
        }

        // Fast path 2: exact match on source_id (indexed)
        let mut stmt_src = conn.prepare(
            "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
             FROM knowledge_artifacts
             WHERE source_id = ?1",
        )?;
        if let Some(h) = stmt_src.query_row(params![id_or_source_id], Self::row_to_header).optional()? {
            return Ok(Some(h));
        }

        // Case-insensitive fallback only for short, plausible identifiers (not search terms)
        // Skip for strings containing spaces (multi-word search terms) to avoid full table scan
        if !id_or_source_id.contains(' ') && id_or_source_id.len() <= 128 {
            let lower_key = id_or_source_id.to_lowercase();
            let mut stmt_src_lower = conn.prepare(
                "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
                 FROM knowledge_artifacts
                 WHERE source_id = ?1 LIMIT 1",
            )?;
            if let Some(h) = stmt_src_lower.query_row(params![lower_key], Self::row_to_header).optional()? {
                return Ok(Some(h));
            }
        }

        Ok(None)
    }

    pub fn get_artifact_headers_by_ids(&self, ids: &[String]) -> Result<Vec<ArtifactHeader>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.get_connection()?;
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for chunk in ids.chunks(500) {
            let mut query1 = String::from(
                "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
                 FROM knowledge_artifacts
                 WHERE id IN ("
            );
            let mut params1: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for (idx, id) in chunk.iter().enumerate() {
                if idx > 0 {
                    query1.push_str(", ");
                }
                query1.push_str(&format!("?{}", idx + 1));
                params1.push(Box::new(id.clone()));
            }
            query1.push(')');

            let mut stmt1 = conn.prepare(&query1)?;
            let rusqlite_params1: Vec<&dyn rusqlite::ToSql> = params1.iter().map(|b| b.as_ref()).collect();
            let rows1 = stmt1.query_map(rusqlite_params1.as_slice(), Self::row_to_header)?;
            for r in rows1 {
                let h = r?;
                if seen_ids.insert(h.id.clone()) {
                    results.push(h);
                }
            }

            let mut query2 = String::from(
                "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
                 FROM knowledge_artifacts
                 WHERE source_id IN ("
            );
            let mut params2: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for (idx, id) in chunk.iter().enumerate() {
                if idx > 0 {
                    query2.push_str(", ");
                }
                query2.push_str(&format!("?{}", idx + 1));
                params2.push(Box::new(id.clone()));
            }
            query2.push(')');

            let mut stmt2 = conn.prepare(&query2)?;
            let rusqlite_params2: Vec<&dyn rusqlite::ToSql> = params2.iter().map(|b| b.as_ref()).collect();
            let rows2 = stmt2.query_map(rusqlite_params2.as_slice(), Self::row_to_header)?;
            for r in rows2 {
                let h = r?;
                if seen_ids.insert(h.id.clone()) {
                    results.push(h);
                }
            }
        }

        Ok(results)
    }

    pub fn get_artifacts_by_ids(&self, ids: &[String]) -> Result<Vec<KnowledgeArtifact>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.get_connection()?;
        let mut results = Vec::new();

        for chunk in ids.chunks(500) {
            let mut query = String::from(
                "SELECT id, kind, title, summary, body, provider, source_id, source_url,
                        repository, tags, relationships, created_at, updated_at, synced_at,
                        checksum, metadata
                 FROM knowledge_artifacts
                 WHERE id IN ("
            );
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for (idx, id) in chunk.iter().enumerate() {
                if idx > 0 {
                    query.push_str(", ");
                }
                query.push_str(&format!("?{}", idx + 1));
                params_vec.push(Box::new(id.clone()));
            }
            query.push(')');

            let mut stmt = conn.prepare(&query)?;
            let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            let rows = stmt.query_map(rusqlite_params.as_slice(), Self::row_to_artifact)?;
            for r in rows {
                results.push(r?);
            }
        }

        Ok(results)
    }

    pub fn get_related_headers(
        &self,
        id_or_source_id: &str,
    ) -> Result<Vec<(ArtifactRelationship, ArtifactHeader)>> {
        let conn = self.get_connection()?;

        let key_header = self.get_artifact_header_by_id(id_or_source_id)?;
        let (id_key, source_key) = match key_header {
            Some(ref a) => (a.id.clone(), a.source_id.clone()),
            None => (id_or_source_id.to_string(), id_or_source_id.to_string()),
        };

        let mut stmt1 = conn.prepare(
            "SELECT source_id, target_id, relationship_type
             FROM artifact_relationships
             WHERE source_id IN (?1, ?2) AND source_id != target_id",
        )?;
        let rel_rows1 = stmt1.query_map(params![id_key, source_key], |row| {
            Ok(ArtifactRelationship {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relationship_type: row.get(2)?,
            })
        })?;

        let mut stmt2 = conn.prepare(
            "SELECT source_id, target_id, relationship_type
             FROM artifact_relationships
             WHERE target_id IN (?1, ?2) AND source_id != target_id",
        )?;
        let rel_rows2 = stmt2.query_map(params![id_key, source_key], |row| {
            Ok(ArtifactRelationship {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relationship_type: row.get(2)?,
            })
        })?;

        let mut relationships = Vec::new();
        let mut keys_to_fetch = Vec::new();

        for rel in rel_rows1.chain(rel_rows2) {
            let rel = rel?;
            let is_outgoing = rel.source_id == id_key || rel.source_id == source_key;
            let other_key = if is_outgoing {
                rel.target_id.clone()
            } else {
                rel.source_id.clone()
            };

            if other_key == id_key || other_key == source_key {
                continue;
            }

            keys_to_fetch.push(other_key.clone());
            relationships.push((rel, is_outgoing, other_key));
        }

        let headers = self.get_artifact_headers_by_ids(&keys_to_fetch)?;
        let mut header_map: std::collections::HashMap<String, ArtifactHeader> = std::collections::HashMap::new();
        for h in headers {
            header_map.insert(h.id.clone(), h.clone());
            header_map.insert(h.source_id.clone(), h);
        }

        let mut results = Vec::new();
        let mut seen_targets = std::collections::HashSet::new();

        for (rel, is_outgoing, other_key) in relationships {
            if let Some(other_header) = header_map.get(&other_key) {
                let target_key = if !other_header.source_id.is_empty() {
                    other_header.source_id.clone()
                } else {
                    other_header.id.clone()
                };

                if seen_targets.insert(target_key) {
                    let effective_rel = if is_outgoing {
                        rel
                    } else {
                        ArtifactRelationship {
                            source_id: source_key.clone(),
                            target_id: other_header.source_id.clone(),
                            relationship_type: Self::inverse_relationship_type(&rel.relationship_type),
                        }
                    };

                    results.push((effective_rel, other_header.clone()));
                }
            }
        }

        Ok(results)
    }

    pub fn get_batch_related_headers(
        &self,
        id_keys: &[String],
    ) -> Result<Vec<(ArtifactRelationship, ArtifactHeader)>> {
        if id_keys.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.get_connection()?;
        let key_set: std::collections::HashSet<String> = id_keys.iter().cloned().collect();
        let mut raw_rels = Vec::new();
        let mut keys_to_fetch = Vec::new();

        for chunk in id_keys.chunks(500) {
            let mut q1 = String::from(
                "SELECT source_id, target_id, relationship_type
                 FROM artifact_relationships
                 WHERE source_id IN ("
            );
            let mut p1: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for (idx, id) in chunk.iter().enumerate() {
                if idx > 0 {
                    q1.push_str(", ");
                }
                q1.push_str(&format!("?{}", idx + 1));
                p1.push(Box::new(id.clone()));
            }
            q1.push(')');

            let mut stmt1 = conn.prepare(&q1)?;
            let params_ref1: Vec<&dyn rusqlite::ToSql> = p1.iter().map(|b| b.as_ref()).collect();
            let rows1 = stmt1.query_map(params_ref1.as_slice(), |row| {
                Ok(ArtifactRelationship {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relationship_type: row.get(2)?,
                })
            })?;
            for r in rows1 {
                raw_rels.push(r?);
            }

            let mut q2 = String::from(
                "SELECT source_id, target_id, relationship_type
                 FROM artifact_relationships
                 WHERE target_id IN ("
            );
            let mut p2: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for (idx, id) in chunk.iter().enumerate() {
                if idx > 0 {
                    q2.push_str(", ");
                }
                q2.push_str(&format!("?{}", idx + 1));
                p2.push(Box::new(id.clone()));
            }
            q2.push(')');

            let mut stmt2 = conn.prepare(&q2)?;
            let params_ref2: Vec<&dyn rusqlite::ToSql> = p2.iter().map(|b| b.as_ref()).collect();
            let rows2 = stmt2.query_map(params_ref2.as_slice(), |row| {
                Ok(ArtifactRelationship {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relationship_type: row.get(2)?,
                })
            })?;
            for r in rows2 {
                raw_rels.push(r?);
            }
        }

        let mut processed_rels = Vec::new();
        for rel in raw_rels {
            let is_outgoing = key_set.contains(&rel.source_id);
            let other_key = if is_outgoing {
                rel.target_id.clone()
            } else {
                rel.source_id.clone()
            };

            if key_set.contains(&other_key) {
                continue;
            }

            keys_to_fetch.push(other_key.clone());
            processed_rels.push((rel, is_outgoing, other_key));
        }

        let headers = self.get_artifact_headers_by_ids(&keys_to_fetch)?;
        let mut header_map: std::collections::HashMap<String, ArtifactHeader> = std::collections::HashMap::new();
        for h in headers {
            header_map.insert(h.id.clone(), h.clone());
            header_map.insert(h.source_id.clone(), h);
        }

        let mut results = Vec::new();
        let mut seen_targets = std::collections::HashSet::new();

        for (rel, is_outgoing, other_key) in processed_rels {
            if let Some(other_header) = header_map.get(&other_key) {
                let target_key = if !other_header.source_id.is_empty() {
                    other_header.source_id.clone()
                } else {
                    other_header.id.clone()
                };

                if seen_targets.insert(target_key) {
                    let effective_rel = if is_outgoing {
                        rel
                    } else {
                        ArtifactRelationship {
                            source_id: rel.target_id.clone(),
                            target_id: other_header.source_id.clone(),
                            relationship_type: Self::inverse_relationship_type(&rel.relationship_type),
                        }
                    };

                    results.push((effective_rel, other_header.clone()));
                }
            }
        }

        Ok(results)
    }

    pub fn query_headers_by_repository(
        &self,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<ArtifactHeader>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, kind, title, provider, source_id, source_url, repository, updated_at, created_at, metadata
             FROM knowledge_artifacts
             WHERE repository = ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![repo, limit as i64], Self::row_to_header)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn search_fts_headers(
        &self,
        query: &str,
        kind: Option<&str>,
        repository: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ArtifactHeader>> {
        let conn = self.get_connection()?;
        let clean_query = query.trim().replace('"', "");
        if clean_query.is_empty() {
            return Ok(Vec::new());
        }

        let formatted_query = if clean_query.contains('-')
            || clean_query.contains(':')
            || clean_query.contains('/')
            || clean_query.contains('\\')
            || clean_query.contains(' ')
        {
            format!("\"{}\"", clean_query)
        } else if clean_query.ends_with('*') {
            clean_query.to_string()
        } else {
            format!("{}*", clean_query)
        };

        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Only attempt exact ID lookup if query looks like a single identifier (no spaces)
        if !clean_query.contains(' ') {
            if let Ok(Some(h)) = self.get_artifact_header_by_id(&clean_query) {
                let matches_kind = kind.map_or(true, |k| h.kind.to_string().eq_ignore_ascii_case(k));
                let matches_repo = repository.map_or(true, |r| h.repository.as_deref() == Some(r));
                if matches_kind && matches_repo {
                    seen_ids.insert(h.id.clone());
                    results.push(h);
                }
            }
        }

        let mut fts_sql = String::from(
            "SELECT ka.id, ka.kind, ka.title, ka.provider, ka.source_id, ka.source_url,
                    ka.repository, ka.updated_at, ka.created_at, ka.metadata
             FROM knowledge_fts fts
             JOIN knowledge_artifacts ka ON ka.id = fts.id
             WHERE fts.knowledge_fts MATCH ?1"
        );

        let mut param_index = 2;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(formatted_query));

        if let Some(k) = kind {
            fts_sql.push_str(&format!(" AND ka.kind = ?{}", param_index));
            params_vec.push(Box::new(k.to_string()));
            param_index += 1;
        }

        if let Some(repo) = repository {
            fts_sql.push_str(&format!(" AND ka.repository = ?{}", param_index));
            params_vec.push(Box::new(repo.to_string()));
            param_index += 1;
        }

        fts_sql.push_str(&format!(" ORDER BY ka.updated_at DESC LIMIT ?{}", param_index));
        params_vec.push(Box::new(limit as i64));

        if let Ok(mut stmt) = conn.prepare(&fts_sql) {
            let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            if let Ok(rows) = stmt.query_map(rusqlite_params.as_slice(), Self::row_to_header) {
                for r in rows {
                    if let Ok(h) = r {
                        if seen_ids.insert(h.id.clone()) {
                            results.push(h);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn row_to_header(row: &rusqlite::Row) -> rusqlite::Result<ArtifactHeader> {
        let id: String = row.get(0)?;
        let kind_str: String = row.get(1)?;
        let title: String = row.get(2)?;
        let provider: String = row.get(3)?;
        let source_id: String = row.get(4)?;
        let source_url: String = row.get(5)?;
        let repository: Option<String> = row.get(6)?;
        let updated_str: String = row.get(7)?;
        let created_str: Option<String> = row.get(8)?;
        let meta_json: String = row.get(9)?;

        let kind = ArtifactKind::from_str(&kind_str).unwrap_or(ArtifactKind::Document);
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

        Ok(ArtifactHeader {
            id,
            kind,
            title,
            provider,
            source_id,
            source_url,
            repository,
            updated_at,
            created_at,
            metadata,
        })
    }
}

