use crate::domain::{ArtifactKind, KnowledgeArtifact};
use crate::storage::Storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Directed edge in the artifact dependency graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
}

/// Artifact attached with an explicit relationship label, category, and graph metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabeledArtifact {
    pub artifact: KnowledgeArtifact,
    pub relationship_label: String,
    pub relationship_category: String,
    pub score: f64,
    pub is_direct_graph: bool,
}

/// Item in the Recommended Reading prioritized list
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendedItem {
    pub id: String,
    pub source_id: String,
    pub title: String,
    pub kind: String,
    pub relationship_label: String,
    pub score: f64,
    pub reason: String,
}

/// Category availability status for context summary & completeness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategoryAvailability {
    pub category: String,
    pub is_available: bool,
    pub count: usize,
    pub label: String,
}

/// Category score for breakdown completeness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategoryScore {
    pub category_name: String,
    pub score_percentage: u8,
    pub progress_bar: String,
    pub is_available: bool,
}

/// Deterministic completeness report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletenessReport {
    pub score_percentage: u8,
    pub progress_bar: String,
    pub category_scores: Vec<CategoryScore>,
    pub available_categories: Vec<CategoryAvailability>,
    pub missing_categories: Vec<CategoryAvailability>,
}

/// LLM-optimized summary briefing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiBriefing {
    pub feature_name: String,
    pub primary_repository: String,
    pub previous_prs: Vec<String>,
    pub released_in: Vec<String>,
    pub related_repositories: Vec<String>,
    pub known_dependencies: Vec<String>,
    pub architecture_documentation_status: String,
    pub historical_implementation_status: String,
    pub confidence_level: String,
}

/// Engineering Readiness overview
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineeringReadiness {
    pub status_label: String,
    pub readiness_summary: String,
    pub available: Vec<String>,
    pub missing: Vec<String>,
}

/// Deterministically generated next action item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NextAction {
    pub step: usize,
    pub action: String,
    pub detail: String,
    pub command: Option<String>,
}

/// Compact source metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceInfo {
    pub provider: String,
    pub repository: Option<String>,
    pub source_url: String,
    pub updated_at: String,
    pub synced_at: String,
}

/// Structured, AI-ready engineering context package assembled by ContextBuilder
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextPackage {
    pub target_kind: String,
    pub target_id: String,
    pub primary_artifact: Option<KnowledgeArtifact>,
    pub title: String,
    pub status: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub engineering_readiness: EngineeringReadiness,
    pub completeness: CompletenessReport,
    pub recommended_reading: Vec<RecommendedItem>,
    pub implementation_hints: Vec<String>,
    pub suggested_next_actions: Vec<NextAction>,
    pub affected_repositories: Vec<String>,
    pub related_artifacts: Vec<LabeledArtifact>,
    pub dependency_graph: Vec<DependencyEdge>,
    pub implementation_history: Vec<LabeledArtifact>,
    pub related_pull_requests: Vec<LabeledArtifact>,
    pub related_commits: Vec<LabeledArtifact>,
    pub related_documentation: Vec<LabeledArtifact>,
    pub apis: Vec<LabeledArtifact>,
    pub architecture_decisions: Vec<LabeledArtifact>,
    pub ai_briefing: Option<AiBriefing>,
    pub source_info: SourceInfo,
    pub summary: String,
}

/// Options to control ContextBuilder assembly limits
#[derive(Debug, Clone)]
pub struct ContextOptions {
    pub max_related: usize,
    pub max_history: usize,
    pub max_prs: usize,
    pub max_commits: usize,
    pub max_docs: usize,
    pub max_apis: usize,
    pub max_adrs: usize,
    pub max_recommended: usize,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            max_related: 10,
            max_history: 5,
            max_prs: 5,
            max_commits: 5,
            max_docs: 5,
            max_apis: 5,
            max_adrs: 5,
            max_recommended: 5,
        }
    }
}

