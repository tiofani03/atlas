use atlas_core::{ArtifactKind, ArtifactRelationship, DomainAspect, KnowledgeArtifact, Storage};

pub fn safe_truncate(text: &str, max_chars: usize) -> &str {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}

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
            let canon_id = if !rel_art.source_id.is_empty() {
                rel_art.source_id.clone()
            } else {
                rel_art.id.clone()
            };
            if counted_ids.insert(canon_id) {
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

        for rel in &art.relationships {
            let target = &rel.target_id;
            let canon_target = if target.contains(':') && !target.contains('#') && !target.contains('@') {
                target.split(':').last().unwrap_or(target).to_string()
            } else {
                target.clone()
            };

            if counted_ids.insert(canon_target) {
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
        ArtifactKind::Repository => {
            if !art.title.is_empty() && !art.title.starts_with("repo_") {
                art.title.clone()
            } else if let Some(ref repo) = art.repository {
                repo.clone()
            } else {
                pid
            }
        }
        ArtifactKind::PullRequest | ArtifactKind::PullRequestReview | ArtifactKind::ReviewComment => {
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

            let clean_num = if pr_num.contains('/') {
                if let Some(pos) = pr_num.rfind('/') {
                    pr_num[pos + 1..].to_string()
                } else {
                    pr_num
                }
            } else {
                pr_num
            };

            if !art.title.is_empty() && art.title != clean_num {
                format!("{} • {}", clean_num, art.title)
            } else {
                clean_num
            }
        }
        ArtifactKind::Commit => {
            let sha = if let Some(pos) = pid.rfind('@') {
                &pid[pos + 1..]
            } else if let Some(pos) = pid.rfind(':') {
                &pid[pos + 1..]
            } else {
                &pid
            };
            let short_sha = if sha.len() >= 8 {
                &sha[..8]
            } else {
                sha
            };

            let first_line = art.title.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() && first_line != sha {
                format!("{} • {}", short_sha, first_line)
            } else {
                short_sha.to_string()
            }
        }
        ArtifactKind::Ticket | ArtifactKind::Issue => {
            if !art.title.is_empty() && art.title != pid {
                format!("{} • {}", pid, art.title)
            } else {
                pid
            }
        }
        ArtifactKind::Release => {
            if !art.title.is_empty() {
                art.title.clone()
            } else if pid.contains('/') {
                if let Some(pos) = pid.rfind('/') {
                    pid[pos + 1..].to_string()
                } else {
                    pid
                }
            } else {
                pid
            }
        }
        ArtifactKind::Document | ArtifactKind::Specification | ArtifactKind::Design => {
            if pid.to_lowercase().starts_with("adr-") || pid.to_lowercase().starts_with("doc-") {
                if !art.title.is_empty() && art.title != pid {
                    format!("{} • {}", pid, art.title)
                } else {
                    pid
                }
            } else if !art.title.is_empty() {
                art.title.clone()
            } else {
                pid
            }
        }
        _ => {
            if !art.title.is_empty() && art.title != pid && !pid.starts_with("repo_") {
                format!("{} • {}", pid, art.title)
            } else {
                pid
            }
        }
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
        if !verbose && art.body.chars().count() > 1000 {
            out.push_str(safe_truncate(&art.body, 1000));
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
        .and_then(|s| s.get_related_artifacts(&art.source_id).ok())
        .or_else(|| storage.and_then(|s| s.get_related_artifacts(&art.id).ok()))
        .unwrap_or_default();
    let counts = RelationshipCounts::from_artifact_and_related(art, &related_list);

    out.push_str("Relationships\n\n");

    let mut implements_list = Vec::new();
    let mut part_of_prs = Vec::new();
    let mut released_in = Vec::new();
    let mut seen_items = std::collections::HashSet::new();

    for (rel, rel_art) in &related_list {
        let label = format_related_item(rel_art);
        if seen_items.insert(label.clone()) {
            if rel.relationship_type == "implements" || rel.relationship_type == "implemented_by" {
                if rel_art.kind == ArtifactKind::Ticket || rel_art.kind == ArtifactKind::Issue {
                    implements_list.push(label);
                }
            } else if rel.relationship_type == "merged_into" || rel.relationship_type == "contains" {
                if rel_art.kind == ArtifactKind::PullRequest {
                    part_of_prs.push(label);
                }
            } else if rel.relationship_type == "released_in" {
                released_in.push(label);
            }
        }
    }

    if !implements_list.is_empty() {
        out.push_str("Implements\n");
        for item in &implements_list {
            out.push_str(&format!("  • {}\n", item));
        }
        out.push('\n');
    }

    if !part_of_prs.is_empty() {
        out.push_str("Part of PR\n");
        for item in &part_of_prs {
            out.push_str(&format!("  • {}\n", item));
        }
        out.push('\n');
    }

    if art.kind == ArtifactKind::PullRequest && counts.commits > 0 {
        out.push_str(&format!("Commits ({})\n\n", counts.commits));
    }

    if !released_in.is_empty() {
        out.push_str("Released In\n");
        for item in &released_in {
            out.push_str(&format!("  • {}\n", item));
        }
        out.push('\n');
    }

    if art.kind == ArtifactKind::Commit {
        let file_count = storage
            .and_then(|s| s.get_commit_file_count(&art.source_id).ok())
            .unwrap_or(0);
        if file_count > 0 {
            out.push_str(&format!("Files Changed\n  {} files\n\n", file_count));
        }
    }

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

/// Trait representing an atomic, reusable context section
pub trait ContextSection {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn aspect(&self) -> Option<DomainAspect>;
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool;
    fn render(&self, pkg: &atlas_core::ContextPackage, verbose: bool) -> String;
}

pub struct KnownFactsSection;
impl ContextSection for KnownFactsSection {
    fn id(&self) -> &'static str {
        "known_facts"
    }
    fn title(&self) -> &'static str {
        "Known Facts"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.known_facts.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Known Facts\n-----------\n\n");
        for fact in &pkg.known_facts {
            let symbol = if fact.is_verified { "✓" } else { "✗" };
            out.push_str(&format!("{} {}\n", symbol, fact.statement));
        }
        out.trim_end().to_string()
    }
}

pub struct MissionSection;
impl ContextSection for MissionSection {
    fn id(&self) -> &'static str {
        "mission"
    }
    fn title(&self) -> &'static str {
        "Mission"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, _pkg: &atlas_core::ContextPackage) -> bool {
        true
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        let header = format!("AI Execution Briefing: {}", pkg.target_id);
        out.push_str(&header);
        out.push('\n');
        out.push_str(&"=".repeat(header.len()));
        out.push_str("\n\n");

        if let Some(ref m) = pkg.mission {
            out.push_str("Mission\n-------\n\n");
            out.push_str(&format!("{:<22}: {}\n", "Target Feature", m.target_feature));
            out.push_str(&format!("{:<22}: {}\n", "Business Objective", m.business_objective));
            out.push_str(&format!("{:<22}: {}\n", "Expected Outcome", m.expected_outcome));
            out.push_str(&format!("{:<22}: {}\n", "Repository", m.repository));
            out.push_str(&format!("{:<22}: {}\n", "Estimated Complexity", m.estimated_complexity));
        } else {
            out.push_str("Mission\n-------\n\n");
            out.push_str(&format!("{:<22}: {}\n", "Target Feature", pkg.title));
            out.push_str(&format!("{:<22}: Execute feature specifications.\n", "Business Objective"));
            out.push_str(&format!("{:<22}: Derived from business requirements.\n", "Expected Outcome"));
            out.push_str(&format!("{:<22}: {}\n", "Repository", pkg.repository.as_deref().unwrap_or("Unspecified")));
            out.push_str(&format!("{:<22}: Medium\n", "Estimated Complexity"));
        }
        out.trim_end().to_string()
    }
}

