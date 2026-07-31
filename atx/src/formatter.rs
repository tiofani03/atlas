use atlas_core::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact, Storage};

#[derive(Debug, Default, Clone)]
pub struct RelationshipCounts {
    pub tickets: usize,
    pub pull_requests: usize,
    pub commits: usize,
    pub documents: usize,
    pub releases: usize,
    pub other: usize,
}

impl RelationshipCounts {
    pub fn from_artifact_and_related(
        art: &KnowledgeArtifact,
        related: &[(ArtifactRelationship, KnowledgeArtifact)],
    ) -> Self {
        let mut counts = Self::default();
        let mut counted_ids = std::collections::HashSet::new();

        for (_rel, rel_art) in related {
            if counted_ids.insert(rel_art.id.clone()) {
                match rel_art.kind {
                    ArtifactKind::Ticket | ArtifactKind::Issue => counts.tickets += 1,
                    ArtifactKind::PullRequest
                    | ArtifactKind::PullRequestReview
                    | ArtifactKind::ReviewComment => counts.pull_requests += 1,
                    ArtifactKind::Commit => counts.commits += 1,
                    ArtifactKind::Document
                    | ArtifactKind::Specification
                    | ArtifactKind::Design => counts.documents += 1,
                    ArtifactKind::Release => counts.releases += 1,
                    _ => counts.other += 1,
                }
            }
        }

        // Check art.relationships for target IDs not present in DB
        for rel in &art.relationships {
            let target = &rel.target_id;
            if counted_ids.insert(target.clone()) {
                let kind_guess = classify_id(target);
                match kind_guess {
                    ArtifactKind::Ticket | ArtifactKind::Issue => counts.tickets += 1,
                    ArtifactKind::PullRequest
                    | ArtifactKind::PullRequestReview
                    | ArtifactKind::ReviewComment => counts.pull_requests += 1,
                    ArtifactKind::Commit => counts.commits += 1,
                    ArtifactKind::Document
                    | ArtifactKind::Specification
                    | ArtifactKind::Design => counts.documents += 1,
                    ArtifactKind::Release => counts.releases += 1,
                    _ => counts.other += 1,
                }
            }
        }

        counts
    }
}

pub fn primary_id(art: &KnowledgeArtifact) -> String {
    if !art.source_id.is_empty() {
        art.source_id.clone()
    } else {
        art.id.clone()
    }
}