pub struct ContextBuilder<'a> {
    storage: &'a Storage,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    /// Build structured engineering context briefing for a given target kind and target ID/name
    pub fn build(
        &self,
        target_kind: Option<&str>,
        target_id: &str,
        options: &ContextOptions,
    ) -> Result<ContextPackage> {
        let normalized_kind = target_kind
            .map(|k| k.trim().to_lowercase())
            .unwrap_or_else(|| "artifact".to_string());
        let clean_target_id = target_id.trim();

        // 1. Resolve Primary Artifact / Repository context
        let (primary_artifact, primary_repo) = self.resolve_primary(&normalized_kind, clean_target_id)?;

        let title = primary_artifact
            .as_ref()
            .map(|a| a.title.clone())
            .unwrap_or_else(|| clean_target_id.to_string());

        let status = primary_artifact
            .as_ref()
            .map(|a| extract_status(a))
            .unwrap_or_else(|| "Unknown".to_string());

        let primary_id_key = primary_artifact
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| clean_target_id.to_string());

        // 2. Gather candidate artifacts & dependency graph edges
        let mut candidates_map: HashMap<String, (KnowledgeArtifact, f64, Option<String>, bool)> = HashMap::new();
        let mut dependency_graph_set: HashSet<DependencyEdge> = HashSet::new();
        let mut direct_graph_ids: HashSet<String> = HashSet::new();

        // 2a. Direct 1-hop relationships
        if let Ok(related_tuples) = self.storage.get_related_artifacts(&primary_id_key) {
            for (rel, art) in related_tuples {
                dependency_graph_set.insert(DependencyEdge {
                    source_id: rel.source_id.clone(),
                    target_id: rel.target_id.clone(),
                    relationship_type: rel.relationship_type.clone(),
                });
                direct_graph_ids.insert(art.id.clone());
                direct_graph_ids.insert(art.source_id.clone());
                let label = map_relationship_type(&rel.relationship_type, &art.kind);
                self.add_candidate(&mut candidates_map, art, 10.0, Some(label), true);
            }
        }

        // Also inspect embedded relationships inside primary_artifact
        if let Some(ref primary) = primary_artifact {
            for rel in &primary.relationships {
                dependency_graph_set.insert(DependencyEdge {
                    source_id: rel.source_id.clone(),
                    target_id: rel.target_id.clone(),
                    relationship_type: rel.relationship_type.clone(),
                });
                direct_graph_ids.insert(rel.target_id.clone());
                let label = map_relationship_type(&rel.relationship_type, &classify_id(&rel.target_id));
                if let Ok(Some(art)) = self.storage.get_artifact_by_id(&rel.target_id) {
                    self.add_candidate(&mut candidates_map, art, 10.0, Some(label), true);
                }
            }
        }

        // 2b. 2-hop relationships (transitive connection expansion)
        let initial_ids: Vec<String> = candidates_map.keys().cloned().collect();
        for id in initial_ids {
            if let Ok(hop2) = self.storage.get_related_artifacts(&id) {
                for (rel, art) in hop2 {
                    dependency_graph_set.insert(DependencyEdge {
                        source_id: rel.source_id,
                        target_id: rel.target_id,
                        relationship_type: rel.relationship_type.clone(),
                    });
                    direct_graph_ids.insert(art.id.clone());
                    direct_graph_ids.insert(art.source_id.clone());
                    let label = map_relationship_type(&rel.relationship_type, &art.kind);
                    self.add_candidate(&mut candidates_map, art, 5.0, Some(label), true);
                }
            }
        }

        // 2c. Query repository artifacts if repo is available
        let active_repo = primary_artifact
            .as_ref()
            .and_then(|a| a.repository.as_deref())
            .or(primary_repo.as_deref());

        if let Some(repo) = active_repo {
            if let Ok(repo_arts) = self.storage.query_by_repository(repo, 30) {
                for art in repo_arts {
                    let is_direct = direct_graph_ids.contains(&art.id) || direct_graph_ids.contains(&art.source_id);
                    let label = infer_label_by_kind(&art.kind);
                    self.add_candidate(&mut candidates_map, art, 3.0, Some(label), is_direct);
                }
            }
        }

        // 2d. Perform BM25 FTS search based on terms from primary artifact or target_id
        if let Some(ref primary) = primary_artifact {
            let terms = extract_search_terms(&primary.title, &primary.tags);
            if !terms.is_empty() {
                if let Ok(fts_results) = self.storage.search_fts(&terms, None, None, active_repo, 20) {
                    for art in fts_results {
                        let is_direct = direct_graph_ids.contains(&art.id) || direct_graph_ids.contains(&art.source_id);
                        let label = if is_direct {
                            infer_label_by_kind(&art.kind)
                        } else {
                            "potentially related".to_string()
                        };
                        self.add_candidate(&mut candidates_map, art, 2.0, Some(label), is_direct);
                    }
                }
            }
        } else if !clean_target_id.is_empty() {
            if let Ok(fts_results) = self.storage.search_fts(clean_target_id, None, None, None, 20) {
                for art in fts_results {
                    let is_direct = direct_graph_ids.contains(&art.id) || direct_graph_ids.contains(&art.source_id);
                    let label = if is_direct {
                        infer_label_by_kind(&art.kind)
                    } else {
                        "potentially related".to_string()
                    };
                    self.add_candidate(&mut candidates_map, art, 2.0, Some(label), is_direct);
                }
            }
        }

        // Remove primary_artifact itself from candidate set
        if let Some(ref primary) = primary_artifact {
            candidates_map.remove(&primary.id);
            candidates_map.remove(&primary.source_id);
        }

        // 3. Classify and partition candidates into context categories
        let mut adr_list = Vec::new();
        let mut pr_list = Vec::new();
        let mut commit_list = Vec::new();
        let mut doc_list = Vec::new();
        let mut api_list = Vec::new();
        let mut history_list = Vec::new();
        let mut other_related = Vec::new();
        let mut affected_repos_set = HashSet::new();

        if let Some(ref primary) = primary_artifact {
            if let Some(ref r) = primary.repository {
                if !r.is_empty() {
                    affected_repos_set.insert(r.clone());
                }
            }
        }
        if let Some(ref r) = primary_repo {
            if !r.is_empty() {
                affected_repos_set.insert(r.clone());
            }
        }

        // Sort candidates deterministically: score desc, updated_at desc, source_id asc
        let mut candidate_entries: Vec<(KnowledgeArtifact, f64, String, bool)> = candidates_map
            .into_values()
            .map(|(art, score, label, is_direct)| {
                let rel_label = label.unwrap_or_else(|| {
                    if is_direct {
                        infer_label_by_kind(&art.kind)
                    } else {
                        "potentially related".to_string()
                    }
                });
                (art, score, rel_label, is_direct)
            })
            .collect();

        candidate_entries.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.0.updated_at.cmp(&a.0.updated_at))
                .then_with(|| a.0.source_id.cmp(&b.0.source_id))
        });

        // Calculate Recommended Reading prior to category deduction
        let recommended_reading = build_recommended_reading(
            &candidate_entries,
            active_repo,
            options.max_recommended,
        );

        let mut used_ids = HashSet::new();

        for (art, score, rel_label, is_direct) in candidate_entries {
            if let Some(ref r) = art.repository {
                if !r.is_empty() {
                    affected_repos_set.insert(r.clone());
                }
            }

            if used_ids.contains(&art.id) || used_ids.contains(&art.source_id) {
                continue;
            }

            let is_adr = is_architecture_decision(&art);
            let is_api = is_api_artifact(&art);
            let is_pr = matches!(art.kind, ArtifactKind::PullRequest | ArtifactKind::PullRequestReview | ArtifactKind::ReviewComment);
            let is_commit = matches!(art.kind, ArtifactKind::Commit);
            let is_doc = matches!(art.kind, ArtifactKind::Document | ArtifactKind::Specification) || art.provider == "confluence";
            let is_history = is_closed_or_historical(&art);

            let rel_category = map_relationship_category(&rel_label, &art.kind, is_direct);

            let labeled = LabeledArtifact {
                artifact: art.clone(),
                relationship_label: rel_label,
                relationship_category: rel_category,
                score,
                is_direct_graph: is_direct,
            };

            if is_adr && adr_list.len() < options.max_adrs {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                adr_list.push(labeled);
            } else if is_api && api_list.len() < options.max_apis {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                api_list.push(labeled);
            } else if is_pr && pr_list.len() < options.max_prs {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                pr_list.push(labeled);
            } else if is_commit && commit_list.len() < options.max_commits {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                commit_list.push(labeled);
            } else if is_doc && doc_list.len() < options.max_docs {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                doc_list.push(labeled);
            } else if is_history && history_list.len() < options.max_history {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                history_list.push(labeled);
            } else if other_related.len() < options.max_related {
                used_ids.insert(art.id.clone());
                used_ids.insert(art.source_id.clone());
                other_related.push(labeled);
            }
        }

        let mut affected_repositories: Vec<String> = affected_repos_set.into_iter().collect();
        affected_repositories.sort();

        let mut dependency_graph: Vec<DependencyEdge> = dependency_graph_set.into_iter().collect();
        dependency_graph.sort_by(|a, b| {
            a.source_id.cmp(&b.source_id)
                .then_with(|| a.target_id.cmp(&b.target_id))
                .then_with(|| a.relationship_type.cmp(&b.relationship_type))
        });

        // 4. Calculate Completeness & Readiness
        let completeness = compute_completeness(
            primary_artifact.is_some(),
            &affected_repositories,
            &adr_list,
            &doc_list,
            &api_list,
            &other_related,
            &pr_list,
            &commit_list,
            &history_list,
        );

        let engineering_readiness = compute_readiness(
            &completeness,
            primary_artifact.is_some(),
        );

        // 5. Generate implementation hints
        let hints = generate_implementation_hints(
            &primary_artifact,
            &adr_list,
            &pr_list,
            &api_list,
            &affected_repositories,
            &recommended_reading,
        );

        // 6. Generate suggested next actions
        let suggested_next_actions = generate_next_actions(
            clean_target_id,
            &title,
            &primary_artifact,
            &recommended_reading,
            active_repo,
        );

        // 7. Extract source metadata
        let source_info = build_source_info(
            &primary_artifact,
            active_repo,
            clean_target_id,
        );

        // 8. Generate briefing assessment summary
        let summary = build_engineering_assessment(
            &completeness,
            clean_target_id,
            &title,
        );

        let description = primary_artifact
            .as_ref()
            .map(|a| a.body.clone())
            .filter(|b| !b.is_empty());

        let final_repo = primary_artifact
            .as_ref()
            .and_then(|a| a.repository.clone())
            .or(primary_repo);

        let ai_briefing = Some(build_ai_briefing(
            &title,
            final_repo.as_deref(),
            &pr_list,
            &commit_list,
            &other_related,
            &affected_repositories,
            &adr_list,
        ));

        Ok(ContextPackage {
            target_kind: normalized_kind,
            target_id: clean_target_id.to_string(),
            primary_artifact,
            title,
            status,
            repository: final_repo,
            description,
            engineering_readiness,
            completeness,
            recommended_reading,
            implementation_hints: hints,
            suggested_next_actions,
            affected_repositories,
            related_artifacts: other_related,
            dependency_graph,
            implementation_history: history_list,
            related_pull_requests: pr_list,
            related_commits: commit_list,
            related_documentation: doc_list,
            apis: api_list,
            architecture_decisions: adr_list,
            ai_briefing,
            source_info,
            summary,
        })
    }

    fn resolve_primary(
        &self,
        kind: &str,
        target_id: &str,
    ) -> Result<(Option<KnowledgeArtifact>, Option<String>)> {
        if let Ok(matches) = self.storage.resolve_artifact_by_alias(target_id) {
            if let Some(art) = matches.into_iter().next() {
                let repo = art.repository.clone();
                return Ok((Some(art), repo));
            }
        }

        match kind {
            "pr" | "pull_request" | "pullrequest" => {
                let search_term = if !target_id.starts_with('#') && target_id.chars().all(|c| c.is_ascii_digit()) {
                    format!("#{}", target_id)
                } else {
                    target_id.to_string()
                };
                if let Ok(results) = self.storage.search_fts(&search_term, Some("pull_request"), None, None, 1) {
                    if let Some(art) = results.into_iter().next() {
                        let repo = art.repository.clone();
                        return Ok((Some(art), repo));
                    }
                }
            }
            "adr" | "design" | "spec" => {
                if let Ok(results) = self.storage.search_fts(target_id, None, None, None, 5) {
                    for art in results {
                        if is_architecture_decision(&art) {
                            let repo = art.repository.clone();
                            return Ok((Some(art), repo));
                        }
                    }
                }
            }
            "repository" | "repo" => {
                if let Ok(arts) = self.storage.query_by_repository(target_id, 10) {
                    if let Some(art) = arts.first() {
                        return Ok((Some(art.clone()), Some(target_id.to_string())));
                    }
                }
                return Ok((None, Some(target_id.to_string())));
            }
            _ => {}
        }

        if let Ok(results) = self.storage.search_fts(target_id, None, None, None, 1) {
            if let Some(art) = results.into_iter().next() {
                let repo = art.repository.clone();
                return Ok((Some(art), repo));
            }
        }

        Ok((None, None))
    }

    fn add_candidate(
        &self,
        map: &mut HashMap<String, (KnowledgeArtifact, f64, Option<String>, bool)>,
        art: KnowledgeArtifact,
        weight: f64,
        label: Option<String>,
        is_direct: bool,
    ) {
        let entry = map.entry(art.id.clone()).or_insert_with(|| (art.clone(), 0.0, label.clone(), is_direct));
        entry.1 += weight;
        if is_direct {
            entry.3 = true;
        }
        if entry.2.is_none() && label.is_some() {
            entry.2 = label;
        }
    }
}