pub struct EvidenceRankingSection;
impl ContextSection for EvidenceRankingSection {
    fn id(&self) -> &'static str {
        "evidence_ranking"
    }
    fn title(&self) -> &'static str {
        "Evidence Ranking"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.evidence_ranking.is_empty() || !pkg.recommended_reading.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Evidence Ranking\n----------------\n\n");

        if !pkg.evidence_ranking.is_empty() {
            let high: Vec<_> = pkg.evidence_ranking.iter().filter(|e| e.confidence_level == "High Confidence").collect();
            let med: Vec<_> = pkg.evidence_ranking.iter().filter(|e| e.confidence_level == "Medium Confidence").collect();
            let low: Vec<_> = pkg.evidence_ranking.iter().filter(|e| e.confidence_level == "Low Confidence").collect();

            if !high.is_empty() {
                out.push_str("High Confidence\n");
                for item in high {
                    out.push_str(&format!("  • {} ({}) [{}]\n", item.artifact_id, item.title, item.kind));
                    out.push_str(&format!("    Reason: {}\n", item.reason));
                }
                out.push('\n');
            }

            if !med.is_empty() {
                out.push_str("Medium Confidence\n");
                for item in med {
                    out.push_str(&format!("  • {} ({}) [{}]\n", item.artifact_id, item.title, item.kind));
                    out.push_str(&format!("    Reason: {}\n", item.reason));
                }
                out.push('\n');
            }

            if !low.is_empty() {
                out.push_str("Low Confidence\n");
                for item in low {
                    out.push_str(&format!("  • {} ({}) [{}]\n", item.artifact_id, item.title, item.kind));
                    out.push_str(&format!("    Reason: {}\n", item.reason));
                }
                out.push('\n');
            }
        } else {
            for item in &pkg.recommended_reading {
                out.push_str(&format!("{} {} ({})\n", item.star_rating, item.source_id, item.title));
                out.push_str(&format!("Reason: {}\n\n", item.reason));
            }
        }

        out.trim_end().to_string()
    }
}

