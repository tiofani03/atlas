use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

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

    async fn load_spec(&self, path_or_url: &str) -> Result<Value> {
        if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
            let res = self.client.get(path_or_url).send().await?.text().await?;
            serde_json::from_str::<Value>(&res)
                .with_context(|| format!("Failed to parse JSON spec from URL: {}", path_or_url))
        } else {
            let content = std::fs::read_to_string(path_or_url)
                .with_context(|| format!("Failed to read OpenAPI spec at {}", path_or_url))?;
            serde_json::from_str::<Value>(&content)
                .with_context(|| format!("Failed to parse JSON spec from file: {}", path_or_url))
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

    async fn fetch_modified(&self, _since: Option<chrono::DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let paths = self.config.get_paths();

        for spec_loc in &paths {
            let spec_val = match self.load_spec(spec_loc).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Warning: failed to load openapi spec {}: {}", spec_loc, e);
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
    async fn test_openapi_parse_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("spec.json");
        let spec_json = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Petstore", "version": "1.0.0" },
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

        let conn = OpenapiConnector::new("openapi-local".to_string(), cfg).unwrap();
        let artifacts = conn.fetch_modified(None).await.unwrap();

        assert!(!artifacts.is_empty());
        assert!(artifacts.iter().any(|a| a.title.contains("GET /pets")));
    }
}