fn map_relationship_type(rel_type: &str, kind: &ArtifactKind) -> String {
    match rel_type.to_lowercase().as_str() {
        "relates" | "relates_to" | "related" => "relates".to_string(),
        "blocks" => "blocks".to_string(),
        "blocked_by" => "blocked by".to_string(),
        "duplicates" | "duplicated_by" => "duplicates".to_string(),
        "implements" | "implemented_by" => "implements".to_string(),
        "documented_by" | "documents" => "defines architecture".to_string(),
        "commit" | "committed" => "introduced feature".to_string(),
        "owns" => "parent task".to_string(),
        "owned_by" | "subtask" => "child task".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => infer_label_by_kind(kind),
    }
}

fn map_relationship_category(label: &str, _kind: &ArtifactKind, is_direct: bool) -> String {
    if !is_direct {
        return "Potentially Related (BM25 Similarity)".to_string();
    }
    match label.to_lowercase().as_str() {
        "blocks" => "Blocks".to_string(),
        "blocked by" => "Blocked By".to_string(),
        "implements" | "implemented_by" => "Implements".to_string(),
        "duplicates" | "duplicated_by" => "Duplicate".to_string(),
        "parent task" | "parent" => "Parent".to_string(),
        "child task" | "child" | "subtask" => "Child".to_string(),
        "defines architecture" => "Architecture / Docs".to_string(),
        "introduced feature" => "Commits".to_string(),
        _ => "References".to_string(),
    }
}