pub struct ImplementationAreasSection;
impl ContextSection for ImplementationAreasSection {
    fn id(&self) -> &'static str {
        "implementation_areas"
    }
    fn title(&self) -> &'static str {
        "Possible Implementation Areas"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        pkg.implementation_areas.is_some() || pkg.hypothesis.is_some()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Possible Implementation Areas\n-----------------------------\n\n");

        if let Some(ref areas) = pkg.implementation_areas {
            if !areas.business_rules.is_empty() {
                out.push_str("Business Rules:\n");
                for rule in &areas.business_rules {
                    out.push_str(&format!("  • {}\n", rule));
                }
                out.push('\n');
            }

            if !areas.potential_components.is_empty() {
                out.push_str("Potential Components:\n");
                for comp in &areas.potential_components {
                    out.push_str(&format!("  • {}\n", comp));
                }
                out.push('\n');
            }

            out.push_str(&format!("{:<22}: {}\n", "Likely Impact", areas.impact_level));
            out.push_str(&format!("{:<22}: {}\n", "Confidence", areas.confidence));
            out.push_str(&format!("{:<22}: {}\n", "Note", areas.uncertainty_note));
        } else if let Some(ref hyp) = pkg.hypothesis {
            if !hyp.likely_modified_modules.is_empty() {
                out.push_str("Potential Components:\n");
                for m in &hyp.likely_modified_modules {
                    out.push_str(&format!("  • {}\n", m.module_name));
                }
                out.push('\n');
            }
            out.push_str(&format!("{:<22}: {}\n", "Likely Impact", hyp.impact_level));
            out.push_str(&format!("{:<22}: {}\n", "Confidence", hyp.confidence));
        }

        out.trim_end().to_string()
    }
}

