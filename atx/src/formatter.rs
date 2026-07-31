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
    art: &KnowledgeArtifact,
    related: &[(ArtifactRelationship, KnowledgeArtifact)],
    verbose: bool,
    raw: bool,
) -> String {
    if raw {
        return format!("Context raw for artifact {:?}, related: {:?}", art, related);
    }

    let primary_key = primary_id(art);
    let mut out = String::new();

    out.push_str("Engineering Context\n\n");

    out.push_str("Artifact\n\n");
    out.push_str(&primary_key);
    out.push('\n');
    out.push_str(&art.title);
    out.push_str("\n\n");

    // Requirement
    let req_text = art
        .summary
        .as_deref()
        .map(|s| {
            if s.starts_with("Status: ") {
                s["Status: ".len()..].trim()
            } else {
                s
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if !art.body.is_empty() {
                art.body.lines().next().unwrap_or(&art.title)
            } else {
                &art.title
            }
        });
    out.push_str("Requirement\n\n");
    out.push_str(req_text);
    out.push_str("\n\n");

    let mut design_docs = Vec::new();
    let mut implementations = Vec::new();
    let mut commits = Vec::new();
    let mut apis = Vec::new();
    let mut releases = Vec::new();
    let mut tickets = Vec::new();

    let mut seen = std::collections::HashSet::new();

    for (_rel, rel_art) in related {
        if seen.insert(rel_art.id.clone()) {
            let display_str = format_related_item(rel_art);
            match rel_art.kind {
                ArtifactKind::Document
                | ArtifactKind::Specification
                | ArtifactKind::Design => design_docs.push((display_str, rel_art.title.clone())),
                ArtifactKind::PullRequest
                | ArtifactKind::PullRequestReview
                | ArtifactKind::ReviewComment => implementations.push(display_str),
                ArtifactKind::Commit => commits.push(display_str),
                ArtifactKind::Component => apis.push((display_str, rel_art.title.clone())),
                ArtifactKind::Release => releases.push(display_str),
                ArtifactKind::Ticket | ArtifactKind::Issue => tickets.push(display_str),
                _ => {
                    if rel_art.tags.iter().any(|t| t.contains("api") || t.contains("openapi")) {
                        apis.push((display_str, rel_art.title.clone()));
                    }
                }
            }
        }
    }

    for rel in &art.relationships {
        let target = &rel.target_id;
        if seen.insert(target.clone()) {
            let kind_guess = classify_id(target);
            let item_str = format_target_id(target, &kind_guess);
            match kind_guess {
                ArtifactKind::Document
                | ArtifactKind::Specification
                | ArtifactKind::Design => design_docs.push((item_str, String::new())),
                ArtifactKind::PullRequest
                | ArtifactKind::PullRequestReview
                | ArtifactKind::ReviewComment => implementations.push(item_str),
                ArtifactKind::Commit => commits.push(item_str),
                ArtifactKind::Release => releases.push(item_str),
                ArtifactKind::Ticket | ArtifactKind::Issue => tickets.push(item_str),
                _ => {}
            }
        }
    }

    // Design Documents
    if !design_docs.is_empty() {
        out.push_str("Design Documents\n\n");
        for (doc_id, doc_title) in &design_docs {
            out.push_str(doc_id);
            out.push('\n');
            if !doc_title.is_empty() && doc_title != doc_id {
                out.push_str(doc_title);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    // Implementation
    if !implementations.is_empty() {
        out.push_str("Implementation\n\n");
        for imp in &implementations {
            out.push_str(imp);
            out.push('\n');
        }
        out.push('\n');
    }

    // Commits
    if !commits.is_empty() {
        out.push_str("Commits\n\n");
        for c in &commits {
            out.push_str(c);
            out.push('\n');
        }
        out.push('\n');
    }

    // Related APIs
    if !apis.is_empty() {
        out.push_str("Related APIs\n\n");
        for (api_id, api_title) in &apis {
            out.push_str(api_id);
            out.push('\n');
            if !api_title.is_empty() && api_title != api_id {
                out.push_str(api_title);
                out.push('\n');
            }
        }
        out.push('\n');
    }

    // Related Releases
    if !releases.is_empty() {
        out.push_str("Related Releases\n\n");
        for rel in &releases {
            out.push_str(rel);
            out.push('\n');
        }
        out.push('\n');
    }

    // Related Tickets
    if !tickets.is_empty() {
        out.push_str("Related Tickets\n\n");
        let limit = if verbose { tickets.len() } else { 3 };
        for t in tickets.iter().take(limit) {
            out.push_str(t);
            out.push('\n');
        }
        if !verbose && tickets.len() > limit {
            out.push_str("...\n");
        }
        out.push('\n');
    }

    // Summary
    out.push_str("Summary\n\n");
    let summary_narrative = build_context_narrative(art, &releases, &implementations);
    out.push_str(&summary_narrative);
    out.push('\n');

    out.trim_end().to_string()
}

fn build_context_narrative(
    art: &KnowledgeArtifact,
    releases: &[String],
    implementations: &[String],
) -> String {
    let clean_summary = art
        .summary
        .as_deref()
        .map(|s| {
            if s.starts_with("Status: ") {
                s["Status: ".len()..].trim()
            } else {
                s
            }
        })
        .unwrap_or("");

    let base_text = if !clean_summary.is_empty() && clean_summary.len() > 5 {
        let mut chars = clean_summary.chars();
        let first = chars.next().unwrap().to_lowercase().to_string();
        format!("This feature {}{}", first, chars.as_str())
    } else {
        format!("This feature relates to {}", art.title)
    };

    if let Some(rel) = releases.first() {
        format!("{} and was released in {}.", base_text.trim_end_matches('.'), rel)
    } else if let Some(pr) = implementations.first() {
        format!("{} via {}.", base_text.trim_end_matches('.'), pr)
    } else {
        format!("{}.", base_text.trim_end_matches('.'))
    }
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
        let formatted = format_context_package(&art, &[], false, false);

        assert!(formatted.contains("Engineering Context"));
        assert!(formatted.contains("Artifact\n\nINIT-219\nMongoDB Atlas"));
        assert!(formatted.contains("Requirement\n\nMigrate MongoDB infrastructure to Atlas..."));
        assert!(formatted.contains("Design Documents\n\nADR-007"));
        assert!(formatted.contains("Implementation\n\n#212"));
        assert!(formatted.contains("Commits\n\na31ef2d"));
        assert!(formatted.contains("Related Tickets\n\nDEV-1101\nDEV-1102\nDEV-1103\n..."));
        assert!(formatted.contains("Summary\n\nThis feature migrate MongoDB infrastructure to Atlas via #212."));
    }
}