fn infer_label_by_kind(kind: &ArtifactKind) -> String {
    match kind {
        ArtifactKind::PullRequest | ArtifactKind::PullRequestReview => "implements".to_string(),
        ArtifactKind::Commit => "introduced feature".to_string(),
        ArtifactKind::Document | ArtifactKind::Specification | ArtifactKind::Design => "defines architecture".to_string(),
        ArtifactKind::Component => "API contract".to_string(),
        ArtifactKind::Repository => "target repo".to_string(),
        _ => "relates".to_string(),
    }
}

fn classify_id(target: &str) -> ArtifactKind {
    let lower = target.to_lowercase();
    if lower.contains('#') || lower.contains("pr") {
        ArtifactKind::PullRequest
    } else if target.len() >= 7 && target.chars().all(|c| c.is_ascii_hexdigit()) {
        ArtifactKind::Commit
    } else if lower.starts_with("adr-") || lower.contains("doc") || lower.contains("spec") {
        ArtifactKind::Document
    } else {
        ArtifactKind::Ticket
    }
}

fn extract_status(art: &KnowledgeArtifact) -> String {
    if let Some(s) = art.metadata.get("status").and_then(|v| v.as_str()) {
        if !s.is_empty() {
            return s.to_string();
        }
    }
    if let Some(s) = art.metadata.get("state").and_then(|v| v.as_str()) {
        if !s.is_empty() {
            return s.to_string();
        }
    }
    "Done".to_string()
}