pub struct ExecutionQueueSection;
impl ContextSection for ExecutionQueueSection {
    fn id(&self) -> &'static str {
        "execution_queue"
    }
    fn title(&self) -> &'static str {
        "Execution Queue"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.execution_queue.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Execution Queue\n---------------\n\n");

        let required_steps: Vec<_> = pkg.execution_queue.iter().filter(|s| s.category == "Required" || s.category.is_empty()).collect();
        let optional_steps: Vec<_> = pkg.execution_queue.iter().filter(|s| s.category == "Optional").collect();

        if !required_steps.is_empty() {
            out.push_str("Required\n");
            for (idx, step) in required_steps.iter().enumerate() {
                if idx > 0 {
                    out.push_str("\n↓\n\n");
                }
                let num_symbol = match idx + 1 {
                    1 => "①", 2 => "②", 3 => "③", 4 => "④", 5 => "⑤",
                    _ => "•",
                };
                out.push_str(&format!("{} {}\n", num_symbol, step.title));
                if let Some(ref label) = step.artifact_label {
                    out.push_str(&format!("   Target  : {}\n", label));
                }
                out.push_str(&format!("   Reason  : {}\n", step.reason));
                if let Some(ref cmd) = step.command {
                    out.push_str(&format!("   Command : {}\n", cmd));
                }
            }
            out.push('\n');
        }

        if !optional_steps.is_empty() {
            out.push_str("Optional\n");
            for (idx, step) in optional_steps.iter().enumerate() {
                let num_symbol = match required_steps.len() + idx + 1 {
                    1 => "①", 2 => "②", 3 => "③", 4 => "④", 5 => "⑤", 6 => "⑥",
                    _ => "•",
                };
                out.push_str(&format!("{} {}\n", num_symbol, step.title));
                if let Some(ref label) = step.artifact_label {
                    out.push_str(&format!("   Target  : {}\n", label));
                }
                out.push_str(&format!("   Reason  : {}\n", step.reason));
                if let Some(ref cmd) = step.command {
                    out.push_str(&format!("   Command : {}\n", cmd));
                }
            }
        }

        out.trim_end().to_string()
    }
}

pub struct ImplementationRisksSection;
impl ContextSection for ImplementationRisksSection {
    fn id(&self) -> &'static str {
        "risks"
    }
    fn title(&self) -> &'static str {
        "Implementation Risks"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.risks.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Implementation Risks\n--------------------\n\n");
        for risk in &pkg.risks {
            let label = if !risk.area.is_empty() {
                format!("{}: {}", risk.level, risk.area)
            } else {
                risk.level.clone()
            };
            out.push_str(&format!("{}\n  • Description: {}\n", label, risk.description));
            if !risk.evidence.is_empty() {
                out.push_str(&format!("  • Evidence   : {}\n", risk.evidence));
            }
            out.push('\n');
        }
        out.trim_end().to_string()
    }
}

pub struct KnowledgeGapsSection;
impl ContextSection for KnowledgeGapsSection {
    fn id(&self) -> &'static str {
        "gaps"
    }
    fn title(&self) -> &'static str {
        "Knowledge Gaps"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.classified_gaps.is_empty()
            || !pkg.prioritized_gaps.critical.is_empty()
            || !pkg.prioritized_gaps.recommended.is_empty()
            || !pkg.prioritized_gaps.optional.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Knowledge Gaps\n--------------\n\n");

        if !pkg.classified_gaps.is_empty() {
            out.push_str("Blocking Unknowns\n");
            for gap in &pkg.classified_gaps {
                out.push_str(&format!("{}\n", gap.severity));
                out.push_str(&format!("  • {}\n", gap.gap_type));
                out.push_str(&format!("    Impact             : {}\n", gap.impact));
                out.push_str(&format!("    Suggested Retrieval: {}\n\n", gap.suggested_retrieval));
            }
        } else {
            let gaps = &pkg.prioritized_gaps;
            if !gaps.critical.is_empty() {
                out.push_str("Critical:\n");
                for item in &gaps.critical {
                    out.push_str(&format!("• {}\n", item));
                }
                out.push('\n');
            }

            if !gaps.recommended.is_empty() {
                out.push_str("Recommended:\n");
                for item in &gaps.recommended {
                    out.push_str(&format!("• {}\n", item));
                }
                out.push('\n');
            }

            if !gaps.optional.is_empty() {
                out.push_str("Optional:\n");
                for item in &gaps.optional {
                    out.push_str(&format!("• {}\n", item));
                }
            }
        }

        out.trim_end().to_string()
    }
}

