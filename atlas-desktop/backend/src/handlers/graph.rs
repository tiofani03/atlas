use crate::state::AppState;
use atlas_core::storage::Storage;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNodeDto {
    pub id: String,
    pub source_id: String,
    pub kind: String,
    pub title: String,
    pub provider: String,
    pub source_url: String,
    pub repository: Option<String>,
    pub summary: Option<String>,
    pub is_root: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GraphEdgeDto {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphResponse {
    pub root_id: String,
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQueryParams {
    pub depth: Option<usize>,
    pub limit: Option<usize>,
}

pub fn fetch_subgraph(
    storage: &Storage,
    root_id_or_source_id: &str,
    depth: usize,
    limit: usize,
) -> anyhow::Result<GraphResponse> {
    let root_header = storage
        .get_artifact_header_by_id(root_id_or_source_id)?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", root_id_or_source_id))?;

    let canonical_root_id = root_header.id.clone();
    let root_summary = storage
        .get_artifact_by_id(&canonical_root_id)?
        .and_then(|a| a.summary);

    let mut nodes_map: HashMap<String, GraphNodeDto> = HashMap::new();
    let mut alias_map: HashMap<String, String> = HashMap::new();
    let mut edges_set: HashSet<GraphEdgeDto> = HashSet::new();

    alias_map.insert(root_header.id.clone(), root_header.id.clone());
    alias_map.insert(root_header.source_id.clone(), root_header.id.clone());

    nodes_map.insert(
        root_header.id.clone(),
        GraphNodeDto {
            id: root_header.id.clone(),
            source_id: root_header.source_id.clone(),
            kind: root_header.kind.to_string(),
            title: root_header.title.clone(),
            provider: root_header.provider.clone(),
            source_url: root_header.source_url.clone(),
            repository: root_header.repository.clone(),
            summary: root_summary,
            is_root: true,
            metadata: root_header.metadata.clone(),
        },
    );

    let hop1_relations = storage.get_related_headers(&canonical_root_id)?;
    let mut hop1_ids = Vec::new();

    for (rel, header) in hop1_relations {
        if nodes_map.len() >= limit {
            break;
        }

        alias_map.insert(header.id.clone(), header.id.clone());
        alias_map.insert(header.source_id.clone(), header.id.clone());

        if !nodes_map.contains_key(&header.id) {
            hop1_ids.push(header.id.clone());
            nodes_map.insert(
                header.id.clone(),
                GraphNodeDto {
                    id: header.id.clone(),
                    source_id: header.source_id.clone(),
                    kind: header.kind.to_string(),
                    title: header.title.clone(),
                    provider: header.provider.clone(),
                    source_url: header.source_url.clone(),
                    repository: header.repository.clone(),
                    summary: None,
                    is_root: false,
                    metadata: header.metadata.clone(),
                },
            );
        }

        let src = alias_map
            .get(&rel.source_id)
            .cloned()
            .unwrap_or(rel.source_id);
        let tgt = alias_map
            .get(&rel.target_id)
            .cloned()
            .unwrap_or(rel.target_id);

        if src != tgt {
            edges_set.insert(GraphEdgeDto {
                id: format!("e-{}-{}-{}", src, rel.relationship_type, tgt),
                source: src,
                target: tgt,
                label: rel.relationship_type,
            });
        }
    }

    if depth >= 2 && !hop1_ids.is_empty() && nodes_map.len() < limit {
        if let Ok(hop2_relations) = storage.get_batch_related_headers(&hop1_ids) {
            for (rel, header) in hop2_relations {
                if nodes_map.len() >= limit {
                    break;
                }

                alias_map.insert(header.id.clone(), header.id.clone());
                alias_map.insert(header.source_id.clone(), header.id.clone());

                if !nodes_map.contains_key(&header.id) {
                    nodes_map.insert(
                        header.id.clone(),
                        GraphNodeDto {
                            id: header.id.clone(),
                            source_id: header.source_id.clone(),
                            kind: header.kind.to_string(),
                            title: header.title.clone(),
                            provider: header.provider.clone(),
                            source_url: header.source_url.clone(),
                            repository: header.repository.clone(),
                            summary: None,
                            is_root: false,
                            metadata: header.metadata.clone(),
                        },
                    );
                }

                let src = alias_map
                    .get(&rel.source_id)
                    .cloned()
                    .unwrap_or(rel.source_id);
                let tgt = alias_map
                    .get(&rel.target_id)
                    .cloned()
                    .unwrap_or(rel.target_id);

                if src != tgt && nodes_map.contains_key(&src) && nodes_map.contains_key(&tgt) {
                    edges_set.insert(GraphEdgeDto {
                        id: format!("e-{}-{}-{}", src, rel.relationship_type, tgt),
                        source: src,
                        target: tgt,
                        label: rel.relationship_type,
                    });
                }
            }
        }
    }

    let mut nodes: Vec<GraphNodeDto> = nodes_map.into_values().collect();
    nodes.sort_by(|a, b| {
        if a.is_root != b.is_root {
            b.is_root.cmp(&a.is_root)
        } else {
            a.title.cmp(&b.title)
        }
    });

    let mut edges: Vec<GraphEdgeDto> = edges_set.into_iter().collect();
    edges.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(GraphResponse {
        root_id: canonical_root_id,
        nodes,
        edges,
    })
}

pub async fn get_graph_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<GraphQueryParams>,
) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    let depth = params.depth.unwrap_or(1).clamp(1, 3);
    let limit = params.limit.unwrap_or(100).clamp(1, 500);

    match fetch_subgraph(&storage, &id, depth, limit) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!(resp))),
        Err(err) => {
            let err_msg = err.to_string();
            if err_msg.contains("Artifact not found") {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": err_msg })),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": err_msg })),
                )
            }
        }
    }
}

