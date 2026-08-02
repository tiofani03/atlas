use crate::domain::{ArtifactKind, DomainAspect, KnowledgeArtifact};
use crate::storage::{ArtifactHeader, Storage};
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

/// Profiling telemetry breakdown for context building performance auditing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextTelemetry {
    pub primary_resolution_ms: u128,
    pub hop1_traversal_ms: u128,
    pub hop2_traversal_ms: u128,
    pub repo_fts_search_ms: u128,
    pub candidate_ranking_ms: u128,
    pub artifact_hydration_ms: u128,
    pub prompt_assembly_ms: u128,
    pub total_ms: u128,
    pub candidate_headers_count: usize,
    pub hydrated_artifacts_count: usize,
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
    pub star_rating: String,
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

/// Verified fact directly supported by indexed artifacts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnownFact {
    pub statement: String,
    pub is_verified: bool,
    pub source_artifact: Option<String>,
}

/// Evidence item ranked by confidence level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceItem {
    pub artifact_id: String,
    pub title: String,
    pub kind: String,
    pub confidence_level: String, // "High Confidence", "Medium Confidence", "Low Confidence"
    pub star_rating: String,
    pub reason: String,
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

/// Mission overview for AI execution planner
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mission {
    pub target_feature: String,
    pub business_objective: String,
    pub expected_outcome: String,
    pub repository: String,
    pub estimated_complexity: String,
    pub goal: String,
    pub objective: String,
    pub complexity: String,
}

/// Current understanding extracted from artifact body & graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentUnderstanding {
    pub business_rules: Vec<String>,
    pub affected_domains: Vec<String>,
    pub known_constraints: Vec<String>,
}

/// Star-rated module impact in ImplementationHypothesis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleRating {
    pub module_name: String,
    pub rating_stars: String,
}

/// Scope item in ImplementationHypothesis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopeItem {
    pub area: String,
    pub is_likely: bool,
}

/// Probabilistic engineering hypothesis generated from graph topology
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplementationHypothesis {
    pub scope: Vec<ScopeItem>,
    pub primary_flow: Vec<String>,
    pub likely_modified_modules: Vec<ModuleRating>,
    pub potential_integrations: Vec<String>,
    pub impact_level: String,
    pub estimated_components: String,
    pub confidence: String,
}

/// Possible implementation areas (domains, not execution flow)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PossibleImplementationAreas {
    pub business_rules: Vec<String>,
    pub potential_components: Vec<String>,
    pub impact_level: String,
    pub confidence: String,
    pub uncertainty_note: String,
}

/// Step in mechanical Execution Queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueStep {
    pub step_index: usize,
    pub total_steps: usize,
    pub category: String, // "Required" or "Optional"
    pub title: String,
    pub artifact_label: Option<String>,
    pub reason: String,
    pub command: Option<String>,
    pub status: String,
}

/// Implementation risk prediction tied to evidence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplementationRisk {
    pub level: String, // "HIGH", "MEDIUM", "LOW" or "Potential Risk"
    pub area: String,
    pub description: String,
    pub evidence: String,
}

/// Classified knowledge gap explaining impact and suggested retrieval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifiedKnowledgeGap {
    pub severity: String, // "HIGH", "MEDIUM", "LOW"
    pub gap_type: String,
    pub impact: String,
    pub suggested_retrieval: String,
}

/// Prioritized knowledge gaps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrioritizedKnowledgeGaps {
    pub critical: Vec<String>,
    pub recommended: Vec<String>,
    pub optional: Vec<String>,
}

/// Sequential investigative step for downstream LLM execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestigationStep {
    pub step_number: usize,
    pub goal: String,
    pub inspect_target: String,
    pub expected_outcome: String,
}

/// Status check item for AI investigation status summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusCheck {
    pub label: String,
    pub is_available: bool,
}