pub struct RetrievedContextSection;
impl ContextSection for RetrievedContextSection {
    fn id(&self) -> &'static str {
        "retrieved_context"
    }
    fn title(&self) -> &'static str {
        "Retrieved Context"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.architecture_decisions.is_empty()
            || !pkg.apis.is_empty()
            || !pkg.related_pull_requests.is_empty()
            || !pkg.related_commits.is_empty()
            || !pkg.related_artifacts.is_empty()
            || pkg.repository.is_some()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Retrieved Context\n-----------------\n\n");

        if !pkg.architecture_decisions.is_empty() {
            out.push_str("Technical Specifications:\n");
            for adr in &pkg.architecture_decisions {
                out.push_str(&format!("★★★★★ {}\n", format_related_item(&adr.artifact)));
            }
            out.push('\n');
        }

        if !pkg.apis.is_empty() {
            out.push_str("API Contracts:\n");
            for api in &pkg.apis {
                out.push_str(&format!("★★★★ {}\n", format_related_item(&api.artifact)));
            }
            out.push('\n');
        }

        if !pkg.related_pull_requests.is_empty() || !pkg.related_commits.is_empty() {
            out.push_str("Existing Features / PRs:\n");
            for pr in &pkg.related_pull_requests {
                let repo_str = pr.artifact.repository.as_deref().unwrap_or("N/A");
                out.push_str(&format!("★★ PR {} ({}) | Repo: {}\n", pr.artifact.source_id, pr.artifact.title, repo_str));
            }
            for commit in &pkg.related_commits {
                let repo_str = commit.artifact.repository.as_deref().unwrap_or("N/A");
                out.push_str(&format!("★★ Commit {} ({}) | Repo: {}\n", commit.artifact.source_id, commit.artifact.title, repo_str));
            }
            out.push('\n');
        }

        if let Some(ref repo) = pkg.repository {
            out.push_str("Repositories:\n");
            out.push_str(&format!("{}\n\n", repo));
        }

        if !pkg.related_artifacts.is_empty() {
            out.push_str("Related Tickets:\n");
            for item in &pkg.related_artifacts {
                out.push_str(&format!("{}\n", item.artifact.source_id));
            }
        }

        out.trim_end().to_string()
    }
}

pub struct AiGuidanceSection;
impl ContextSection for AiGuidanceSection {
    fn id(&self) -> &'static str {
        "ai_guidance"
    }
    fn title(&self) -> &'static str {
        "Guidance for AI Agent"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.ai_guidance_bullets.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Guidance for AI Agent\n---------------------\n\n");
        for bullet in &pkg.ai_guidance_bullets {
            out.push_str(&format!("• {}\n", bullet));
        }
        out.trim_end().to_string()
    }
}

pub struct SuggestedCommandsSection;
impl ContextSection for SuggestedCommandsSection {
    fn id(&self) -> &'static str {
        "next_commands"
    }
    fn title(&self) -> &'static str {
        "Suggested Commands"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        !pkg.execution_queue.is_empty()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Suggested Commands\n------------------\n\n");

        for step in &pkg.execution_queue {
            if let Some(ref cmd) = step.command {
                out.push_str(&format!("{}\n", cmd));
            }
        }

        out.trim_end().to_string()
    }
}

pub struct SourceInfoSection;
impl ContextSection for SourceInfoSection {
    fn id(&self) -> &'static str {
        "source_info"
    }
    fn title(&self) -> &'static str {
        "Source Information"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, _pkg: &atlas_core::ContextPackage) -> bool {
        true
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let mut out = String::new();
        out.push_str("Source Information\n------------------\n\n");
        out.push_str(&format!("{:<14}: {}\n", "Provider", pkg.source_info.provider));
        out.push_str(&format!(
            "{:<14}: {}\n",
            "Repository",
            pkg.source_info.repository.as_deref().unwrap_or("N/A")
        ));
        out.push_str(&format!("{:<14}: {}\n", "Updated", pkg.source_info.updated_at));
        out.push_str(&format!("{:<14}: {}\n", "Source URL", pkg.source_info.source_url));
        out.push_str(&format!("{:<14}: {}\n", "Last Synced", pkg.source_info.synced_at));
        out.trim_end().to_string()
    }
}