fn is_architecture_decision(art: &KnowledgeArtifact) -> bool {
    let lower_id = art.source_id.to_lowercase();
    let lower_title = art.title.to_lowercase();

    if lower_id.starts_with("adr-") || lower_id.contains("adr") || lower_title.contains("adr") {
        return true;
    }
    if matches!(art.kind, ArtifactKind::Design | ArtifactKind::Specification) {
        return true;
    }
    if art.tags.iter().any(|t| {
        let lt = t.to_lowercase();
        lt.contains("adr") || lt.contains("architecture") || lt.contains("design")
    }) {
        return true;
    }
    if lower_title.contains("architecture") || lower_title.contains("design decision") {
        return true;
    }
    false
}

fn is_api_artifact(art: &KnowledgeArtifact) -> bool {
    if matches!(art.kind, ArtifactKind::Component) {
        return true;
    }
    let lower_title = art.title.to_lowercase();
    let lower_id = art.source_id.to_lowercase();
    if lower_title.contains("api") || lower_id.contains("api") || lower_title.contains("endpoint") || lower_title.contains("openapi") || lower_title.contains("graphql") {
        return true;
    }
    if art.tags.iter().any(|t| {
        let lt = t.to_lowercase();
        lt.contains("api") || lt.contains("openapi") || lt.contains("graphql") || lt.contains("grpc") || lt.contains("rest")
    }) {
        return true;
    }
    false
}

fn is_closed_or_historical(art: &KnowledgeArtifact) -> bool {
    if let Some(status) = art.metadata.get("status").and_then(|v| v.as_str()) {
        let s = status.to_lowercase();
        if s == "closed" || s == "merged" || s == "done" || s == "resolved" {
            return true;
        }
    }
    if let Some(state) = art.metadata.get("state").and_then(|v| v.as_str()) {
        let s = state.to_lowercase();
        if s == "closed" || s == "merged" || s == "done" || s == "resolved" {
            return true;
        }
    }
    matches!(art.kind, ArtifactKind::Commit | ArtifactKind::Release)
}

fn extract_search_terms(title: &str, tags: &[String]) -> String {
    let mut words: Vec<&str> = title
        .split_whitespace()
        .filter(|w| w.len() > 3 && !w.chars().all(|c| c.is_ascii_punctuation()))
        .collect();
    for tag in tags {
        if tag.len() > 3 {
            words.push(tag.as_str());
        }
    }
    words.dedup();
    words.join(" ")
}