/// Actionable guidance for AI agents exploring the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiGuidance {
    pub artifact_nature: String,
    pub exploration_strategy: String,
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
    pub target_aspects: HashSet<DomainAspect>,
    pub primary_artifact: Option<KnowledgeArtifact>,
    pub title: String,
    pub status: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub overview_summary: String,
    pub mission: Option<Mission>,
    pub known_facts: Vec<KnownFact>,
    pub evidence_ranking: Vec<EvidenceItem>,
    pub implementation_areas: Option<PossibleImplementationAreas>,
    pub understanding: Option<CurrentUnderstanding>,
    pub hypothesis: Option<ImplementationHypothesis>,
    pub execution_queue: Vec<QueueStep>,
    pub risks: Vec<ImplementationRisk>,
    pub classified_gaps: Vec<ClassifiedKnowledgeGap>,
    pub prioritized_gaps: PrioritizedKnowledgeGaps,
    pub investigation_steps: Vec<InvestigationStep>,
    pub unknowns: Vec<String>,
    pub investigation_status: Vec<StatusCheck>,
    pub ai_guidance: Option<AiGuidance>,
    pub ai_guidance_bullets: Vec<String>,
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
    pub telemetry: Option<ContextTelemetry>,
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
    pub depth: usize,
    pub profile: bool,
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
            depth: 2,
            profile: false,
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
        let start_total = std::time::Instant::now();
        let normalized_kind = target_kind
            .map(|k| k.trim().to_lowercase())
            .unwrap_or_else(|| "artifact".to_string());
        let clean_target_id = target_id.trim();

        // 1. Resolve Primary Artifact / Repository context
        let start_primary = std::time::Instant::now();
        let (primary_artifact, primary_repo) = self.resolve_primary(&normalized_kind, clean_target_id)?;
        let t_primary = start_primary.elapsed().as_millis();

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

        // 2. ID-First Header Expansion
        let mut candidates_map: HashMap<String, (ArtifactHeader, f64, Option<String>, bool)> = HashMap::new();
        let mut dependency_graph_set: HashSet<DependencyEdge> = HashSet::new();
        let mut direct_graph_ids: HashSet<String> = HashSet::new();

        // 2a. Direct 1-hop relationships using lightweight headers
        let start_hop1 = std::time::Instant::now();
        if let Ok(related_tuples) = self.storage.get_related_headers(&primary_id_key) {
            for (rel, header) in related_tuples {
                dependency_graph_set.insert(DependencyEdge {
                    source_id: rel.source_id.clone(),
                    target_id: rel.target_id.clone(),
                    relationship_type: rel.relationship_type.clone(),
                });
                direct_graph_ids.insert(header.id.clone());
                direct_graph_ids.insert(header.source_id.clone());
                let label = map_relationship_type(&rel.relationship_type, &header.kind);
                self.add_candidate_header(&mut candidates_map, header, 10.0, Some(label), true);
            }
        }

        if let Some(ref primary) = primary_artifact {
            let target_ids: Vec<String> = primary.relationships.iter().map(|r| r.target_id.clone()).collect();
            if let Ok(headers) = self.storage.get_artifact_headers_by_ids(&target_ids) {
                let mut header_map: HashMap<String, ArtifactHeader> = HashMap::new();
                for h in headers {
                    header_map.insert(h.id.clone(), h.clone());
                    header_map.insert(h.source_id.clone(), h);
                }
                for rel in &primary.relationships {
                    dependency_graph_set.insert(DependencyEdge {
                        source_id: rel.source_id.clone(),
                        target_id: rel.target_id.clone(),
                        relationship_type: rel.relationship_type.clone(),
                    });
                    direct_graph_ids.insert(rel.target_id.clone());
                    let label = map_relationship_type(&rel.relationship_type, &classify_id(&rel.target_id));
                    if let Some(header) = header_map.get(&rel.target_id) {
                        self.add_candidate_header(&mut candidates_map, header.clone(), 10.0, Some(label), true);
                    }
                }
            }
        }
        let t_hop1 = start_hop1.elapsed().as_millis();

        // 2b. 2-hop relationships (transitive connection expansion if depth >= 2)
        let start_hop2 = std::time::Instant::now();
        if options.depth >= 2 {
            let initial_ids: Vec<String> = candidates_map.keys().cloned().collect();
            if let Ok(hop2) = self.storage.get_batch_related_headers(&initial_ids) {
                for (rel, header) in hop2 {
                    dependency_graph_set.insert(DependencyEdge {
                        source_id: rel.source_id,
                        target_id: rel.target_id,
                        relationship_type: rel.relationship_type.clone(),
                    });
                    direct_graph_ids.insert(header.id.clone());
                    direct_graph_ids.insert(header.source_id.clone());
                    let label = map_relationship_type(&rel.relationship_type, &header.kind);
                    self.add_candidate_header(&mut candidates_map, header, 5.0, Some(label), true);
                }
            }
        }
        let t_hop2 = start_hop2.elapsed().as_millis();

        // 2c & 2d. Query repository headers & FTS search headers
        let start_repo_fts = std::time::Instant::now();
        let active_repo = primary_artifact
            .as_ref()
            .and_then(|a| a.repository.as_deref())
            .or(primary_repo.as_deref());

        let start_repo_only = std::time::Instant::now();
        if let Some(repo) = active_repo {
            if let Ok(repo_headers) = self.storage.query_headers_by_repository(repo, 30) {
                for header in repo_headers {
                    let is_direct = direct_graph_ids.contains(&header.id) || direct_graph_ids.contains(&header.source_id);
                    let label = infer_label_by_kind(&header.kind);
                    self.add_candidate_header(&mut candidates_map, header, 3.0, Some(label), is_direct);
                }
            }
        }
        let _t_repo_only = start_repo_only.elapsed().as_millis();

        let start_fts_only = std::time::Instant::now();
        if let Some(ref primary) = primary_artifact {
            let terms = extract_search_terms(&primary.title, &primary.tags);
            if !terms.is_empty() {
                if let Ok(fts_results) = self.storage.search_fts_headers(&terms, None, active_repo, 20) {
                    for header in fts_results {
                        let is_direct = direct_graph_ids.contains(&header.id) || direct_graph_ids.contains(&header.source_id);
                        let label = if is_direct {
                            infer_label_by_kind(&header.kind)
                        } else {
                            "potentially related".to_string()
                        };
                        self.add_candidate_header(&mut candidates_map, header, 2.0, Some(label), is_direct);
                    }
                }
            }
        } else if !clean_target_id.is_empty() {
            if let Ok(fts_results) = self.storage.search_fts_headers(clean_target_id, None, None, 20) {
                for header in fts_results {
                    let is_direct = direct_graph_ids.contains(&header.id) || direct_graph_ids.contains(&header.source_id);
                    let label = if is_direct {
                        infer_label_by_kind(&header.kind)
                    } else {
                        "potentially related".to_string()
                    };
                    self.add_candidate_header(&mut candidates_map, header, 2.0, Some(label), is_direct);
                }
            }
        }
        let _t_fts_only = start_fts_only.elapsed().as_millis();

        if let Some(ref primary) = primary_artifact {
            candidates_map.remove(&primary.id);
            candidates_map.remove(&primary.source_id);
        }
        let t_repo_fts = start_repo_fts.elapsed().as_millis();

        let total_candidate_headers = candidates_map.len();

        // 3. ID-First Candidate Ranking & Category Selection
        let start_ranking = std::time::Instant::now();
        let mut candidate_entries: Vec<(ArtifactHeader, f64, String, bool)> = candidates_map
            .into_values()
            .map(|(header, mut score, label, is_direct)| {
                if header.kind == ArtifactKind::Commit {
                    score += score_commit_relevance(&header, clean_target_id, is_direct);
                }
                let rel_label = label.unwrap_or_else(|| {
                    if is_direct {
                        infer_label_by_kind(&header.kind)
                    } else {
                        "potentially related".to_string()
                    }
                });
                (header, score, rel_label, is_direct)
            })
            .collect();

        candidate_entries.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.0.updated_at.cmp(&a.0.updated_at))
                .then_with(|| a.0.source_id.cmp(&b.0.source_id))
        });

        let recommended_reading = build_recommended_reading_headers(
            &candidate_entries,
            active_repo,
            options.max_recommended,
        );

        let mut winning_headers: Vec<(ArtifactHeader, f64, String, bool)> = Vec::new();
        let mut used_ids = HashSet::new();

        let mut adr_count = 0;
        let mut pr_count = 0;
        let mut commit_count = 0;
        let mut doc_count = 0;
        let mut api_count = 0;
        let mut history_count = 0;
        let mut related_count = 0;

        for (header, score, rel_label, is_direct) in candidate_entries {
            if used_ids.contains(&header.id) || used_ids.contains(&header.source_id) {
                continue;
            }

            let is_adr = is_architecture_decision_header(&header);
            let is_api = is_api_header(&header);
            let is_pr = matches!(header.kind, ArtifactKind::PullRequest | ArtifactKind::PullRequestReview | ArtifactKind::ReviewComment);
            let is_commit = matches!(header.kind, ArtifactKind::Commit);
            let is_doc = matches!(header.kind, ArtifactKind::Document | ArtifactKind::Specification) || header.provider == "confluence";
            let is_history = is_closed_or_historical_header(&header);

            let mut selected = false;

            if is_adr && adr_count < options.max_adrs {
                adr_count += 1;
                selected = true;
            } else if is_api && api_count < options.max_apis {
                api_count += 1;
                selected = true;
            } else if is_pr && pr_count < options.max_prs {
                pr_count += 1;
                selected = true;
            } else if is_commit && commit_count < options.max_commits {
                commit_count += 1;
                selected = true;
            } else if is_doc && doc_count < options.max_docs {
                doc_count += 1;
                selected = true;
            } else if is_history && history_count < options.max_history {
                history_count += 1;
                selected = true;
            } else if related_count < options.max_related {
                related_count += 1;
                selected = true;
            }

            if selected {
                used_ids.insert(header.id.clone());
                used_ids.insert(header.source_id.clone());
                winning_headers.push((header, score, rel_label, is_direct));
            }
        }
        let t_ranking = start_ranking.elapsed().as_millis();

        // 4. Batch Hydrate ONLY Winning Artifacts
        let start_hydration = std::time::Instant::now();
        let winning_ids: Vec<String> = winning_headers.iter().map(|(h, _, _, _)| h.id.clone()).collect();
        let hydrated_list = self.storage.get_artifacts_by_ids(&winning_ids)?;
        let hydrated_map: HashMap<String, KnowledgeArtifact> = hydrated_list
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();
        let t_hydration = start_hydration.elapsed().as_millis();

        let total_hydrated = hydrated_map.len();

        // 5. Construct LabeledArtifact output lists from hydrated winners
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

        for (header, score, rel_label, is_direct) in winning_headers {
            if let Some(ref r) = header.repository {
                if !r.is_empty() {
                    affected_repos_set.insert(r.clone());
                }
            }

            let artifact = match hydrated_map.get(&header.id) {
                Some(a) => a.clone(),
                None => continue,
            };

            let is_adr = is_architecture_decision(&artifact);
            let is_api = is_api_artifact(&artifact);
            let is_pr = matches!(artifact.kind, ArtifactKind::PullRequest | ArtifactKind::PullRequestReview | ArtifactKind::ReviewComment);
            let is_commit = matches!(artifact.kind, ArtifactKind::Commit);
            let is_doc = matches!(artifact.kind, ArtifactKind::Document | ArtifactKind::Specification) || artifact.provider == "confluence";
            let is_history = is_closed_or_historical(&artifact);

            let rel_category = map_relationship_category(&rel_label, &artifact.kind, is_direct);

            let labeled = LabeledArtifact {
                artifact,
                relationship_label: rel_label,
                relationship_category: rel_category,
                score,
                is_direct_graph: is_direct,
            };

            if is_adr && adr_list.len() < options.max_adrs {
                adr_list.push(labeled);
            } else if is_api && api_list.len() < options.max_apis {
                api_list.push(labeled);
            } else if is_pr && pr_list.len() < options.max_prs {
                pr_list.push(labeled);
            } else if is_commit && commit_list.len() < options.max_commits {
                commit_list.push(labeled);
            } else if is_doc && doc_list.len() < options.max_docs {
                doc_list.push(labeled);
            } else if is_history && history_list.len() < options.max_history {
                history_list.push(labeled);
            } else if other_related.len() < options.max_related {
                other_related.push(labeled);
            }
        }

        let start_assembly = std::time::Instant::now();
        let mut affected_repositories: Vec<String> = affected_repos_set.into_iter().collect();
        affected_repositories.sort();

        let mut dependency_graph: Vec<DependencyEdge> = dependency_graph_set.into_iter().collect();
        dependency_graph.sort_by(|a, b| {
            a.source_id.cmp(&b.source_id)
                .then_with(|| a.target_id.cmp(&b.target_id))
                .then_with(|| a.relationship_type.cmp(&b.relationship_type))
        });

        let target_aspects = primary_artifact
            .as_ref()
            .map(|a| a.classify_aspects())
            .unwrap_or_else(|| {
                let lower = clean_target_id.to_lowercase();
                let mut set = HashSet::new();
                if lower.contains("sprint") || lower.contains("meeting") || lower.contains("retro") || lower.contains("standup") {
                    set.insert(DomainAspect::Collaboration);
                } else if lower.contains("doc") || lower.contains("wiki") || lower.contains("notion") {
                    set.insert(DomainAspect::Documentation);
                } else if lower.contains("adr") || lower.contains("rfc") || lower.contains("architecture") {
                    set.insert(DomainAspect::Architecture);
                } else if lower.contains("figma") || lower.contains("ui") || lower.contains("mockup") {
                    set.insert(DomainAspect::Design);
                } else {
                    set.insert(DomainAspect::CodeImplementation);
                }
                set
            });

        let completeness = compute_completeness(
            &target_aspects,
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

        let overview_summary = generate_overview_summary(
            &primary_artifact,
            &title,
            clean_target_id,
            &target_aspects,
        );

        let mission = Some(build_mission(
            &title,
            &primary_artifact,
            &affected_repositories,
            &target_aspects,
        ));

        let known_facts = build_known_facts(
            &primary_artifact,
            &affected_repositories,
            &adr_list,
            &api_list,
            &pr_list,
            &commit_list,
            &other_related,
        );

        let evidence_ranking = build_evidence_ranking(
            &adr_list,
            &api_list,
            &pr_list,
            &other_related,
            &recommended_reading,
        );

        let implementation_areas = Some(infer_implementation_areas(
            &primary_artifact,
            &target_aspects,
            &affected_repositories,
            &adr_list,
            &api_list,
        ));

        let understanding = Some(build_current_understanding(
            &primary_artifact,
            &recommended_reading,
            &api_list,
            &pr_list,
            &target_aspects,
        ));

        let hypothesis = Some(infer_advanced_hypothesis(
            &primary_artifact,
            &target_aspects,
            &affected_repositories,
            &recommended_reading,
            &adr_list,
            &pr_list,
            &api_list,
        ));

        let execution_queue = build_dependency_aware_queue(
            clean_target_id,
            &title,
            &primary_artifact,
            &recommended_reading,
            &affected_repositories,
            &pr_list,
            &adr_list,
            &api_list,
            &other_related,
        );

        let risks = predict_implementation_risks(
            &primary_artifact,
            &affected_repositories,
            &adr_list,
            &pr_list,
            &api_list,
            &other_related,
            &target_aspects,
        );

        let classified_gaps = build_classified_gaps(
            &primary_artifact,
            &affected_repositories,
            &adr_list,
            &doc_list,
            &pr_list,
            clean_target_id,
        );

        let prioritized_gaps = prioritize_knowledge_gaps(
            &primary_artifact,
            &affected_repositories,
            &adr_list,
            &doc_list,
            &pr_list,
            &target_aspects,
        );

        let investigation_steps = generate_investigation_steps(
            clean_target_id,
            &title,
            &primary_artifact,
            &target_aspects,
            &recommended_reading,
            &affected_repositories,
        );

        let unknowns = compute_unknowns(
            &primary_artifact,
            &affected_repositories,
            &adr_list,
            &doc_list,
            &pr_list,
            &target_aspects,
        );

        let investigation_status = compute_investigation_status(
            primary_artifact.is_some(),
            &affected_repositories,
            &adr_list,
            &doc_list,
            &pr_list,
            &commit_list,
            &target_aspects,
        );

        let ai_guidance = Some(generate_ai_guidance(
            &primary_artifact,
            &target_aspects,
            &adr_list,
            &pr_list,
            &recommended_reading,
        ));

        let ai_guidance_bullets = generate_downstream_ai_guidance();

        let engineering_readiness = compute_readiness(
            &target_aspects,
            &completeness,
            primary_artifact.is_some(),
        );

        let hints = generate_implementation_hints(
            &primary_artifact,
            &adr_list,
            &pr_list,
            &api_list,
            &affected_repositories,
            &recommended_reading,
        );

        let suggested_next_actions = generate_next_actions(
            clean_target_id,
            &title,
            &primary_artifact,
            &recommended_reading,
            active_repo,
        );

        let source_info = build_source_info(
            &primary_artifact,
            active_repo,
            clean_target_id,
        );

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
        let t_assembly = start_assembly.elapsed().as_millis();
        let total_ms = start_total.elapsed().as_millis();

        let telemetry = if options.profile {
            Some(ContextTelemetry {
                primary_resolution_ms: t_primary,
                hop1_traversal_ms: t_hop1,
                hop2_traversal_ms: t_hop2,
                repo_fts_search_ms: t_repo_fts,
                candidate_ranking_ms: t_ranking,
                artifact_hydration_ms: t_hydration,
                prompt_assembly_ms: t_assembly,
                total_ms,
                candidate_headers_count: total_candidate_headers,
                hydrated_artifacts_count: total_hydrated,
            })
        } else {
            None
        };

        Ok(ContextPackage {
            target_kind: normalized_kind,
            target_id: clean_target_id.to_string(),
            target_aspects,
            primary_artifact,
            title,
            status,
            repository: final_repo,
            description,
            overview_summary,
            mission,
            known_facts,
            evidence_ranking,
            implementation_areas,
            understanding,
            hypothesis,
            execution_queue,
            risks,
            classified_gaps,
            prioritized_gaps,
            investigation_steps,
            unknowns,
            investigation_status,
            ai_guidance,
            ai_guidance_bullets,
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
            telemetry,
        })
    }

    fn add_candidate_header(
        &self,
        map: &mut HashMap<String, (ArtifactHeader, f64, Option<String>, bool)>,
        header: ArtifactHeader,
        weight: f64,
        label: Option<String>,
        is_direct: bool,
    ) {
        let entry = map.entry(header.id.clone()).or_insert_with(|| (header.clone(), 0.0, label.clone(), is_direct));
        entry.1 += weight;
        if is_direct {
            entry.3 = true;
        }
        if entry.2.is_none() && label.is_some() {
            entry.2 = label;
        }
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

fn is_architecture_decision_header(header: &ArtifactHeader) -> bool {
    let lower_id = header.source_id.to_lowercase();
    let lower_title = header.title.to_lowercase();
    if lower_id.starts_with("adr-") || lower_id.contains("adr") || lower_title.contains("adr") {
        return true;
    }
    if matches!(header.kind, ArtifactKind::Design | ArtifactKind::Specification) {
        return true;
    }
    if lower_title.contains("architecture") || lower_title.contains("design decision") {
        return true;
    }
    false
}

fn is_api_header(header: &ArtifactHeader) -> bool {
    if matches!(header.kind, ArtifactKind::Component) {
        return true;
    }
    let lower_title = header.title.to_lowercase();
    let lower_id = header.source_id.to_lowercase();
    if lower_title.contains("api") || lower_id.contains("api") || lower_title.contains("endpoint") || lower_title.contains("openapi") || lower_title.contains("graphql") {
        return true;
    }
    false
}

fn is_closed_or_historical_header(header: &ArtifactHeader) -> bool {
    if let Some(status) = header.metadata.get("status").and_then(|v| v.as_str()) {
        let s = status.to_lowercase();
        if s == "closed" || s == "merged" || s == "done" || s == "resolved" {
            return true;
        }
    }
    if let Some(state) = header.metadata.get("state").and_then(|v| v.as_str()) {
        let s = state.to_lowercase();
        if s == "closed" || s == "merged" || s == "done" || s == "resolved" {
            return true;
        }
    }
    matches!(header.kind, ArtifactKind::Commit | ArtifactKind::Release)
}

fn build_recommended_reading_headers(
    candidates: &[(ArtifactHeader, f64, String, bool)],
    active_repo: Option<&str>,
    limit: usize,
) -> Vec<RecommendedItem> {
    let mut scored_items: Vec<(RecommendedItem, f64)> = Vec::new();

    for (header, base_score, label, is_direct) in candidates {
        let mut r_score = *base_score;
        let mut reason = format!("Context reference ({})", label);

        if is_architecture_decision_header(header) {
            r_score += 50.0;
            reason = "Architecture Decision (ADR) guideline".to_string();
        } else if matches!(header.kind, ArtifactKind::Design | ArtifactKind::Specification) {
            r_score += 45.0;
            reason = "Design Specification Document".to_string();
        } else if is_api_header(header) {
            r_score += 40.0;
            reason = "API Contract Specification".to_string();
        } else if matches!(header.kind, ArtifactKind::PullRequest | ArtifactKind::PullRequestReview) {
            r_score += 30.0;
            reason = "Prior Implementation PR".to_string();
        } else if matches!(header.kind, ArtifactKind::Ticket | ArtifactKind::Issue) {
            r_score += 20.0;
            reason = "Related Issue / Ticket".to_string();
        } else if matches!(header.kind, ArtifactKind::Commit) {
            r_score += 15.0;
            reason = "Related Commit History".to_string();
        }

        if *is_direct {
            r_score += 20.0;
        }

        if let Some(repo) = active_repo {
            if header.repository.as_deref() == Some(repo) {
                r_score += 10.0;
            }
        }

        let star_rating = if r_score >= 100.0 {
            "★★★★★".to_string()
        } else if r_score >= 70.0 {
            "★★★★☆".to_string()
        } else if r_score >= 45.0 {
            "★★★☆☆".to_string()
        } else if r_score >= 25.0 {
            "★★☆☆☆".to_string()
        } else {
            "★☆☆☆☆".to_string()
        };

        let item = RecommendedItem {
            id: header.id.clone(),
            source_id: header.source_id.clone(),
            title: header.title.clone(),
            kind: header.kind.to_string(),
            relationship_label: label.clone(),
            score: r_score,
            star_rating,
            reason,
        };

        scored_items.push((item, r_score));
    }

    scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored_items.into_iter().map(|(item, _)| item).take(limit).collect()
}

pub fn score_commit_relevance(header: &ArtifactHeader, target_id: &str, is_direct: bool) -> f64 {
    let mut score = if is_direct { 20.0 } else { 5.0 };
    let lower_title = header.title.to_lowercase();
    let lower_target = target_id.to_lowercase();

    if !lower_target.is_empty() && lower_title.contains(&lower_target) {
        score += 100.0;
    }
    if header.metadata.get("is_merge").and_then(|v| v.as_bool()).unwrap_or(false) {
        score += 40.0;
    }
    if lower_title.contains("merge pull request") || lower_title.contains("merge pr") {
        score += 30.0;
    }
    if lower_title.contains("bump") || lower_title.contains("dependabot") || lower_title.contains("deps") {
        score -= 80.0;
    }
    score
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

fn compute_completeness(
    target_aspects: &HashSet<DomainAspect>,
    has_primary: bool,
    repos: &[String],
    adrs: &[LabeledArtifact],
    docs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    _tickets: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    commits: &[LabeledArtifact],
    history: &[LabeledArtifact],
) -> CompletenessReport {
    let mut available = Vec::new();
    let mut missing = Vec::new();
    let mut category_scores = Vec::new();

    let is_code_target = target_aspects.contains(&DomainAspect::CodeImplementation)
        || target_aspects.contains(&DomainAspect::TaskTracking);

    // Business Context
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

    let mut total_possible: u32 = 0;
    let mut total_score: u32 = 0;

    if has_primary {
        total_score += 20;
    }
    total_possible += 20;

    // 1. Repository
    if is_code_target {
        total_possible += 20;
        if !repos.is_empty() {
            total_score += 20;
            available.push(CategoryAvailability {
                category: "Repository".to_string(),
                is_available: true,
                count: repos.len(),
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
    }

    // 2. Architecture Decision (ADR) / Specs
    if target_aspects.contains(&DomainAspect::Architecture) || target_aspects.contains(&DomainAspect::Documentation) || is_code_target {
        total_possible += 20;
        if !adrs.is_empty() || !docs.is_empty() {
            total_score += 20;
            available.push(CategoryAvailability {
                category: "Architecture & Specs".to_string(),
                is_available: true,
                count: adrs.len() + docs.len(),
                label: "Architecture & Specs".to_string(),
            });
        } else {
            missing.push(CategoryAvailability {
                category: "Architecture & Specs".to_string(),
                is_available: false,
                count: 0,
                label: "Architecture & Specs".to_string(),
            });
        }
    }

    // 3. Documentation
    if !docs.is_empty() {
        available.push(CategoryAvailability {
            category: "Documentation".to_string(),
            is_available: true,
            count: docs.len(),
            label: "Documentation".to_string(),
        });
    }

    // 4. Pull Requests
    if is_code_target {
        total_possible += 20;
        if !prs.is_empty() {
            total_score += 20;
            available.push(CategoryAvailability {
                category: "Previous PRs".to_string(),
                is_available: true,
                count: prs.len(),
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
    }

    // 5. Commits
    if is_code_target {
        total_possible += 20;
        if !commits.is_empty() || !history.is_empty() {
            total_score += 20;
            available.push(CategoryAvailability {
                category: "Commit History".to_string(),
                is_available: true,
                count: commits.len() + history.len(),
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
    }

    let score_percentage = if total_possible > 0 {
        ((total_score * 100) / total_possible).min(100) as u8
    } else {
        100
    };

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

fn compute_readiness(
    target_aspects: &HashSet<DomainAspect>,
    completeness: &CompletenessReport,
    _has_primary: bool,
) -> EngineeringReadiness {
    let is_code_target = target_aspects.contains(&DomainAspect::CodeImplementation)
        || target_aspects.contains(&DomainAspect::TaskTracking);

    let (status_label, readiness_summary) = if !is_code_target {
        (
            "Knowledge context active.".to_string(),
            "Atlas assembled intent-driven context for non-code artifact.".to_string(),
        )
    } else if completeness.score_percentage >= 80 {
        (
            "Ready for implementation.".to_string(),
            "Atlas found comprehensive engineering context and architectural references.".to_string(),
        )
    } else if completeness.score_percentage >= 50 {
        (
            "Ready for implementation.".to_string(),
            "Atlas found sufficient implementation context, but architectural references are incomplete.".to_string(),
        )
    } else {
        (
            "Needs architectural clarification.".to_string(),
            "Atlas found initial context, but key architectural and implementation artifacts are missing.".to_string(),
        )
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

fn generate_overview_summary(
    primary: &Option<KnowledgeArtifact>,
    title: &str,
    target_id: &str,
    aspects: &HashSet<DomainAspect>,
) -> String {
    if let Some(ref art) = primary {
        let first_body_line = art.body.lines().next().unwrap_or("").trim();
        let body_preview = if !first_body_line.is_empty() {
            let clean_line = first_body_line.trim_start_matches('#').trim();
            if clean_line.chars().count() > 150 {
                format!(" Summary excerpt: {}", &clean_line[..150])
            } else {
                format!(" Summary excerpt: {}", clean_line)
            }
        } else {
            String::new()
        };

        let kind_str = art.kind.to_string();
        let provider_str = &art.provider;
        format!(
            "'{}' ({}) represents a {} artifact indexed from {}.{}",
            title, target_id, kind_str, provider_str, body_preview
        )
    } else {
        let aspect_desc = if aspects.contains(&DomainAspect::Collaboration) {
            "collaboration document or retrospective"
        } else if aspects.contains(&DomainAspect::Documentation) {
            "knowledge specification document"
        } else if aspects.contains(&DomainAspect::Architecture) {
            "architectural decision proposal"
        } else if aspects.contains(&DomainAspect::Design) {
            "design system asset"
        } else {
            "engineering artifact"
        };
        format!(
            "Target '{}' ({}) represents an un-indexed {} context query within Atlas.",
            title, target_id, aspect_desc
        )
    }
}

fn compute_investigation_status(
    has_primary: bool,
    repos: &[String],
    adrs: &[LabeledArtifact],
    docs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    commits: &[LabeledArtifact],
    aspects: &HashSet<DomainAspect>,
) -> Vec<StatusCheck> {
    let mut status = Vec::new();

    status.push(StatusCheck {
        label: "Business requirements available".to_string(),
        is_available: has_primary,
    });

    let has_docs = !docs.is_empty() || !adrs.is_empty();
    status.push(StatusCheck {
        label: "Related documentation and specifications available".to_string(),
        is_available: has_docs,
    });

    let has_impl = !prs.is_empty() || !commits.is_empty();
    status.push(StatusCheck {
        label: "Previous implementation pull requests & commits found".to_string(),
        is_available: has_impl,
    });

    if aspects.contains(&DomainAspect::CodeImplementation) || aspects.contains(&DomainAspect::TaskTracking) {
        status.push(StatusCheck {
            label: "Target repository identified".to_string(),
            is_available: !repos.is_empty(),
        });
        status.push(StatusCheck {
            label: "Architecture decision references attached".to_string(),
            is_available: !adrs.is_empty(),
        });
    }

    status
}

fn generate_ai_guidance(
    _primary: &Option<KnowledgeArtifact>,
    aspects: &HashSet<DomainAspect>,
    adrs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    reading: &[RecommendedItem],
) -> AiGuidance {
    if aspects.contains(&DomainAspect::Collaboration) {
        AiGuidance {
            artifact_nature: "Collaboration Document / Retrospective".to_string(),
            exploration_strategy: "This artifact is a team retrospective or discussion note. Implementation history is irrelevant. Focus on extracting action items and inspecting linked tickets.".to_string(),
        }
    } else if aspects.contains(&DomainAspect::Architecture) || !adrs.is_empty() {
        let first_adr = reading.iter().find(|r| r.reason.contains("Architecture") || r.source_id.to_lowercase().contains("adr"));
        let adr_hint = first_adr.map(|a| format!(" Read {} first before drafting implementation details.", a.source_id)).unwrap_or_default();
        AiGuidance {
            artifact_nature: "Architecture Specification".to_string(),
            exploration_strategy: format!("This artifact is specification and architecture driven.{} Inspect architectural decision records (ADRs) to satisfy design constraints.", adr_hint),
        }
    } else if aspects.contains(&DomainAspect::Documentation) {
        AiGuidance {
            artifact_nature: "Knowledge Base Specification".to_string(),
            exploration_strategy: "This artifact is a technical documentation reference. Inspect linked design documents and OpenAPI specifications to confirm system constraints.".to_string(),
        }
    } else if aspects.contains(&DomainAspect::Design) {
        AiGuidance {
            artifact_nature: "UI/UX Design Asset".to_string(),
            exploration_strategy: "This artifact is a design asset or mockup. Inspect linked UI components and endpoint contracts before writing code.".to_string(),
        }
    } else {
        let pr_hint = if !prs.is_empty() {
            " Review prior pull request patterns for guidance."
        } else {
            ""
        };
        AiGuidance {
            artifact_nature: "Code Implementation Task".to_string(),
            exploration_strategy: format!("This artifact is an engineering implementation task.{} Inspect target repository code locations before generating code.", pr_hint),
        }
    }
}



fn generate_investigation_steps(
    target_id: &str,
    title: &str,
    primary: &Option<KnowledgeArtifact>,
    _aspects: &HashSet<DomainAspect>,
    reading: &[RecommendedItem],
    repos: &[String],
) -> Vec<InvestigationStep> {
    let mut steps = Vec::new();
    let mut step_num = 1;

    if let Some(item) = reading.first() {
        steps.push(InvestigationStep {
            step_number: step_num,
            goal: "Understand primary business rules and requirements.".to_string(),
            inspect_target: format!("{} ({})", item.source_id, item.title),
            expected_outcome: format!("Understand eligibility rules, mechanics, and customer flow ({})", item.reason),
        });
        step_num += 1;
    } else if primary.is_some() {
        steps.push(InvestigationStep {
            step_number: step_num,
            goal: "Understand primary requirements.".to_string(),
            inspect_target: target_id.to_string(),
            expected_outcome: "Understand core feature description and acceptance criteria.".to_string(),
        });
        step_num += 1;
    }

    if let Some(item) = reading.iter().nth(1) {
        steps.push(InvestigationStep {
            step_number: step_num,
            goal: "Locate prior implementation pattern or reference.".to_string(),
            inspect_target: format!("{} ({})", item.source_id, item.title),
            expected_outcome: "Reuse existing implementation patterns and mechanics.".to_string(),
        });
        step_num += 1;
    }

    if let Some(repo) = repos.first() {
        let search_kw = extract_search_terms(title, &[]);
        steps.push(InvestigationStep {
            step_number: step_num,
            goal: "Locate affected code modules and entry points.".to_string(),
            inspect_target: format!("Repository {} (Keywords: '{}')", repo, search_kw),
            expected_outcome: "Identify candidate implementation locations and source files.".to_string(),
        });
        step_num += 1;
    }

    steps.push(InvestigationStep {
        step_number: step_num,
        goal: "Generate technical implementation checklist.".to_string(),
        inspect_target: "Target Codebase & Unit Tests".to_string(),
        expected_outcome: "Draft step-by-step code modifications with minimal reasoning overhead.".to_string(),
    });

    steps
}

fn compute_unknowns(
    primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    adrs: &[LabeledArtifact],
    docs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    aspects: &HashSet<DomainAspect>,
) -> Vec<String> {
    let mut unknowns = Vec::new();

    if primary.is_none() {
        unknowns.push("Primary artifact is un-indexed in local Atlas database.".to_string());
    }

    if aspects.contains(&DomainAspect::CodeImplementation) || aspects.contains(&DomainAspect::TaskTracking) {
        if repos.is_empty() {
            unknowns.push("Target repository is not explicitly identified.".to_string());
        }
        if adrs.is_empty() {
            unknowns.push("Architecture Decision Records (ADRs) are missing or not referenced.".to_string());
        }
        if prs.is_empty() {
            unknowns.push("Previous pull requests and commits are not linked to this target.".to_string());
        }
    }

    if docs.is_empty() {
        unknowns.push("Technical specification or knowledge base document is not indexed.".to_string());
    }

    unknowns
}

fn build_mission(
    title: &str,
    primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    aspects: &HashSet<DomainAspect>,
) -> Mission {
    let goal = format!("Implement {}", title);

    let objective = if let Some(art) = primary {
        let first_body = art.body.lines().next().unwrap_or("").trim();
        let clean = first_body.trim_start_matches('#').trim();
        if !clean.is_empty() {
            if clean.chars().count() > 140 {
                format!("{}...", &clean[..140])
            } else {
                clean.to_string()
            }
        } else {
            format!("Execute requirements and acceptance criteria defined in {}", art.source_id)
        }
    } else {
        "Allow customers to execute target feature mechanics with system validation.".to_string()
    };

    let complexity = if aspects.contains(&DomainAspect::Collaboration) {
        "Low (Process / Non-Code)".to_string()
    } else if aspects.contains(&DomainAspect::Architecture) || repos.len() > 1 {
        "High".to_string()
    } else {
        "Medium".to_string()
    };

    let repository = repos.first().cloned().unwrap_or_else(|| "N/A".to_string());

    Mission {
        target_feature: format!("{} {}", if let Some(art) = primary { art.source_id.clone() } else { String::new() }, title).trim().to_string(),
        business_objective: objective.clone(),
        expected_outcome: if let Some(art) = primary {
            format!("Derived from retrieved requirements in {}", art.source_id)
        } else {
            "Derived from retrieved business requirements".to_string()
        },
        estimated_complexity: complexity.clone(),
        goal,
        objective,
        complexity,
        repository,
    }
}

fn build_current_understanding(
    primary: &Option<KnowledgeArtifact>,
    reading: &[RecommendedItem],
    apis: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    aspects: &HashSet<DomainAspect>,
) -> CurrentUnderstanding {
    let mut business_rules = Vec::new();
    let mut affected_domains = Vec::new();
    let mut known_constraints = Vec::new();

    if let Some(art) = primary {
        for line in art.body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with("1.") || trimmed.starts_with("2.") {
                let rule_clean = trimmed.trim_start_matches(|c: char| c == '-' || c == '*' || c == '.' || c.is_numeric()).trim();
                if !rule_clean.is_empty() && rule_clean.len() < 120 && !rule_clean.starts_with("http") {
                    business_rules.push(rule_clean.to_string());
                    if business_rules.len() >= 3 {
                        break;
                    }
                }
            }
        }
        for tag in &art.tags {
            let clean_tag = tag.trim_start_matches("project:").trim_start_matches("area:");
            if !clean_tag.is_empty() && !affected_domains.contains(&clean_tag.to_string()) {
                affected_domains.push(clean_tag.to_string());
            }
        }
    }

    if business_rules.is_empty() {
        business_rules.push("Primary feature requirements and acceptance criteria defined.".to_string());
        if !reading.is_empty() {
            business_rules.push(format!("Sub-system integration defined by {}", reading[0].source_id));
        }
    }

    if affected_domains.is_empty() {
        if aspects.contains(&DomainAspect::Collaboration) {
            affected_domains.push("Team Governance".to_string());
            affected_domains.push("Task Tracking".to_string());
        } else {
            affected_domains.push("Core Business Domain".to_string());
            affected_domains.push("Application Integration".to_string());
        }
    }

    if !apis.is_empty() {
        known_constraints.push("Requires adherence to strict API contract schema.".to_string());
    }
    if !prs.is_empty() {
        known_constraints.push("Requires backward compatibility with prior PR implementation.".to_string());
    }
    if aspects.contains(&DomainAspect::Collaboration) {
        known_constraints.push("Non-code action items; no codebase modification required.".to_string());
    } else {
        known_constraints.push("Validation rules and error responses must match system specification.".to_string());
    }

    CurrentUnderstanding {
        business_rules,
        affected_domains,
        known_constraints,
    }
}

fn infer_advanced_hypothesis(
    primary: &Option<KnowledgeArtifact>,
    aspects: &HashSet<DomainAspect>,
    repos: &[String],
    reading: &[RecommendedItem],
    adrs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
) -> ImplementationHypothesis {
    let mut scope = Vec::new();
    let mut primary_flow = Vec::new();
    let mut likely_modified_modules = Vec::new();
    let mut potential_integrations = Vec::new();

    if aspects.contains(&DomainAspect::Collaboration) {
        scope.push(ScopeItem { area: "Team Retrospective / Action Items".to_string(), is_likely: true });
        scope.push(ScopeItem { area: "Referenced Task Tracking".to_string(), is_likely: true });
        scope.push(ScopeItem { area: "Codebase Refactoring".to_string(), is_likely: false });

        primary_flow.push("Retro Discussion".to_string());
        primary_flow.push("Action Item Extraction".to_string());
        primary_flow.push("Task Assignment".to_string());

        likely_modified_modules.push(ModuleRating { module_name: "Task Board / Tracking".to_string(), rating_stars: "★★★★★".to_string() });
        potential_integrations.push("Issue Tracker".to_string());

        return ImplementationHypothesis {
            scope,
            primary_flow,
            likely_modified_modules,
            potential_integrations,
            impact_level: "Low".to_string(),
            estimated_components: "0 (Process / Non-Code)".to_string(),
            confidence: "High".to_string(),
        };
    }

    scope.push(ScopeItem { area: "Core Business Domain Logic".to_string(), is_likely: true });
    scope.push(ScopeItem { area: "Module / Component Rules".to_string(), is_likely: true });
    scope.push(ScopeItem { area: "API Contract / Interface".to_string(), is_likely: !apis.is_empty() || !adrs.is_empty() });
    scope.push(ScopeItem { area: "Database / Schema Migration".to_string(), is_likely: false });

    primary_flow.push("Request Entry / Trigger".to_string());
    primary_flow.push("Domain Eligibility & Rule Validation".to_string());
    primary_flow.push("Sub-system State Transformation".to_string());
    primary_flow.push("API Response / Persistence".to_string());

    likely_modified_modules.push(ModuleRating { module_name: "Core Domain Engine".to_string(), rating_stars: "★★★★★".to_string() });
    likely_modified_modules.push(ModuleRating { module_name: "Validation Module".to_string(), rating_stars: "★★★★☆".to_string() });
    if !apis.is_empty() {
        likely_modified_modules.push(ModuleRating { module_name: "API Service Interface".to_string(), rating_stars: "★★★★☆".to_string() });
    }

    potential_integrations.push("Domain Business Rules".to_string());
    potential_integrations.push("Validation Pipeline".to_string());
    if !apis.is_empty() {
        potential_integrations.push("External API Contract".to_string());
    }

    let impact = if !repos.is_empty() && (!prs.is_empty() || !adrs.is_empty()) {
        "Medium"
    } else if !repos.is_empty() {
        "Medium"
    } else {
        "High"
    };

    let est = if !repos.is_empty() { "2-4 Components" } else { "3-6 Components" };
    let conf = if primary.is_some() && !repos.is_empty() { "High" } else { "Moderate" };

    ImplementationHypothesis {
        scope,
        primary_flow,
        likely_modified_modules,
        potential_integrations,
        impact_level: impact.to_string(),
        estimated_components: est.to_string(),
        confidence: conf.to_string(),
    }
}

fn build_execution_queue(
    target_id: &str,
    title: &str,
    primary: &Option<KnowledgeArtifact>,
    reading: &[RecommendedItem],
    repos: &[String],
    prs: &[LabeledArtifact],
    _aspects: &HashSet<DomainAspect>,
) -> Vec<QueueStep> {
    let mut steps = Vec::new();
    let mut idx = 1;
    let mut total = 4;

    if reading.iter().any(|r| r.reason.contains("Architecture") || r.kind.contains("Doc")) {
        total += 1;
    }
    if !prs.is_empty() {
        total += 1;
    }

    if let Some(item) = reading.first() {
        steps.push(QueueStep {
            step_index: idx,
            total_steps: total,
            category: "Required".to_string(),
            title: "Read technical specification".to_string(),
            artifact_label: Some(format!("{} ({})", item.source_id, item.title)),
            reason: item.reason.clone(),
            command: Some(format!("atx context \"{}\"", item.source_id)),
            status: "Pending".to_string(),
        });
        idx += 1;
    } else {
        steps.push(QueueStep {
            step_index: idx,
            total_steps: total,
            category: "Required".to_string(),
            title: "Inspect primary artifact requirements".to_string(),
            artifact_label: Some(target_id.to_string()),
            reason: "Extract core acceptance criteria and feature scope.".to_string(),
            command: Some(format!("atx artifact {}", target_id)),
            status: "Pending".to_string(),
        });
        idx += 1;
    }

    if let Some(item) = reading.iter().skip(1).find(|r| r.reason.contains("API") || r.reason.contains("Architecture")) {
        steps.push(QueueStep {
            step_index: idx,
            total_steps: total,
            category: "Required".to_string(),
            title: "Inspect API contracts and architecture guidelines".to_string(),
            artifact_label: Some(format!("{} ({})", item.source_id, item.title)),
            reason: item.reason.clone(),
            command: Some(format!("atx context \"{}\"", item.source_id)),
            status: "Pending".to_string(),
        });
        idx += 1;
    }

    if let Some(repo) = repos.first() {
        steps.push(QueueStep {
            step_index: idx,
            total_steps: total,
            category: "Required".to_string(),
            title: "Locate repository implementation pattern".to_string(),
            artifact_label: Some(format!("Repository {}", repo)),
            reason: "Identify candidate source files and code structure.".to_string(),
            command: Some(format!("atx repository {}", repo)),
            status: "Pending".to_string(),
        });
        idx += 1;
    }

    let search_kw = extract_search_terms(title, &[]);
    if !search_kw.is_empty() {
        steps.push(QueueStep {
            step_index: idx,
            total_steps: total,
            category: "Required".to_string(),
            title: format!("Search existing implementation matching '{}'", search_kw),
            artifact_label: None,
            reason: "Find existing code mechanics and reusable functions.".to_string(),
            command: Some(format!("atx search \"{}\"", search_kw)),
            status: "Pending".to_string(),
        });
        idx += 1;
    }

    steps.push(QueueStep {
        step_index: idx,
        total_steps: total,
        category: "Optional".to_string(),
        title: "Inspect related tickets for additional context".to_string(),
        artifact_label: None,
        reason: "Supplemental context from related work items.".to_string(),
        command: None,
        status: "Pending".to_string(),
    });

    steps
}

fn build_known_facts(
    _primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    adrs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    commits: &[LabeledArtifact],
    other_related: &[LabeledArtifact],
) -> Vec<KnownFact> {
    let mut facts = Vec::new();

    if let Some(repo) = repos.first() {
        facts.push(KnownFact {
            statement: format!("Target repository identified ({})", repo),
            is_verified: true,
            source_artifact: Some(repo.clone()),
        });
    } else {
        facts.push(KnownFact {
            statement: "Target repository not explicitly identified".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    if let Some(adr) = adrs.first() {
        facts.push(KnownFact {
            statement: format!("Technical specification indexed ({})", adr.artifact.source_id),
            is_verified: true,
            source_artifact: Some(adr.artifact.source_id.clone()),
        });
    } else {
        facts.push(KnownFact {
            statement: "No technical specification or ADR indexed".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    if let Some(api) = apis.first() {
        facts.push(KnownFact {
            statement: format!("API contracts available ({})", api.artifact.source_id),
            is_verified: true,
            source_artifact: Some(api.artifact.source_id.clone()),
        });
    } else {
        facts.push(KnownFact {
            statement: "No API contracts found".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    let ticket_count = other_related.iter().filter(|a| matches!(a.artifact.kind, ArtifactKind::Ticket | ArtifactKind::Issue)).count();
    if ticket_count > 0 {
        facts.push(KnownFact {
            statement: format!("Related implementation tickets exist ({} ticket(s))", ticket_count),
            is_verified: true,
            source_artifact: None,
        });
    } else {
        facts.push(KnownFact {
            statement: "No related implementation tickets found".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    if !prs.is_empty() {
        facts.push(KnownFact {
            statement: format!("Linked pull requests found ({} PR(s))", prs.len()),
            is_verified: true,
            source_artifact: None,
        });
    } else {
        facts.push(KnownFact {
            statement: "No pull requests linked".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    if !commits.is_empty() {
        facts.push(KnownFact {
            statement: format!("Linked commits found ({} commit(s))", commits.len()),
            is_verified: true,
            source_artifact: None,
        });
    } else {
        facts.push(KnownFact {
            statement: "No commits linked".to_string(),
            is_verified: false,
            source_artifact: None,
        });
    }

    facts
}

fn build_evidence_ranking(
    adrs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    other_related: &[LabeledArtifact],
    reading: &[RecommendedItem],
) -> Vec<EvidenceItem> {
    let mut items = Vec::new();
    let mut seen_ids = HashSet::new();

    for adr in adrs {
        if seen_ids.insert(adr.artifact.source_id.clone()) {
            items.push(EvidenceItem {
                artifact_id: adr.artifact.source_id.clone(),
                title: adr.artifact.title.clone(),
                kind: "Specification".to_string(),
                confidence_level: "High Confidence".to_string(),
                star_rating: "★★★★★".to_string(),
                reason: "Direct technical specification / ADR".to_string(),
            });
        }
    }

    for api in apis {
        if seen_ids.insert(api.artifact.source_id.clone()) {
            items.push(EvidenceItem {
                artifact_id: api.artifact.source_id.clone(),
                title: api.artifact.title.clone(),
                kind: "API Contract".to_string(),
                confidence_level: "High Confidence".to_string(),
                star_rating: "★★★★☆".to_string(),
                reason: "Defines integration & contract boundaries".to_string(),
            });
        }
    }

    for pr in prs {
        if seen_ids.insert(pr.artifact.source_id.clone()) {
            items.push(EvidenceItem {
                artifact_id: pr.artifact.source_id.clone(),
                title: pr.artifact.title.clone(),
                kind: "Pull Request".to_string(),
                confidence_level: "High Confidence".to_string(),
                star_rating: "★★★★☆".to_string(),
                reason: "Direct implementation PR".to_string(),
            });
        }
    }

    for rel in other_related {
        if seen_ids.insert(rel.artifact.source_id.clone()) {
            let is_direct = rel.is_direct_graph;
            let conf = if is_direct { "Medium Confidence" } else { "Low Confidence" };
            let stars = if is_direct { "★★★☆☆" } else { "★☆☆☆☆" };
            let reason = if is_direct {
                "Related feature with similar promotion mechanics"
            } else {
                "Indirect relationship only"
            };

            items.push(EvidenceItem {
                artifact_id: rel.artifact.source_id.clone(),
                title: rel.artifact.title.clone(),
                kind: format!("{:?}", rel.artifact.kind),
                confidence_level: conf.to_string(),
                star_rating: stars.to_string(),
                reason: reason.to_string(),
            });
        }
    }

    for rec in reading {
        if seen_ids.insert(rec.source_id.clone()) {
            items.push(EvidenceItem {
                artifact_id: rec.source_id.clone(),
                title: rec.title.clone(),
                kind: rec.kind.clone(),
                confidence_level: "Medium Confidence".to_string(),
                star_rating: rec.star_rating.clone(),
                reason: rec.reason.clone(),
            });
        }
    }

    items
}

fn infer_implementation_areas(
    primary: &Option<KnowledgeArtifact>,
    _aspects: &HashSet<DomainAspect>,
    repos: &[String],
    adrs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
) -> PossibleImplementationAreas {
    let mut business_rules = Vec::new();
    let mut potential_components = Vec::new();

    if let Some(art) = primary {
        for line in art.body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with("1.") || trimmed.starts_with("2.") {
                let rule_clean = trimmed.trim_start_matches(|c: char| c == '-' || c == '*' || c == '.' || c.is_numeric()).trim();
                if !rule_clean.is_empty() && rule_clean.len() < 100 && !rule_clean.starts_with("http") {
                    business_rules.push(rule_clean.to_string());
                    if business_rules.len() >= 3 {
                        break;
                    }
                }
            }
        }
    }

    if business_rules.is_empty() {
        business_rules.push("Promotion eligibility".to_string());
        business_rules.push("Voucher redemption mechanics".to_string());
        business_rules.push("Campaign configuration limits".to_string());
    }

    potential_components.push("Promotion Engine".to_string());
    potential_components.push("Validation Layer".to_string());
    potential_components.push("Campaign Configuration".to_string());

    if !apis.is_empty() {
        potential_components.push("API Service Interface".to_string());
    }

    let impact = if !repos.is_empty() && (!adrs.is_empty() || !apis.is_empty()) {
        "Medium"
    } else if !repos.is_empty() {
        "Medium"
    } else {
        "High"
    };

    let conf = if primary.is_some() && (!adrs.is_empty() || !apis.is_empty()) {
        "High"
    } else {
        "Moderate"
    };

    PossibleImplementationAreas {
        business_rules,
        potential_components,
        impact_level: impact.to_string(),
        confidence: conf.to_string(),
        uncertainty_note: "Candidate implementation domains derived from indexed evidence, not guaranteed execution flow.".to_string(),
    }
}

fn build_dependency_aware_queue(
    target_id: &str,
    title: &str,
    _primary: &Option<KnowledgeArtifact>,
    reading: &[RecommendedItem],
    repos: &[String],
    _prs: &[LabeledArtifact],
    adrs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    other_related: &[LabeledArtifact],
) -> Vec<QueueStep> {
    let mut steps = Vec::new();
    let mut step_index = 1;

    // ① Required: Technical Specification (TSD / ADR)
    if let Some(adr) = adrs.first() {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: format!("Technical Specification ({})", adr.artifact.source_id),
            artifact_label: Some(adr.artifact.title.clone()),
            reason: "Contains business mechanics and requirements.".to_string(),
            command: Some(format!("atx context {}", adr.artifact.source_id)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    } else if let Some(rec) = reading.iter().find(|r| r.reason.contains("Architecture") || r.kind.contains("Doc")) {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: format!("Technical Specification ({})", rec.source_id),
            artifact_label: Some(rec.title.clone()),
            reason: "Contains business mechanics.".to_string(),
            command: Some(format!("atx context {}", rec.source_id)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    } else {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: format!("Primary Feature Specification ({})", target_id),
            artifact_label: Some(title.to_string()),
            reason: "Inspect core business mechanics and requirements.".to_string(),
            command: Some(format!("atx artifact {}", target_id)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    }

    // ② Required: API Contract
    if let Some(api) = apis.first() {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: format!("API Contract ({})", api.artifact.source_id),
            artifact_label: Some(api.artifact.title.clone()),
            reason: "Defines integration boundaries.".to_string(),
            command: Some(format!("atx context {}", api.artifact.source_id)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    }

    // ③ Required: Repository
    if let Some(repo) = repos.first() {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: format!("Repository ({})", repo),
            artifact_label: None,
            reason: "Locate implementation entry points.".to_string(),
            command: Some(format!("atx repository {}", repo)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    }

    // ④ Required: Existing Search Results / PRs
    let search_kw = extract_search_terms(title, &[]);
    if !search_kw.is_empty() {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Required".to_string(),
            title: "Existing Search Results".to_string(),
            artifact_label: None,
            reason: "Reuse implementation patterns.".to_string(),
            command: Some(format!("atx search \"{}\"", search_kw)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    }

    // ⑤ Optional: Related Tickets
    if let Some(rel) = other_related.first() {
        steps.push(QueueStep {
            step_index,
            total_steps: 0,
            category: "Optional".to_string(),
            title: format!("Related Ticket ({})", rel.artifact.source_id),
            artifact_label: Some(rel.artifact.title.clone()),
            reason: "Historical references only.".to_string(),
            command: Some(format!("atx artifact {}", rel.artifact.source_id)),
            status: "Pending".to_string(),
        });
        step_index += 1;
    }

    let total = steps.len();
    for s in &mut steps {
        s.total_steps = total;
    }

    steps
}

fn build_classified_gaps(
    _primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    adrs: &[LabeledArtifact],
    _docs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    clean_target_id: &str,
) -> Vec<ClassifiedKnowledgeGap> {
    let mut gaps = Vec::new();

    let target_kw = extract_search_terms(clean_target_id, &[]);
    let query_term = if !target_kw.is_empty() { &target_kw } else { clean_target_id };

    if adrs.is_empty() {
        gaps.push(ClassifiedKnowledgeGap {
            severity: "HIGH".to_string(),
            gap_type: "Architecture Decision".to_string(),
            impact: "Implementation boundaries remain uncertain.".to_string(),
            suggested_retrieval: format!("atx search architecture {}", query_term),
        });
    }

    if repos.is_empty() {
        gaps.push(ClassifiedKnowledgeGap {
            severity: "HIGH".to_string(),
            gap_type: "Target Repository Link".to_string(),
            impact: "Target codebase location is unverified.".to_string(),
            suggested_retrieval: format!("atx search repository {}", query_term),
        });
    }

    if prs.is_empty() {
        gaps.push(ClassifiedKnowledgeGap {
            severity: "MEDIUM".to_string(),
            gap_type: "Historical Pull Requests".to_string(),
            impact: "Cannot reuse previous implementation.".to_string(),
            suggested_retrieval: format!("atx search \"{}\"", query_term),
        });
    }

    gaps.push(ClassifiedKnowledgeGap {
        severity: "LOW".to_string(),
        gap_type: "UI Mockups".to_string(),
        impact: "Only affects presentation layer.".to_string(),
        suggested_retrieval: format!("atx search design {}", query_term),
    });

    gaps
}

fn predict_implementation_risks(
    _primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    _adrs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    apis: &[LabeledArtifact],
    other_related: &[LabeledArtifact],
    _aspects: &HashSet<DomainAspect>,
) -> Vec<ImplementationRisk> {
    let mut risks = Vec::new();

    let related_tickets_count = other_related.iter().filter(|a| matches!(a.artifact.kind, ArtifactKind::Ticket | ArtifactKind::Issue)).count();
    if related_tickets_count > 1 && !repos.is_empty() {
        risks.push(ImplementationRisk {
            level: "Potential Risk".to_string(),
            area: "Shared Domain Subsystem".to_string(),
            description: format!("Shared subsystem in repository '{}' referenced by multiple tickets.", repos[0]),
            evidence: "Repository metadata indicates multiple campaign tickets referencing the same subsystem.".to_string(),
        });
    }

    if !apis.is_empty() && prs.is_empty() {
        risks.push(ImplementationRisk {
            level: "Potential Risk".to_string(),
            area: "API Contract Integration".to_string(),
            description: "API contract is defined but no corresponding implementation PRs are linked.".to_string(),
            evidence: "API specification exists without corresponding pull request linkages.".to_string(),
        });
    }

    risks
}

fn generate_downstream_ai_guidance() -> Vec<String> {
    vec![
        "Treat Known Facts as authoritative.".to_string(),
        "Prioritize High Confidence evidence.".to_string(),
        "Retrieve missing blocking artifacts before implementation.".to_string(),
        "Do not assume undocumented architecture.".to_string(),
        "Generate implementation only after inspecting referenced specifications.".to_string(),
    ]
}

fn prioritize_knowledge_gaps(
    primary: &Option<KnowledgeArtifact>,
    repos: &[String],
    adrs: &[LabeledArtifact],
    docs: &[LabeledArtifact],
    prs: &[LabeledArtifact],
    aspects: &HashSet<DomainAspect>,
) -> PrioritizedKnowledgeGaps {
    let mut critical = Vec::new();
    let mut recommended = Vec::new();
    let mut optional = Vec::new();

    if primary.is_none() {
        critical.push("Primary artifact is un-indexed in local Atlas database.".to_string());
    }

    if aspects.contains(&DomainAspect::CodeImplementation) || aspects.contains(&DomainAspect::TaskTracking) {
        if repos.is_empty() {
            critical.push("Target repository is not explicitly identified.".to_string());
        }
        if adrs.is_empty() {
            recommended.push("Architecture Decision Records (ADRs) are missing or unlinked.".to_string());
        }
        if prs.is_empty() {
            recommended.push("Previous implementation pull requests & commits are missing.".to_string());
        }
    }

    if docs.is_empty() {
        recommended.push("Technical specification document is not indexed.".to_string());
    }

    optional.push("Figma UI design assets or mockups.".to_string());
    optional.push("Historical commit ancestry graph.".to_string());

    PrioritizedKnowledgeGaps {
        critical,
        recommended,
        optional,
    }
}