pub struct TelemetrySection;
impl ContextSection for TelemetrySection {
    fn id(&self) -> &'static str {
        "telemetry"
    }
    fn title(&self) -> &'static str {
        "Performance Telemetry"
    }
    fn aspect(&self) -> Option<DomainAspect> {
        None
    }
    fn should_render(&self, pkg: &atlas_core::ContextPackage) -> bool {
        pkg.telemetry.is_some()
    }
    fn render(&self, pkg: &atlas_core::ContextPackage, _verbose: bool) -> String {
        let t = match pkg.telemetry {
            Some(ref t) => t,
            None => return String::new(),
        };
        let mut out = String::new();
        out.push_str("Performance Telemetry (Profiling)\n----------------------------------\n");
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "Primary Artifact Resolution", t.primary_resolution_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "1-Hop Graph Header Traversal", t.hop1_traversal_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "2-Hop Graph Header Traversal", t.hop2_traversal_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "Repository / FTS Search", t.repo_fts_search_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "Candidate Header Ranking", t.candidate_ranking_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "Batch Artifact Hydration", t.artifact_hydration_ms
        ));
        out.push_str(&format!(
            "{:<32}: {} ms\n",
            "Prompt & Briefing Assembly", t.prompt_assembly_ms
        ));
        out.push_str(&format!("{:<32}: {} ms\n", "TOTAL TIME", t.total_ms));
        out.push_str(&format!(
            "{:<32}: {}\n",
            "Candidate Headers Evaluated", t.candidate_headers_count
        ));
        out.push_str(&format!(
            "{:<32}: {}",
            "Artifacts Hydrated", t.hydrated_artifacts_count
        ));
        out
    }
}

pub struct LayoutCompositor {
    sections: Vec<Box<dyn ContextSection>>,
}

impl LayoutCompositor {
    pub fn default_pipeline() -> Self {
        Self {
            sections: vec![
                Box::new(KnownFactsSection),
                Box::new(MissionSection),
                Box::new(EvidenceRankingSection),
                Box::new(ImplementationAreasSection),
                Box::new(ExecutionQueueSection),
                Box::new(KnowledgeGapsSection),
                Box::new(ImplementationRisksSection),
                Box::new(RetrievedContextSection),
                Box::new(AiGuidanceSection),
                Box::new(SuggestedCommandsSection),
                Box::new(SourceInfoSection),
                Box::new(TelemetrySection),
            ],
        }
    }

    pub fn compose(&self, pkg: &atlas_core::ContextPackage, verbose: bool) -> String {
        let mut out = String::new();
        for section in &self.sections {
            if section.should_render(pkg) {
                let text = section.render(pkg, verbose);
                if !text.trim().is_empty() {
                    if !out.is_empty() {
                        out.push_str("\n\n");
                    }
                    out.push_str(&text);
                }
            }
        }
        out
    }
}

pub fn format_context_package(
    pkg: &atlas_core::ContextPackage,
    verbose: bool,
    raw: bool,
) -> String {
    if raw {
        return format!("{:?}", pkg);
    }

    let compositor = LayoutCompositor::default_pipeline();
    compositor.compose(pkg, verbose)
}

fn format_dots(label: &str, value: &str, total_width: usize) -> String {
    let label_chars = label.chars().count();
    let val_chars = value.chars().count();
    let needed = total_width.saturating_sub(label_chars + val_chars + 2);
    let dots = ".".repeat(needed.max(3));
    format!("{} {} {}", label, dots, value)
}