fn build_recommended_reading(
    candidates: &[(KnowledgeArtifact, f64, String, bool)],
    active_repo: Option<&str>,
    limit: usize,
) -> Vec<RecommendedItem> {
    let mut scored_items: Vec<(RecommendedItem, f64)> = Vec::new();

    for (art, base_score, label, is_direct) in candidates {
        // Priority order requested:
        // 1. Architecture Decisions (ADR)
        // 2. Design Documents
        // 3. API Specifications
        // 4. Previous Pull Requests
        // 5. Previous Issues / Tickets
        // 6. Related Commits
        let mut r_score = *base_score;
        let mut reason = format!("Context reference ({})", label);

        if is_architecture_decision(art) {
            r_score += 50.0;
            reason = "Architecture Decision (ADR) guideline".to_string();
        } else if matches!(art.kind, ArtifactKind::Design | ArtifactKind::Specification) {
            r_score += 45.0;
            reason = "Design Specification Document".to_string();
        } else if is_api_artifact(art) {
            r_score += 40.0;
            reason = "API Contract Specification".to_string();
        } else if matches!(art.kind, ArtifactKind::PullRequest | ArtifactKind::PullRequestReview) {
            r_score += 30.0;
            reason = "Prior Implementation PR".to_string();
        } else if matches!(art.kind, ArtifactKind::Ticket | ArtifactKind::Issue) {
            r_score += 20.0;
            reason = "Related Issue / Ticket".to_string();
        } else if matches!(art.kind, ArtifactKind::Commit) {
            r_score += 15.0;
            reason = "Related Commit History".to_string();
        }

        if *is_direct {
            r_score += 20.0;
        }

        if let Some(repo) = active_repo {
            if art.repository.as_deref() == Some(repo) {
                r_score += 10.0;
            }
        }

        let item = RecommendedItem {
            id: art.id.clone(),
            source_id: art.source_id.clone(),
            title: art.title.clone(),
            kind: art.kind.to_string(),
            relationship_label: label.clone(),
            score: r_score,
            reason,
        };

        scored_items.push((item, r_score));
    }

    scored_items.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.source_id.cmp(&b.0.source_id))
    });

    scored_items
        .into_iter()
        .take(limit)
        .map(|(item, _)| item)
        .collect()
}

fn compute_completeness(
    has_primary: bool,
    repos: &[String],
    adrs: &[LabeledArtifact],
    docs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    tickets: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    commits: &[LabeledArtifact],
    history: &[LabeledArtifact],
) -> CompletenessReport {
    let mut available = Vec::new();
    let mut missing = Vec::new();
    let mut category_scores = Vec::new();

    // Category breakdown calculation for Phase 5
    let biz_score: u8 = if has_primary { 100 } else { 0 };
    let biz_bar = if has_primary { "█".repeat(10) } else { "░".repeat(10) };
    category_scores.push(CategoryScore {
        category_name: "Business Context".to_string(),
        score_percentage: biz_score,
        progress_bar: biz_bar,
        is_available: has_primary,
    });

    let has_impl = !prs.is_empty() || !commits.is_empty() || !history.is_empty();
    let impl_score: u8 = if has_impl { 100 } else { 0 };
    let impl_bar = if has_impl { "█".repeat(10) } else { "░".repeat(10) };
    category_scores.push(CategoryScore {
        category_name: "Implementation Context".to_string(),
        score_percentage: impl_score,
        progress_bar: impl_bar,
        is_available: has_impl,
    });

    let has_repo = !repos.is_empty();
    let repo_score: u8 = if has_repo { 100 } else { 0 };
    let repo_bar = if has_repo { "█".repeat(10) } else { "░".repeat(10) };
    category_scores.push(CategoryScore {
        category_name: "Repository Knowledge".to_string(),
        score_percentage: repo_score,
        progress_bar: repo_bar,
        is_available: has_repo,
    });

    let arch_score: u8 = if !adrs.is_empty() {
        100
    } else if !apis.is_empty() {
        30
    } else {
        0
    };
    let arch_filled = ((arch_score as usize + 5) / 10).min(10);
    let arch_bar = format!("{}{}", "█".repeat(arch_filled), "░".repeat(10 - arch_filled));
    category_scores.push(CategoryScore {
        category_name: "Architecture Context".to_string(),
        score_percentage: arch_score,
        progress_bar: arch_bar,
        is_available: !adrs.is_empty() || !apis.is_empty(),
    });

    let has_doc = !docs.is_empty();
    let doc_score: u8 = if has_doc { 100 } else { 0 };
    let doc_bar = if has_doc { "█".repeat(10) } else { "░".repeat(10) };
    category_scores.push(CategoryScore {
        category_name: "Documentation".to_string(),
        score_percentage: doc_score,
        progress_bar: doc_bar,
        is_available: has_doc,
    });

    let mut total_score: u32 = 0;

    if has_primary {
        total_score += 10;
    }

    // 1. Repository (15%)
    if !repos.is_empty() {
        total_score += 15;
        let count = repos.len();
        available.push(CategoryAvailability {
            category: "Repository".to_string(),
            is_available: true,
            count,
            label: "Repository".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Repository".to_string(),
            is_available: false,
            count: 0,
            label: "Repository".to_string(),
        });
    }

    // 2. Architecture Decision (ADR) (20%)
    if !adrs.is_empty() {
        total_score += 20;
        let count = adrs.len();
        available.push(CategoryAvailability {
            category: "Architecture Decision".to_string(),
            is_available: true,
            count,
            label: "ADR".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Architecture Decision".to_string(),
            is_available: false,
            count: 0,
            label: "ADR".to_string(),
        });
    }

    // 3. Documentation (15%)
    if !docs.is_empty() || !adrs.is_empty() {
        total_score += 15;
        let count = docs.len() + adrs.len();
        available.push(CategoryAvailability {
            category: "Documentation".to_string(),
            is_available: true,
            count,
            label: "Documentation".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Documentation".to_string(),
            is_available: false,
            count: 0,
            label: "Documentation".to_string(),
        });
    }

    // 4. Related APIs (15%)
    if !apis.is_empty() {
        total_score += 15;
        let count = apis.len();
        available.push(CategoryAvailability {
            category: "Related APIs".to_string(),
            is_available: true,
            count,
            label: "Related APIs".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Related APIs".to_string(),
            is_available: false,
            count: 0,
            label: "Related APIs".to_string(),
        });
    }

    // 5. Pull Requests (15%)
    if !prs.is_empty() {
        total_score += 15;
        let count = prs.len();
        available.push(CategoryAvailability {
            category: "Previous PRs".to_string(),
            is_available: true,
            count,
            label: "Previous Pull Requests".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Previous PRs".to_string(),
            is_available: false,
            count: 0,
            label: "Previous Pull Requests".to_string(),
        });
    }

    // 6. Commits (10%)
    if !commits.is_empty() || !history.is_empty() {
        total_score += 10;
        let count = commits.len() + history.len();
        available.push(CategoryAvailability {
            category: "Commit History".to_string(),
            is_available: true,
            count,
            label: "Commit History".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Commit History".to_string(),
            is_available: false,
            count: 0,
            label: "Commit History".to_string(),
        });
    }

    // 7. Related Issues (10%)
    if !tickets.is_empty() {
        total_score += 10;
        let count = tickets.len();
        available.push(CategoryAvailability {
            category: "Related Issues".to_string(),
            is_available: true,
            count,
            label: "Related Issues".to_string(),
        });
    } else {
        missing.push(CategoryAvailability {
            category: "Related Issues".to_string(),
            is_available: false,
            count: 0,
            label: "Related Issues".to_string(),
        });
    }

    let score_percentage = (total_score.min(100)) as u8;
    let blocks_filled = ((score_percentage as u32 + 5) / 10).min(10) as usize;
    let progress_bar = format!("{}{}", "█".repeat(blocks_filled), "░".repeat(10 - blocks_filled));

    CompletenessReport {
        score_percentage,
        progress_bar,
        category_scores,
        available_categories: available,
        missing_categories: missing,
    }
}

