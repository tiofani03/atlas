use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::Path;

pub struct OpenapiConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl OpenapiConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let client = reqwest::Client::builder().build()?;
        Ok(Self { id, config, client })
    }

    /// Load specification content from a URL or local file, attempting JSON and YAML parsing
    pub async fn load_spec(&self, path_or_url: &str) -> Result<Value> {
        let raw_content = if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
            self.client
                .get(path_or_url)
                .send()
                .await
                .with_context(|| format!("Failed to fetch OpenAPI spec from URL: {}", path_or_url))?
                .text()
                .await
                .with_context(|| format!("Failed to read text from URL: {}", path_or_url))?
        } else {
            std::fs::read_to_string(path_or_url)
                .with_context(|| format!("Failed to read OpenAPI spec at local file path: {}", path_or_url))?
        };

        // 1. Try parsing as JSON first
        if let Ok(json_val) = serde_json::from_str::<Value>(&raw_content) {
            return Ok(json_val);
        }

        // 2. Fallback to parsing as YAML
        match serde_yaml::from_str::<Value>(&raw_content) {
            Ok(yaml_val) => Ok(yaml_val),
            Err(yaml_err) => {
                bail!(
                    "Failed to parse OpenAPI spec from '{}' as either JSON or YAML: {}",
                    path_or_url,
                    yaml_err
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl Connector for OpenapiConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "openapi"
    }

    async fn verify(&self) -> Result<String> {
        let paths = self.config.get_paths();
        if paths.is_empty() {
            bail!("No OpenAPI spec paths or URLs configured for connector '{}'", self.id);
        }

        let mut verified_count = 0;
        for path in &paths {
            let spec_val = self.load_spec(path).await.with_context(|| format!("Verification failed for spec at '{}'", path))?;
            let title = spec_val["info"]["title"].as_str().unwrap_or("OpenAPI Specification");
            let version = spec_val["info"]["version"].as_str().unwrap_or("1.0.0");
            verified_count += 1;
            tracing::info!("Verified OpenAPI spec: {} (v{}) at {}", title, version, path);
        }

        Ok(format!("Successfully verified {} OpenAPI specification(s).", verified_count))
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let paths = self.config.get_paths();

        for spec_loc in &paths {
            // Check local file mtime for incremental skip
            if !spec_loc.starts_with("http://") && !spec_loc.starts_with("https://") {
                if let Some(since_dt) = since {
                    let path_buf = Path::new(spec_loc);
                    if let Ok(metadata) = path_buf.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let mtime: DateTime<Utc> = modified.into();
                            if mtime < since_dt {
                                continue;
                            }
                        }
                    }
                }
            }

            let spec_val = match self.load_spec(spec_loc).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Warning: failed to load openapi spec {}: {}", spec_loc, e);
                    continue;
                }
            };

            let title = spec_val["info"]["title"].as_str().unwrap_or("OpenAPI Specification").to_string();
            let version = spec_val["info"]["version"].as_str().unwrap_or("1.0.0").to_string();
            let description = spec_val["info"]["description"].as_str().unwrap_or("");
            let api_source_id = format!("{}:{}", title, version);

            let canonical_id = KnowledgeArtifact::generate_id("openapi", spec_loc, &api_source_id);
            let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &[]);

            let api_artifact = KnowledgeArtifact {
                id: canonical_id.clone(),
                kind: ArtifactKind::Specification,
                title: format!("{} (v{})", title, version),
                summary: None,
                body: description.to_string(),
                provider: "openapi".to_string(),
                source_id: api_source_id.clone(),
                source_url: spec_loc.clone(),
                repository: None,
                tags: vec!["openapi".to_string(), format!("version:{}", version)],
                relationships: Vec::new(),
                created_at: None,
                updated_at: Utc::now(),
                synced_at: Utc::now(),
                checksum,
                metadata: spec_val.clone(),
            };
            artifacts.push(api_artifact);

            // Parse endpoints under "paths"
            if let Some(paths_obj) = spec_val["paths"].as_object() {
                for (path, methods) in paths_obj {
                    if let Some(methods_obj) = methods.as_object() {
                        for (method, op_val) in methods_obj {
                            let method_upper = method.to_uppercase();
                            if !["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"].contains(&method_upper.as_str()) {
                                continue;
                            }

                            let summary = op_val["summary"].as_str().unwrap_or("");
                            let op_desc = op_val["description"].as_str().unwrap_or("");
                            let endpoint_title = format!("{} {}", method_upper, path);
                            let endpoint_source_id = format!("{} {}", method_upper, path);
                            let endpoint_url = format!("{}#{}", spec_loc, endpoint_source_id.replace(' ', "-"));

                            let ep_canonical_id = KnowledgeArtifact::generate_id("openapi", spec_loc, &endpoint_source_id);
                            let ep_checksum = KnowledgeArtifact::compute_checksum(&endpoint_title, Some(summary), op_desc, &[]);

                            let mut ep_relationships = vec![ArtifactRelationship {
                                source_id: endpoint_source_id.clone(),
                                target_id: api_source_id.clone(),
                                relationship_type: "part_of_spec".to_string(),
                            }];

                            if let Some(tags) = op_val["tags"].as_array() {
                                for tag in tags {
                                    if let Some(t_str) = tag.as_str() {
                                        ep_relationships.push(ArtifactRelationship {
                                            source_id: endpoint_source_id.clone(),
                                            target_id: t_str.to_string(),
                                            relationship_type: "tagged_by".to_string(),
                                        });
                                    }
                                }
                            }

                            artifacts.push(KnowledgeArtifact {
                                id: ep_canonical_id,
                                kind: ArtifactKind::Component,
                                title: endpoint_title,
                                summary: if summary.is_empty() { None } else { Some(summary.to_string()) },
                                body: format!("## Summary\n{}\n\n## Description\n{}", summary, op_desc),
                                provider: "openapi".to_string(),
                                source_id: endpoint_source_id,
                                source_url: endpoint_url,
                                repository: None,
                                tags: vec!["openapi:endpoint".to_string(), method_upper.clone()],
                                relationships: ep_relationships,
                                created_at: None,
                                updated_at: Utc::now(),
                                synced_at: Utc::now(),
                                checksum: ep_checksum,
                                metadata: op_val.clone(),
                            });
                        }
                    }
                }
            }

            // Parse Component Schemas under components.schemas or definitions
            let schemas_opt = spec_val["components"]["schemas"]
                .as_object()
                .or_else(|| spec_val["definitions"].as_object());

            if let Some(schemas) = schemas_opt {
                for (schema_name, schema_val) in schemas {
                    let schema_source_id = format!("schema:{}", schema_name);
                    let schema_title = format!("Schema: {}", schema_name);
                    let schema_desc = schema_val["description"].as_str().unwrap_or("");

                    let sch_canonical_id = KnowledgeArtifact::generate_id("openapi", spec_loc, &schema_source_id);
                    let sch_checksum = KnowledgeArtifact::compute_checksum(&schema_title, None, schema_desc, &[]);

                    artifacts.push(KnowledgeArtifact {
                        id: sch_canonical_id,
                        kind: ArtifactKind::Component,
                        title: schema_title,
                        summary: None,
                        body: format!("```json\n{}\n```", serde_json::to_string_pretty(schema_val).unwrap_or_default()),
                        provider: "openapi".to_string(),
                        source_id: schema_source_id,
                        source_url: format!("{}#/components/schemas/{}", spec_loc, schema_name),
                        repository: None,
                        tags: vec!["openapi:schema".to_string()],
                        relationships: vec![ArtifactRelationship {
                            source_id: format!("schema:{}", schema_name),
                            target_id: api_source_id.clone(),
                            relationship_type: "defined_in".to_string(),
                        }],
                        created_at: None,
                        updated_at: Utc::now(),
                        synced_at: Utc::now(),
                        checksum: sch_checksum,
                        metadata: schema_val.clone(),
                    });
                }
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_openapi_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "openapi".to_string();
        let conn = OpenapiConnector::new("openapi-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "openapi-test");
        assert_eq!(conn.provider(), "openapi");
    }

    #[tokio::test]
    async fn test_openapi_parse_local_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("spec.json");
        let spec_json = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Petstore JSON", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": { "summary": "List pets", "description": "Returns pets" }
                }
            }
        });
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(spec_json.to_string().as_bytes()).unwrap();

        let mut cfg = ConnectorConfig::default();
        cfg.provider = "openapi".to_string();
        cfg.path = Some(file_path.to_str().unwrap().to_string());

        let conn = OpenapiConnector::new("openapi-local-json".to_string(), cfg).unwrap();
        let artifacts = conn.fetch_modified(None).await.unwrap();

        assert!(!artifacts.is_empty());
        assert!(artifacts.iter().any(|a| a.title.contains("GET /pets")));
    }

    #[tokio::test]
    async fn test_openapi_parse_local_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("spec.yaml");
        let spec_yaml = r#"
openapi: 3.0.0
info:
  title: Users Service API
  version: 2.1.0
  description: Authentication and User Management API
paths:
  /users/{id}:
    get:
      summary: Get user by ID
      description: Returns user profile
    delete:
      summary: Delete user
      description: Deactivates account
components:
  schemas:
    UserProfile:
      type: object
      properties:
        id:
          type: string
        email:
          type: string
"#;
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(spec_yaml.as_bytes()).unwrap();

        let mut cfg = ConnectorConfig::default();
        cfg.provider = "openapi".to_string();
        cfg.path = Some(file_path.to_str().unwrap().to_string());

        let conn = OpenapiConnector::new("openapi-local-yaml".to_string(), cfg).unwrap();
        let artifacts = conn.fetch_modified(None).await.unwrap();

        assert!(!artifacts.is_empty());
        assert!(artifacts.iter().any(|a| a.title.contains("GET /users/{id}")));
        assert!(artifacts.iter().any(|a| a.title.contains("DELETE /users/{id}")));
        assert!(artifacts.iter().any(|a| a.title == "Schema: UserProfile"));
    }
}