fn group_related_artifacts(pkg: &atlas_core::ContextPackage) -> Vec<(String, Vec<(String, String)>)> {
    let mut groups: std::collections::BTreeMap<String, Vec<(String, String)>> = std::collections::BTreeMap::new();

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
            let cat_name = match item.artifact.kind {
                ArtifactKind::Repository => "Repositories",
                ArtifactKind::PullRequest | ArtifactKind::PullRequestReview | ArtifactKind::ReviewComment => "Pull Requests",
                ArtifactKind::Commit => "Commits",
                ArtifactKind::Release => "Release History",
                ArtifactKind::Ticket | ArtifactKind::Issue => "Related Tickets",
                ArtifactKind::Document | ArtifactKind::Specification | ArtifactKind::Design => "Architecture & Specs",
                _ => "Other Related Items",
            }.to_string();

            let id_display = format_related_item(&item.artifact);
            if id_display.starts_with("repo_") || id_display.len() <= 2 {
                continue;
            }

            let entry = groups.entry(cat_name).or_default();
            if !entry.iter().any(|(id, _)| id == &id_display) {
                entry.push((id_display, item.relationship_label.clone()));
            }
        }
    }

    if !pkg.affected_repositories.is_empty() {
        let entry = groups.entry("Repositories".to_string()).or_default();
        for repo in &pkg.affected_repositories {
            if !entry.iter().any(|(id, _)| id == repo) {
                entry.push((repo.clone(), "target repo".to_string()));
            }
        }
    }

    groups.into_iter().collect()
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
            target_aspects: std::collections::HashSet::from([atlas_core::DomainAspect::CodeImplementation]),
            primary_artifact: Some(art),
            title: "MongoDB Atlas Migration".to_string(),
            status: "Done".to_string(),
            repository: Some("INIT".to_string()),
            description: Some("Migrate MongoDB infrastructure to Atlas...".to_string()),
            overview_summary: "MongoDB Atlas Migration (INIT-219) represents a ticket artifact from Jira.".to_string(),
            mission: Some(atlas_core::Mission {
                target_feature: "INIT-219 MongoDB Atlas Migration".to_string(),
                business_objective: "Migrate database infrastructure to Atlas.".to_string(),
                expected_outcome: "Derived from retrieved business requirements in INIT-219".to_string(),
                repository: "INIT".to_string(),
                estimated_complexity: "Medium".to_string(),
                goal: "Implement MongoDB Atlas Migration".to_string(),
                objective: "Migrate database infrastructure to Atlas.".to_string(),
                complexity: "Medium".to_string(),
            }),
            known_facts: vec![atlas_core::KnownFact {
                statement: "Target repository identified (INIT)".to_string(),
                is_verified: true,
                source_artifact: Some("INIT".to_string()),
            }],
            evidence_ranking: vec![atlas_core::EvidenceItem {
                artifact_id: "ADR-007".to_string(),
                title: "MongoDB Architecture".to_string(),
                kind: "Specification".to_string(),
                confidence_level: "High Confidence".to_string(),
                star_rating: "★★★★★".to_string(),
                reason: "Direct technical specification".to_string(),
            }],
            implementation_areas: Some(atlas_core::PossibleImplementationAreas {
                business_rules: vec!["Database migration rules".to_string()],
                potential_components: vec!["Database Driver".to_string()],
                impact_level: "Medium".to_string(),
                confidence: "High".to_string(),
                uncertainty_note: "Candidate implementation domains".to_string(),
            }),
            understanding: Some(atlas_core::CurrentUnderstanding {
                business_rules: vec!["Migrate MongoDB infrastructure to Atlas...".to_string()],
                affected_domains: vec!["Database".to_string()],
                known_constraints: vec!["Zero downtime".to_string()],
            }),
            hypothesis: Some(atlas_core::ImplementationHypothesis {
                scope: vec![atlas_core::ScopeItem { area: "Core Business Domain Logic".to_string(), is_likely: true }],
                primary_flow: vec!["Database Driver -> Mongo Atlas Cluster".to_string()],
                likely_modified_modules: vec![atlas_core::ModuleRating { module_name: "Database Config".to_string(), rating_stars: "★★★★★".to_string() }],
                potential_integrations: vec!["MongoDB Atlas".to_string()],
                impact_level: "Medium".to_string(),
                estimated_components: "2-4 Components".to_string(),
                confidence: "High".to_string(),
            }),
            execution_queue: vec![atlas_core::QueueStep {
                step_index: 1,
                total_steps: 3,
                category: "Required".to_string(),
                title: "Read technical specification".to_string(),
                artifact_label: Some("ADR-007".to_string()),
                reason: "Contains architecture rules.".to_string(),
                command: Some("atx context ADR-007".to_string()),
                status: "Pending".to_string(),
            }],
            risks: vec![atlas_core::ImplementationRisk {
                level: "Potential Risk".to_string(),
                area: "Shared Cluster".to_string(),
                description: "Shared database cluster.".to_string(),
                evidence: "Repository metadata indicates shared database usage.".to_string(),
            }],
            classified_gaps: vec![atlas_core::ClassifiedKnowledgeGap {
                severity: "HIGH".to_string(),
                gap_type: "Architecture Decision".to_string(),
                impact: "Boundaries uncertain".to_string(),
                suggested_retrieval: "atx search architecture mongo".to_string(),
            }],
            prioritized_gaps: atlas_core::PrioritizedKnowledgeGaps {
                critical: vec![],
                recommended: vec!["Previous implementation PRs missing.".to_string()],
                optional: vec![],
            },
            investigation_steps: vec![atlas_core::InvestigationStep {
                step_number: 1,
                goal: "Understand primary requirements.".to_string(),
                inspect_target: "INIT-219".to_string(),
                expected_outcome: "Understand core feature description.".to_string(),
            }],
            unknowns: vec!["Target repository is not explicitly identified.".to_string()],
            investigation_status: vec![atlas_core::StatusCheck {
                label: "Business requirements available".to_string(),
                is_available: true,
            }],
            ai_guidance: Some(atlas_core::AiGuidance {
                artifact_nature: "Code Implementation Task".to_string(),
                exploration_strategy: "Review ADR-007 before coding.".to_string(),
            }),
            ai_guidance_bullets: vec![
                "Treat Known Facts as authoritative.".to_string(),
                "Prioritize High Confidence evidence.".to_string(),
            ],
            engineering_readiness: atlas_core::EngineeringReadiness {
                status_label: "Ready for implementation.".to_string(),
                readiness_summary: "Atlas found sufficient context.".to_string(),
                available: vec!["Repository".to_string()],
                missing: vec!["Pull Requests".to_string()],
            },
            completeness: atlas_core::CompletenessReport {
                score_percentage: 62,
                progress_bar: "██████░░░░".to_string(),
                category_scores: vec![atlas_core::CategoryScore {
                    category_name: "Business Context".to_string(),
                    score_percentage: 100,
                    progress_bar: "██████████".to_string(),
                    is_available: true,
                }],
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
                star_rating: "★★★★★".to_string(),
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
            ai_briefing: None,
            source_info: atlas_core::SourceInfo {
                provider: "Jira".to_string(),
                repository: Some("INIT".to_string()),
                source_url: "https://jira.example.com/browse/INIT-219".to_string(),
                updated_at: "2026-07-30".to_string(),
                synced_at: "2026-07-31 15:45 UTC".to_string(),
            },
            summary: "Implementation can begin. Business requirements are available.".to_string(),
            telemetry: None,
        };

        let formatted = format_context_package(&pkg, false, false);

        assert!(formatted.contains("AI Execution Briefing: INIT-219"));
        assert!(formatted.contains("Known Facts"));
        assert!(formatted.contains("Mission"));
        assert!(formatted.contains("Evidence Ranking"));
        assert!(formatted.contains("Possible Implementation Areas"));
        assert!(formatted.contains("Execution Queue"));
        assert!(formatted.contains("Knowledge Gaps"));
        assert!(formatted.contains("Implementation Risks"));
        assert!(formatted.contains("Guidance for AI Agent"));
        assert!(formatted.contains("Source Information"));
        assert!(formatted.contains("Provider"));
        assert!(formatted.contains("Jira"));
    }
}