fn compute_readiness(completeness: &CompletenessReport, _has_primary: bool) -> EngineeringReadiness {
    let status_label = if completeness.score_percentage >= 80 {
        "Ready for implementation.".to_string()
    } else if completeness.score_percentage >= 50 {
        "Ready for implementation.".to_string()
    } else {
        "Needs architectural clarification.".to_string()
    };

    let readiness_summary = if completeness.score_percentage >= 80 {
        "Atlas found comprehensive engineering context and architectural references.".to_string()
    } else if completeness.score_percentage >= 50 {
        "Atlas found sufficient implementation context, but architectural references are incomplete.".to_string()
    } else {
        "Atlas found initial context, but key architectural and implementation artifacts are missing.".to_string()
    };

    let available = completeness
        .available_categories
        .iter()
        .map(|c| c.category.clone())
        .collect();

    let missing = completeness
        .missing_categories
        .iter()
        .map(|c| c.category.clone())
        .collect();

    EngineeringReadiness {
        status_label,
        readiness_summary,
        available,
        missing,
    }
}

fn generate_implementation_hints(
    _primary: &Option<KnowledgeArtifact>,
    _adrs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    repos: &[String],
    reading: &[RecommendedItem],
) -> Vec<String> {
    let mut hints = Vec::new();

    if let Some(ref r) = repos.first() {
        hints.push(format!("Feature belongs to {} repository.", r));
    }

    if let Some(item) = reading.first() {
        hints.push(format!("Review {} before implementation.", item.source_id));
    }

    if !apis.is_empty() {
        hints.push("Verify API contract before coding.".to_string());
    }

    if !prs.is_empty() {
        hints.push("Review prior pull request patterns for implementation guidance.".to_string());
    }

    if hints.is_empty() {
        hints.push("Context assembled cleanly from Atlas knowledge engine.".to_string());
    }

    hints
}