pub fn format_kind(kind: &ArtifactKind) -> String {
    match kind {
        ArtifactKind::Repository => "Repository".to_string(),
        ArtifactKind::Issue => "Issue".to_string(),
        ArtifactKind::PullRequest => "Pull Request".to_string(),
        ArtifactKind::PullRequestReview => "Pull Request Review".to_string(),
        ArtifactKind::ReviewComment => "Review Comment".to_string(),
        ArtifactKind::Commit => "Commit".to_string(),
        ArtifactKind::Release => "Release".to_string(),
        ArtifactKind::Discussion => "Discussion".to_string(),
        ArtifactKind::WorkflowRun => "Workflow Run".to_string(),
        ArtifactKind::Deployment => "Deployment".to_string(),
        ArtifactKind::Ticket => "Ticket".to_string(),
        ArtifactKind::Document => "Document".to_string(),
        ArtifactKind::Specification => "Specification".to_string(),
        ArtifactKind::Design => "Design Document".to_string(),
        ArtifactKind::Component => "Component".to_string(),
        ArtifactKind::Other(s) => {
            let mut c = s.chars();
            match c.next() {
                None => "Other".to_string(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    }
}

pub fn format_provider(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "jira" => "Jira".to_string(),
        "github" => "GitHub".to_string(),
        "confluence" => "Confluence".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    }
}

pub fn extract_status(art: &KnowledgeArtifact) -> String {
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
    if let Some(ref sum) = art.summary {
        if sum.starts_with("Status: ") {
            let s = sum["Status: ".len()..].trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    "Done".to_string()
}

pub fn classify_id(target: &str) -> ArtifactKind {
    let lower = target.to_lowercase();
    if lower.contains('#') || lower.contains("pr") {
        ArtifactKind::PullRequest
    } else if target.len() >= 7 && target.chars().all(|c| c.is_ascii_hexdigit()) {
        ArtifactKind::Commit
    } else if lower.starts_with("adr-") || lower.contains("doc") || lower.contains("spec") {
        ArtifactKind::Document
    } else if lower.starts_with('v') && lower.contains('.') {
        ArtifactKind::Release
    } else {
        ArtifactKind::Ticket
    }
}

pub fn format_related_item(art: &KnowledgeArtifact) -> String {
    let pid = primary_id(art);
    match art.kind {
        ArtifactKind::PullRequest => {
            if pid.contains('#') {
                if let Some(pos) = pid.rfind('#') {
                    return pid[pos..].to_string();
                }
            } else if !pid.starts_with('#') && pid.chars().all(|c| c.is_ascii_digit()) {
                return format!("#{}", pid);
            }
            pid
        }
        ArtifactKind::Commit => {
            if pid.len() >= 7 {
                pid[..7].to_string()
            } else {
                pid
            }
        }
        ArtifactKind::Document | ArtifactKind::Specification | ArtifactKind::Design => {
            if pid.to_lowercase().starts_with("adr-") || pid.to_lowercase().starts_with("doc-") {
                pid
            } else if !art.title.is_empty() {
                art.title.clone()
            } else {
                pid
            }
        }
        _ => pid,
    }
}

pub fn format_target_id(target: &str, kind: &ArtifactKind) -> String {
    match kind {
        ArtifactKind::PullRequest => {
            if target.contains('#') {
                if let Some(pos) = target.rfind('#') {
                    return target[pos..].to_string();
                }
            } else if !target.starts_with('#') && target.chars().all(|c| c.is_ascii_digit()) {
                return format!("#{}", target);
            }
            target.to_string()
        }
        ArtifactKind::Commit => {
            if target.len() >= 7 {
                target[..7].to_string()
            } else {
                target.to_string()
            }
        }
        _ => target.to_string(),
    }
}

pub fn format_dot_aligned(label: &str, count: usize, width: usize) -> String {
    let count_str = count.to_string();
    let needed_dots = width.saturating_sub(label.len() + count_str.len() + 2);
    let dots = ".".repeat(needed_dots.max(3));
    format!("{} {} {}", label, dots, count_str)
}

pub fn format_search_results(
    artifacts: &[KnowledgeArtifact],
    storage: Option<&Storage>,
    _verbose: bool,
    raw: bool,
) -> String {
    if artifacts.is_empty() {
        return "No matching engineering artifacts found.".to_string();
    }

    if raw {
        let mut out = String::new();
        out.push_str(&format!("Found {} results:\n\n", artifacts.len()));
        for (i, art) in artifacts.iter().enumerate() {
            out.push_str(&format!("{}. {:?}\n", i + 1, art));
        }
        return out;
    }

    let mut out = String::new();
    let count_str = if artifacts.len() == 1 {
        "artifact"
    } else {
        "artifacts"
    };
    out.push_str(&format!("Found {} {}\n\n", artifacts.len(), count_str));

    for (i, art) in artifacts.iter().enumerate() {
        let divider = "──────────────────────────────────────────────";
        out.push_str(divider);
        out.push('\n');

        let kind_str = format_kind(&art.kind).to_uppercase();
        let display_id = primary_id(art);
        out.push_str(&format!("[{}] {} • {}\n\n", kind_str, display_id, art.title));

        let status = extract_status(art);
        let repo = art.repository.as_deref().unwrap_or("N/A");
        let provider = format_provider(&art.provider);

        out.push_str(&format!("{:<12}{}\n", "Status", status));
        out.push_str(&format!("{:<12}{}\n", "Repository", repo));
        out.push_str(&format!("{:<12}{}\n\n", "Provider", provider));

        if let Some(ref sum) = art.summary {
            let clean_sum = if sum.starts_with("Status: ") {
                sum["Status: ".len()..].trim()
            } else {
                sum.as_str()
            };
            if !clean_sum.is_empty() {
                out.push_str("Summary\n");
                out.push_str(clean_sum);
                out.push_str("\n\n");
            }
        }

        if !art.tags.is_empty() {
            out.push_str("Tags\n");
            for tag in &art.tags {
                out.push_str(tag);
                out.push('\n');
            }
            out.push('\n');
        }

        let related_list = storage
            .and_then(|s| s.get_related_artifacts(&art.id).ok())
            .unwrap_or_default();
        let counts = RelationshipCounts::from_artifact_and_related(art, &related_list);

        out.push_str("Relationships\n");
        out.push_str(&format!("{} related tickets\n", counts.tickets));
        out.push_str(&format!("{} pull requests\n", counts.pull_requests));
        out.push_str(&format!("{} commits\n", counts.commits));
        out.push_str(&format!("{} documents\n\n", counts.documents));

        out.push_str("Hint\n");
        out.push_str(&format!("atx artifact {}\n", display_id));
        out.push_str(&format!("atx related {}\n", display_id));
        out.push_str(&format!("atx context {}\n", display_id));

        out.push_str(divider);
        if i + 1 < artifacts.len() {
            out.push_str("\n\n");
        } else {
            out.push('\n');
        }
    }

    out
}

pub fn format_related_results(
    id_or_key: &str,
    key_artifact: Option<&KnowledgeArtifact>,
    related: &[(ArtifactRelationship, KnowledgeArtifact)],
    verbose: bool,
    raw: bool,
) -> String {
    if raw {
        let mut out = String::new();
        out.push_str(&format!("Relationships for {} (RAW):\n\n", id_or_key));
        for (rel, art) in related {
            out.push_str(&format!("Relationship: {:?}, Artifact: {:?}\n", rel, art));
        }
        return out;
    }

    let display_key = key_artifact
        .map(primary_id)
        .unwrap_or_else(|| id_or_key.to_string());
    let mut out = String::new();
    out.push_str(&format!("Relationships for {}\n\n", display_key));

    let mut tickets = Vec::new();
    let mut prs = Vec::new();
    let mut commits = Vec::new();
    let mut docs = Vec::new();
    let mut releases = Vec::new();
    let mut others = Vec::new();

    let mut seen = std::collections::HashSet::new();

    for (_rel, art) in related {
        if seen.insert(art.id.clone()) {
            let item_str = format_related_item(art);
            match art.kind {
                ArtifactKind::Ticket | ArtifactKind::Issue => tickets.push(item_str),
                ArtifactKind::PullRequest
                | ArtifactKind::PullRequestReview
                | ArtifactKind::ReviewComment => prs.push(item_str),
                ArtifactKind::Commit => commits.push(item_str),
                ArtifactKind::Document
                | ArtifactKind::Specification
                | ArtifactKind::Design => docs.push(item_str),
                ArtifactKind::Release => releases.push(item_str),
                _ => others.push(item_str),
            }
        }
    }

    if let Some(key_art) = key_artifact {
        for rel in &key_art.relationships {
            let target = &rel.target_id;
            if seen.insert(target.clone()) {
                let kind_guess = classify_id(target);
                let item_str = format_target_id(target, &kind_guess);
                match kind_guess {
                    ArtifactKind::Ticket | ArtifactKind::Issue => tickets.push(item_str),
                    ArtifactKind::PullRequest
                    | ArtifactKind::PullRequestReview
                    | ArtifactKind::ReviewComment => prs.push(item_str),
                    ArtifactKind::Commit => commits.push(item_str),
                    ArtifactKind::Document
                    | ArtifactKind::Specification
                    | ArtifactKind::Design => docs.push(item_str),
                    ArtifactKind::Release => releases.push(item_str),
                    _ => others.push(item_str),
                }
            }
        }
    }

    tickets.sort();
    prs.sort();
    commits.sort();
    docs.sort();
    releases.sort();
    others.sort();

    let categories = [
        ("Tickets", tickets),
        ("Pull Requests", prs),
        ("Commits", commits),
        ("Documents", docs),
        ("Releases", releases),
        ("Other", others),
    ];

    let mut printed_category = false;
    for (cat_name, items) in categories {
        if items.is_empty() {
            continue;
        }
        if printed_category {
            out.push('\n');
        }
        printed_category = true;

        let total_count = items.len();
        out.push_str(&format!("{} ({})\n\n", cat_name, total_count));

        let limit = if verbose { total_count } else { 3 };
        for item in items.iter().take(limit) {
            out.push_str(&format!("• {}\n", item));
        }
        if !verbose && total_count > limit {
            out.push_str("• ...\n");
        }
    }

    if !printed_category {
        out.push_str("No relationships found.\n");
    }

    out.trim_end().to_string()
}

pub fn format_artifact_detail(
    art: &KnowledgeArtifact,
    storage: Option<&Storage>,
    verbose: bool,
    raw: bool,
) -> String {
    if raw {
        return format!("{:?}", art);
    }

    let divider = "──────────────────────────────────────────────";
    let mut out = String::new();

    out.push_str(divider);
    out.push_str("\n\n");
    out.push_str(&art.title);
    out.push_str("\n\n");
    let display_id = primary_id(art);
    out.push_str(&display_id);
    out.push('\n');
    out.push_str(&format_kind(&art.kind));
    out.push_str("\n\n");
    out.push_str(divider);
    out.push_str("\n\n");

    let status = extract_status(art);
    let repo = art.repository.as_deref().unwrap_or("N/A");
    let provider = format_provider(&art.provider);

    out.push_str("Status\n");
    out.push_str(&status);
    out.push_str("\n\n");

    out.push_str("Repository\n");
    out.push_str(repo);
    out.push_str("\n\n");

    out.push_str("Provider\n");
    out.push_str(&provider);
    out.push_str("\n\n");

    if let Some(ref sum) = art.summary {
        let clean_sum = if sum.starts_with("Status: ") {
            sum["Status: ".len()..].trim()
        } else {
            sum.as_str()
        };
        if !clean_sum.is_empty() {
            out.push_str("Summary\n\n");
            out.push_str(clean_sum);
            out.push_str("\n\n");
        }
    }

    if !art.body.is_empty() {
        out.push_str("Description\n\n");
        if !verbose && art.body.len() > 1000 {
            out.push_str(&art.body[..1000]);
            out.push_str("...\n\n");
        } else {
            out.push_str(&art.body);
            out.push_str("\n\n");
        }
    }

    if !art.tags.is_empty() {
        out.push_str("Tags\n\n");
        for tag in &art.tags {
            out.push_str(tag);
            out.push('\n');
        }
        out.push('\n');
    }

    let related_list = storage
        .and_then(|s| s.get_related_artifacts(&art.id).ok())
        .unwrap_or_default();
    let counts = RelationshipCounts::from_artifact_and_related(art, &related_list);

    out.push_str("Relationships\n\n");
    out.push_str(&format_dot_aligned("Tickets", counts.tickets, 24));
    out.push('\n');
    out.push_str(&format_dot_aligned("Pull Requests", counts.pull_requests, 24));
    out.push('\n');
    out.push_str(&format_dot_aligned("Commits", counts.commits, 24));
    out.push('\n');
    out.push_str(&format_dot_aligned("Documents", counts.documents, 24));
    out.push_str("\n\n");

    out.push_str("Source\n\n");
    out.push_str(&art.source_url);
    out.push_str("\n\n");

    out.push_str(divider);
    out.push_str("\n\n");

    out.push_str("Next Commands\n\n");
    out.push_str(&format!("atx related {}\n", display_id));
    out.push_str(&format!("atx context {}\n", display_id));

    out.trim_end().to_string()
}

pub fn format_context_package(
    pkg: &atlas_core::ContextPackage,
    _verbose: bool,
    raw: bool,
) -> String {
    if raw {
        return format!("{:?}", pkg);
    }

    let mut out = String::new();

    // Header
    out.push_str(&format!("Context for {}\n", pkg.target_id));
    out.push_str(&"=".repeat(12 + pkg.target_id.len()));
    out.push_str("\n\n");

    // 1. Title
    out.push_str("Title\n-----\n");
    out.push_str(&pkg.title);
    out.push_str("\n\n");

    // 2. Status
    out.push_str("Status\n------\n");
    out.push_str(&pkg.status);
    out.push_str("\n\n");

    // 3. Repository
    out.push_str("Repository\n----------\n");
    out.push_str(pkg.repository.as_deref().unwrap_or("N/A"));
    out.push_str("\n\n");

    // Description (if present)
    if let Some(ref desc) = pkg.description {
        out.push_str("Description\n-----------\n");
        let clean_desc = if desc.len() > 1000 && !_verbose {
            format!("{}...", &desc[..1000])
        } else {
            desc.to_string()
        };
        out.push_str(&clean_desc);
        out.push_str("\n\n");
    }

    // 4. Engineering Readiness
    out.push_str("Engineering Readiness\n---------------------\n\n");
    out.push_str(&format!("{}\n\n", pkg.engineering_readiness.status_label));
    out.push_str(&format!("{}\n\n", pkg.engineering_readiness.readiness_summary));

    out.push_str("Available\n\n");
    if !pkg.engineering_readiness.available.is_empty() {
        for item in &pkg.engineering_readiness.available {
            out.push_str(&format!("✓ {}\n", item));
        }
    } else {
        out.push_str("(None)\n");
    }

    if !pkg.engineering_readiness.missing.is_empty() {
        out.push_str("\nMissing\n\n");
        for item in &pkg.engineering_readiness.missing {
            out.push_str(&format!("✗ {}\n", item));
        }
    }
    out.push_str("\n");

    // 5. Context Completeness
    out.push_str("Context Completeness\n--------------------\n\n");
    out.push_str(&format!("{} {}%\n\n", pkg.completeness.progress_bar, pkg.completeness.score_percentage));

    out.push_str("Scoring\n\n");
    let all_categories = [
        ("Repository", !pkg.affected_repositories.is_empty()),
        ("Related APIs", !pkg.apis.is_empty()),
        ("Related Issues", !pkg.related_artifacts.is_empty()),
        ("Architecture Decision", !pkg.architecture_decisions.is_empty()),
        ("Documentation", !pkg.related_documentation.is_empty()),
        ("Previous PRs", !pkg.related_pull_requests.is_empty()),
        ("Commit History", !pkg.related_commits.is_empty()),
    ];

    for (label, is_present) in all_categories {
        let mark = if is_present { "✓" } else { "✗" };
        let dots = format_dots(label, mark, 28);
        out.push_str(&dots);
        out.push('\n');
    }
    out.push('\n');

    // 6. Recommended Reading
    if !pkg.recommended_reading.is_empty() {
        out.push_str("Recommended Reading\n-------------------\n\n");
        for (idx, item) in pkg.recommended_reading.iter().enumerate() {
            out.push_str(&format!("{}. {}\n   {}\n\n", idx + 1, item.source_id, item.title));
        }
    }

    // 7. Implementation Hints
    if !pkg.implementation_hints.is_empty() {
        out.push_str("Implementation Hints\n--------------------\n\n");
        for hint in &pkg.implementation_hints {
            out.push_str(&format!("• {}\n\n", hint));
        }
    }

    // 8. Suggested Next Actions
    if !pkg.suggested_next_actions.is_empty() {
        out.push_str("Suggested Next Actions\n----------------------\n\n");
        for action in &pkg.suggested_next_actions {
            out.push_str(&format!("{}. {}\n   {}\n", action.step, action.action, action.detail));
            if let Some(ref cmd) = action.command {
                out.push_str(&format!("   {}\n", cmd));
            }
            out.push('\n');
        }
    }

    // 9. Related Artifacts
    let categories_map = group_related_artifacts(pkg);
    if !categories_map.is_empty() {
        out.push_str("Related Artifacts\n-----------------\n\n");
        for (cat_name, items) in categories_map {
            out.push_str(&format!("{}\n", cat_name));
            for (id_display, label) in items {
                out.push_str(&format!("  • {} ({})\n", id_display, label));
            }
            out.push('\n');
        }
    }

    // 10. Source Information
    out.push_str("Source Information\n------------------\n\n");
    out.push_str(&format!("{:<14}: {}\n", "Provider", pkg.source_info.provider));
    out.push_str(&format!("{:<14}: {}\n", "Repository", pkg.source_info.repository.as_deref().unwrap_or("N/A")));
    out.push_str(&format!("{:<14}: {}\n", "Updated", pkg.source_info.updated_at));
    out.push_str(&format!("{:<14}: {}\n", "Source URL", pkg.source_info.source_url));
    out.push_str(&format!("{:<14}: {}\n\n", "Last Synced", pkg.source_info.synced_at));

    // Summary (Engineering Assessment)
    out.push_str("Engineering Assessment\n----------------------\n\n");
    out.push_str(&pkg.summary);
    out.push('\n');

    out.trim_end().to_string()
}

fn format_dots(label: &str, value: &str, total_width: usize) -> String {
    let label_chars = label.chars().count();
    let val_chars = value.chars().count();
    let needed = total_width.saturating_sub(label_chars + val_chars + 2);
    let dots = ".".repeat(needed.max(3));
    format!("{} {} {}", label, dots, value)
}

fn group_related_artifacts(pkg: &atlas_core::ContextPackage) -> Vec<(String, Vec<(String, String)>)> {
    let mut groups: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();

    let all_collections = [
        &pkg.architecture_decisions,
        &pkg.apis,
        &pkg.related_pull_requests,
        &pkg.related_commits,
        &pkg.related_documentation,
        &pkg.implementation_history,
        &pkg.related_artifacts,
    ];

    for col in all_collections {
        for item in col {
            let cat_name = item.relationship_category.clone();
            let id_display = format_related_item(&item.artifact);
            let entry = groups.entry(cat_name).or_default();
            if !entry.iter().any(|(id, _)| id == &id_display) {
                entry.push((id_display, item.relationship_label.clone()));
            }
        }
    }

    if !pkg.affected_repositories.is_empty() {
        let repo_items: Vec<(String, String)> = pkg.affected_repositories.iter().map(|r| (r.clone(), "target repo".to_string())).collect();
        groups.insert("Repositories".to_string(), repo_items);
    }

    let mut result: Vec<(String, Vec<(String, String)>)> = groups.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mock_artifact() -> KnowledgeArtifact {
        let mut rels = Vec::new();
        for i in 1..=37 {
            rels.push(ArtifactRelationship {
                source_id: "INIT-219".to_string(),
                target_id: format!("DEV-{}", 1100 + i),
                relationship_type: "relates_to".to_string(),
            });
        }
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "owner/repo#212".to_string(),
            relationship_type: "implemented_by".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "owner/repo#214".to_string(),
            relationship_type: "implemented_by".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "owner/repo#230".to_string(),
            relationship_type: "implemented_by".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "owner/repo#215".to_string(),
            relationship_type: "implemented_by".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "a31ef2d81bf992112233".to_string(),
            relationship_type: "commit".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "81bf9921122334455667".to_string(),
            relationship_type: "commit".to_string(),
        });
        rels.push(ArtifactRelationship {
            source_id: "INIT-219".to_string(),
            target_id: "ADR-007".to_string(),
            relationship_type: "documented_by".to_string(),
        });

        KnowledgeArtifact {
            id: "init-219-hash".to_string(),
            kind: ArtifactKind::Ticket,
            title: "MongoDB Atlas".to_string(),
            summary: Some("Migrate MongoDB infrastructure to Atlas...".to_string()),
            body: "Full details of MongoDB migration...".to_string(),
            provider: "jira".to_string(),
            source_id: "INIT-219".to_string(),
            source_url: "https://jira.example.com/browse/INIT-219".to_string(),
            repository: Some("INIT".to_string()),
            tags: vec!["project:INIT".to_string()],
            relationships: rels,
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: "checksum".to_string(),
            metadata: serde_json::json!({ "status": "Done" }),
        }
    }

    #[test]
    fn test_format_search_results() {
        let art = mock_artifact();
        let formatted = format_search_results(&[art], None, false, false);

        assert!(formatted.contains("Found 1 artifact"));
        assert!(formatted.contains("[TICKET] INIT-219 • MongoDB Atlas"));
        assert!(formatted.contains("Status      Done"));
        assert!(formatted.contains("Repository  INIT"));
        assert!(formatted.contains("Provider    Jira"));
        assert!(formatted.contains("Summary\nMigrate MongoDB infrastructure to Atlas..."));
        assert!(formatted.contains("37 related tickets"));
        assert!(formatted.contains("4 pull requests"));
        assert!(formatted.contains("2 commits"));
        assert!(formatted.contains("1 documents"));
        assert!(formatted.contains("atx artifact INIT-219"));
        assert!(formatted.contains("atx related INIT-219"));
        assert!(formatted.contains("atx context INIT-219"));
    }

    #[test]
    fn test_format_related_results_truncation() {
        let art = mock_artifact();
        let formatted = format_related_results("INIT-219", Some(&art), &[], false, false);

        assert!(formatted.contains("Relationships for INIT-219"));
        assert!(formatted.contains("Tickets (37)"));
        assert!(formatted.contains("• DEV-1101"));
        assert!(formatted.contains("• DEV-1102"));
        assert!(formatted.contains("• DEV-1103"));
        assert!(formatted.contains("• ..."));
        assert!(formatted.contains("Pull Requests (4)"));
        assert!(formatted.contains("• #212"));
        assert!(formatted.contains("• #214"));
        assert!(formatted.contains("• #215"));
        assert!(formatted.contains("• ..."));
        assert!(formatted.contains("Commits (2)"));
        assert!(formatted.contains("• 81bf992"));
        assert!(formatted.contains("• a31ef2d"));
        assert!(formatted.contains("Documents (1)"));
        assert!(formatted.contains("• ADR-007"));
    }

    #[test]
    fn test_format_related_results_verbose() {
        let art = mock_artifact();
        let formatted = format_related_results("INIT-219", Some(&art), &[], true, false);

        assert!(formatted.contains("Tickets (37)"));
        assert!(!formatted.contains("• ..."));
        assert!(formatted.contains("• DEV-1137"));
    }

    #[test]
    fn test_format_artifact_detail() {
        let art = mock_artifact();
        let formatted = format_artifact_detail(&art, None, false, false);

        assert!(formatted.contains("MongoDB Atlas"));
        assert!(formatted.contains("INIT-219"));
        assert!(formatted.contains("Ticket"));
        assert!(formatted.contains("Status\nDone"));
        assert!(formatted.contains("Repository\nINIT"));
        assert!(formatted.contains("Provider\nJira"));
        assert!(formatted.contains("Summary\n\nMigrate MongoDB infrastructure to Atlas..."));
        assert!(formatted.contains("Description\n\nFull details of MongoDB migration..."));
        assert!(formatted.contains("Tags\n\nproject:INIT"));
        assert!(formatted.contains("Tickets ............. 37"));
        assert!(formatted.contains("Pull Requests ........ 4"));
        assert!(formatted.contains("Next Commands\n\natx related INIT-219\natx context INIT-219"));
    }

    #[test]
    fn test_format_context_package() {
        let art = mock_artifact();
        let pkg = atlas_core::ContextPackage {
            target_kind: "issue".to_string(),
            target_id: "INIT-219".to_string(),
            primary_artifact: Some(art),
            title: "MongoDB Atlas Migration".to_string(),
            status: "Done".to_string(),
            repository: Some("INIT".to_string()),
            description: Some("Migrate MongoDB infrastructure to Atlas...".to_string()),
            engineering_readiness: atlas_core::EngineeringReadiness {
                status_label: "Ready for implementation.".to_string(),
                readiness_summary: "Atlas found sufficient context.".to_string(),
                available: vec!["Repository".to_string()],
                missing: vec!["Pull Requests".to_string()],
            },
            completeness: atlas_core::CompletenessReport {
                score_percentage: 62,
                progress_bar: "██████░░░░".to_string(),
                available_categories: vec![atlas_core::CategoryAvailability {
                    category: "Repository".to_string(),
                    is_available: true,
                    count: 1,
                    label: "1 Repository".to_string(),
                }],
                missing_categories: vec![atlas_core::CategoryAvailability {
                    category: "Previous PRs".to_string(),
                    is_available: false,
                    count: 0,
                    label: "Previous Pull Requests".to_string(),
                }],
            },
            recommended_reading: vec![atlas_core::RecommendedItem {
                id: "adr-007".to_string(),
                source_id: "ADR-007".to_string(),
                title: "MongoDB Architecture".to_string(),
                kind: "document".to_string(),
                relationship_label: "defines architecture".to_string(),
                score: 85.0,
                reason: "Architecture Decision (ADR) guideline".to_string(),
            }],
            implementation_hints: vec!["Review ADR-007 before implementation.".to_string()],
            suggested_next_actions: vec![atlas_core::NextAction {
                step: 1,
                action: "Review ADR-007".to_string(),
                detail: "Architecture guideline".to_string(),
                command: None,
            }],
            affected_repositories: vec!["INIT".to_string()],
            related_artifacts: vec![],
            dependency_graph: vec![],
            implementation_history: vec![],
            related_pull_requests: vec![],
            related_commits: vec![],
            related_documentation: vec![],
            apis: vec![],
            architecture_decisions: vec![atlas_core::LabeledArtifact {
                artifact: KnowledgeArtifact {
                    id: "adr-7".to_string(),
                    kind: ArtifactKind::Document,
                    title: "MongoDB Architecture".to_string(),
                    summary: None,
                    body: "".to_string(),
                    provider: "confluence".to_string(),
                    source_id: "ADR-007".to_string(),
                    source_url: "".to_string(),
                    repository: None,
                    tags: vec!["adr".to_string()],
                    relationships: vec![],
                    created_at: None,
                    updated_at: Utc::now(),
                    synced_at: Utc::now(),
                    checksum: "".to_string(),
                    metadata: serde_json::Value::Null,
                },
                relationship_label: "defines architecture".to_string(),
                relationship_category: "Architecture / Docs".to_string(),
                score: 85.0,
                is_direct_graph: true,
            }],
            source_info: atlas_core::SourceInfo {
                provider: "Jira".to_string(),
                repository: Some("INIT".to_string()),
                source_url: "https://jira.example.com/browse/INIT-219".to_string(),
                updated_at: "2026-07-30".to_string(),
                synced_at: "2026-07-31 15:45 UTC".to_string(),
            },
            summary: "Implementation can begin. Business requirements are available.".to_string(),
        };

        let formatted = format_context_package(&pkg, false, false);

        assert!(formatted.contains("Context for INIT-219"));
        assert!(formatted.contains("Title\n-----\nMongoDB Atlas Migration"));
        assert!(formatted.contains("Status\n------\nDone"));
        assert!(formatted.contains("Repository\n----------\nINIT"));
        assert!(formatted.contains("Engineering Readiness"));
        assert!(formatted.contains("Ready for implementation."));
        assert!(formatted.contains("✓ Repository"));
        assert!(formatted.contains("✗ Pull Requests"));
        assert!(formatted.contains("Context Completeness"));
        assert!(formatted.contains("██████░░░░ 62%"));
        assert!(formatted.contains("Scoring"));
        assert!(formatted.contains("Repository"));
        assert!(formatted.contains("Recommended Reading"));
        assert!(formatted.contains("1. ADR-007"));
        assert!(formatted.contains("MongoDB Architecture"));
        assert!(formatted.contains("Implementation Hints"));
        assert!(formatted.contains("• Review ADR-007 before implementation."));
        assert!(formatted.contains("Suggested Next Actions"));
        assert!(formatted.contains("1. Review ADR-007"));
        assert!(formatted.contains("Related Artifacts"));
        assert!(formatted.contains("• ADR-007 (defines architecture)"));
        assert!(formatted.contains("Source Information"));
        assert!(formatted.contains("Provider"));
        assert!(formatted.contains("Jira"));
        assert!(formatted.contains("Engineering Assessment"));
        assert!(formatted.contains("Implementation can begin."));
    }
}