pub async fn get_recent_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    match storage.query_structured_paginated(None, None, None, None, 12, 0) {
        Ok((artifacts, _total)) => {
            let nodes: Vec<GraphNodeDto> = artifacts
                .into_iter()
                .map(|a| GraphNodeDto {
                    id: a.id.clone(),
                    source_id: a.source_id.clone(),
                    kind: a.kind.to_string(),
                    title: a.title,
                    provider: a.provider,
                    source_url: a.source_url,
                    repository: a.repository,
                    summary: a.summary,
                    is_root: false,
                    metadata: a.metadata,
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!(nodes)))
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::domain::{ArtifactKind, KnowledgeArtifact};
    use atlas_core::storage::Storage;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_empty_graph_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        let res = fetch_subgraph(&storage, "nonexistent-id", 1, 50);
        assert!(res.is_err());
    }

    #[test]
    fn test_graph_traversal_1_hop() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        let a1 = KnowledgeArtifact {
            id: "jira:PAY-100".to_string(),
            kind: ArtifactKind::Ticket,
            title: "Global Retry Engine".to_string(),
            summary: Some("Summary".to_string()),
            body: "Body".to_string(),
            provider: "jira".to_string(),
            source_id: "PAY-100".to_string(),
            source_url: "https://jira.com/PAY-100".to_string(),
            repository: None,
            tags: vec!["payment".to_string()],
            relationships: vec![atlas_core::domain::ArtifactRelationship {
                source_id: "PAY-100".to_string(),
                target_id: "PAY-102".to_string(),
                relationship_type: "contains".to_string(),
            }],
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: "chk1".to_string(),
            metadata: serde_json::json!({}),
        };

        let a2 = KnowledgeArtifact {
            id: "jira:PAY-102".to_string(),
            kind: ArtifactKind::Issue,
            title: "Stripe Webhook".to_string(),
            summary: Some("Summary 2".to_string()),
            body: "Body 2".to_string(),
            provider: "jira".to_string(),
            source_id: "PAY-102".to_string(),
            source_url: "https://jira.com/PAY-102".to_string(),
            repository: None,
            tags: vec![],
            relationships: vec![],
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: "chk2".to_string(),
            metadata: serde_json::json!({}),
        };

        storage.upsert_artifact(&a1).unwrap();
        storage.upsert_artifact(&a2).unwrap();

        let subgraph = fetch_subgraph(&storage, "jira:PAY-100", 1, 50).unwrap();
        assert_eq!(subgraph.root_id, "jira:PAY-100");
        assert_eq!(subgraph.nodes.len(), 2);
        assert_eq!(subgraph.edges.len(), 1);
        assert_eq!(subgraph.edges[0].label, "contains");
        assert_eq!(subgraph.edges[0].source, "jira:PAY-100");
        assert_eq!(subgraph.edges[0].target, "jira:PAY-102");
    }
}