fn generate_next_actions(
    target_id: &str,
    title: &str,
    primary: &Option<KnowledgeArtifact>,
    reading: &[RecommendedItem],
    active_repo: Option<&str>,
) -> Vec<NextAction> {
    let mut actions = Vec::new();
    let mut step = 1;

    if let Some(item) = reading.first() {
        actions.push(NextAction {
            step,
            action: format!("Review {}", item.source_id),
            detail: format!("({})", item.reason),
            command: None,
        });
        step += 1;
    }

    if let Some(repo) = active_repo {
        actions.push(NextAction {
            step,
            action: format!("Inspect repository {}", repo),
            detail: "Review codebase location and setup".to_string(),
            command: None,
        });
        step += 1;
    }

    let search_term = extract_search_terms(title, &[]);
    if !search_term.is_empty() {
        actions.push(NextAction {
            step,
            action: "Search previous implementation".to_string(),
            detail: format!("Search codebase using term: '{}'", search_term),
            command: Some(format!("atx search \"{}\"", search_term)),
        });
        step += 1;
    }

    if let Some(ref art) = primary {
        if !art.source_url.is_empty() {
            actions.push(NextAction {
                step,
                action: format!("Open original {}", art.provider),
                detail: art.source_url.clone(),
                command: None,
            });
        }
    } else {
        actions.push(NextAction {
            step,
            action: format!("Open artifact {}", target_id),
            detail: "Retrieve original source URL".to_string(),
            command: Some(format!("atx artifact {}", target_id)),
        });
    }

    actions
}

fn build_source_info(
    primary: &Option<KnowledgeArtifact>,
    active_repo: Option<&str>,
    target_id: &str,
) -> SourceInfo {
    if let Some(ref art) = primary {
        SourceInfo {
            provider: art.provider.clone(),
            repository: art.repository.clone().or_else(|| active_repo.map(|s| s.to_string())),
            source_url: art.source_url.clone(),
            updated_at: art.updated_at.format("%Y-%m-%d").to_string(),
            synced_at: art.synced_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        }
    } else {
        SourceInfo {
            provider: "Atlas".to_string(),
            repository: active_repo.map(|s| s.to_string()),
            source_url: format!("atlas://{}", target_id),
            updated_at: "Unknown".to_string(),
            synced_at: "Just now".to_string(),
        }
    }
}

fn build_engineering_assessment(
    completeness: &CompletenessReport,
    _target_id: &str,
    _title: &str,
) -> String {
    let mut lines = Vec::new();

    if completeness.score_percentage >= 50 {
        lines.push("Implementation can begin.");
        lines.push("Business requirements are available.");
    } else {
        lines.push("Implementation should proceed with caution.");
        lines.push("Business requirements are partially available.");
    }

    let has_history = completeness
        .available_categories
        .iter()
        .any(|c| c.category.contains("PR") || c.category.contains("Commit"));
    if has_history {
        lines.push("Implementation history is available.");
    } else {
        lines.push("Implementation history is incomplete.");
    }

    let has_adr = completeness
        .available_categories
        .iter()
        .any(|c| c.category.contains("Architecture") || c.category.contains("ADR"));
    if has_adr {
        lines.push("Architecture guidelines are documented.");
    } else {
        lines.push("Architecture knowledge is missing.");
    }

    lines.join("\n")
}

fn build_ai_briefing(
    title: &str,
    repo: Option<&str>,
    prs: &[LabeledArtifact],
    commits: &[LabeledArtifact],
    other_related: &[LabeledArtifact],
    affected_repos: &[String],
    adrs: &[LabeledArtifact],
) -> AiBriefing {
    let feature_name = title.to_string();
    let primary_repository = repo.unwrap_or("Unknown").to_string();
    let previous_prs = prs
        .iter()
        .map(|p| {
            let pid = &p.artifact.source_id;
            let pr_num = if pid.contains('#') {
                if let Some(pos) = pid.rfind('#') {
                    pid[pos..].to_string()
                } else {
                    pid.to_string()
                }
            } else if !pid.starts_with('#') && pid.chars().all(|c| c.is_ascii_digit()) {
                format!("#{}", pid)
            } else {
                pid.to_string()
            };
            format!("PR {} ({})", pr_num, p.artifact.title)
        })
        .collect();

    let released_in = other_related
        .iter()
        .filter(|a| a.artifact.kind == ArtifactKind::Release)
        .map(|r| r.artifact.title.clone())
        .collect();

    let related_repositories = affected_repos.to_vec();

    let known_dependencies = other_related
        .iter()
        .filter(|a| a.artifact.kind == ArtifactKind::Ticket || a.artifact.kind == ArtifactKind::Issue)
        .map(|t| t.artifact.source_id.clone())
        .collect();

    let arch_status = if !adrs.is_empty() {
        "Available".to_string()
    } else {
        "Not available".to_string()
    };

    let impl_status = if !prs.is_empty() || !commits.is_empty() {
        format!("Available ({} PRs, {} commits)", prs.len(), commits.len())
    } else {
        "Not available".to_string()
    };

    let confidence = if !prs.is_empty() && !affected_repos.is_empty() {
        "High".to_string()
    } else if !commits.is_empty() {
        "Moderate".to_string()
    } else {
        "Low".to_string()
    };

    AiBriefing {
        feature_name,
        primary_repository,
        previous_prs,
        released_in,
        related_repositories,
        known_dependencies,
        architecture_documentation_status: arch_status,
        historical_implementation_status: impl_status,
        confidence_level: confidence,
    }
}
