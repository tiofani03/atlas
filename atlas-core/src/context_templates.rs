use crate::context::ContextPackage;
use std::fs;
use std::path::PathBuf;

/// Built-in and custom Markdown context templates.
pub fn render_context_markdown(pkg: &ContextPackage, template: Option<&str>) -> String {
    let tpl_name = template.unwrap_or("default").trim();

    // Check user-defined custom template at ~/.config/atlas/templates/<name>.md
    if let Some(custom) = load_custom_template(tpl_name) {
        return apply_placeholders(&custom, pkg);
    }

    match tpl_name.to_lowercase().as_str() {
        "pr-review" | "pr_review" => render_pr_review(pkg),
        "incident" => render_incident(pkg),
        "onboarding" => render_onboarding(pkg),
        _ => render_default(pkg),
    }
}

fn load_custom_template(name: &str) -> Option<String> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home)
        .join(".config")
        .join("atlas")
        .join("templates")
        .join(format!("{}.md", name));
    fs::read_to_string(path).ok()
}

fn apply_placeholders(template: &str, pkg: &ContextPackage) -> String {
    let facts_md = pkg
        .known_facts
        .iter()
        .map(|f| format!("- {}", f.statement))
        .collect::<Vec<_>>()
        .join("\n");

    let related_md = pkg
        .related_artifacts
        .iter()
        .map(|r| {
            format!(
                "- `[{}]` **{}** (`{}`)",
                r.artifact.kind, r.artifact.title, r.artifact.source_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let actions_md = pkg
        .suggested_next_actions
        .iter()
        .map(|a| format!("- [Step {}] {}: {}", a.step, a.action, a.detail))
        .collect::<Vec<_>>()
        .join("\n");

    let provider = pkg
        .primary_artifact
        .as_ref()
        .map(|a| a.provider.as_str())
        .unwrap_or("atlas");
    let body = pkg
        .primary_artifact
        .as_ref()
        .map(|a| a.body.as_str())
        .or(pkg.description.as_deref())
        .unwrap_or("");

    template
        .replace("{{title}}", &pkg.title)
        .replace("{{id}}", &pkg.target_id)
        .replace("{{kind}}", &pkg.target_kind)
        .replace("{{provider}}", provider)
        .replace("{{summary}}", &pkg.summary)
        .replace("{{body}}", body)
        .replace("{{facts}}", &facts_md)
        .replace("{{related}}", &related_md)
        .replace("{{next_actions}}", &actions_md)
}

fn render_default(pkg: &ContextPackage) -> String {
    let mut out = String::new();
    let provider = pkg
        .primary_artifact
        .as_ref()
        .map(|a| a.provider.as_str())
        .unwrap_or("atlas");

    out.push_str(&format!("# {} (`{}`)\n\n", pkg.title, pkg.target_id));
    out.push_str(&format!(
        "- **Kind**: `{}` | **Provider**: `{}` | **Readiness**: `{}%`\n",
        pkg.target_kind, provider, pkg.completeness.score_percentage
    ));
    if let Some(ref primary) = pkg.primary_artifact {
        if !primary.source_url.is_empty() {
            out.push_str(&format!("- **URL**: {}\n", primary.source_url));
        }
    }
    out.push('\n');

    if !pkg.summary.trim().is_empty() {
        out.push_str(&format!("## Summary\n\n{}\n\n", pkg.summary.trim()));
    }

    if let Some(ref desc) = pkg.description {
        if !desc.trim().is_empty() {
            out.push_str(&format!("## Description\n\n{}\n\n", desc.trim()));
        }
    }

    if !pkg.known_facts.is_empty() {
        out.push_str("## Known Facts\n\n");
        for f in &pkg.known_facts {
            out.push_str(&format!("- {}\n", f.statement));
        }
        out.push('\n');
    }

    if !pkg.related_artifacts.is_empty() {
        out.push_str(&format!(
            "## Related Artifacts ({})\n\n",
            pkg.related_artifacts.len()
        ));
        for rel in &pkg.related_artifacts {
            out.push_str(&format!(
                "- `[{}]` **{}** (`{}` via `{}`)\n",
                rel.artifact.kind,
                rel.artifact.title,
                rel.artifact.source_id,
                rel.relationship_label
            ));
        }
        out.push('\n');
    }

    if !pkg.suggested_next_actions.is_empty() {
        out.push_str("## Suggested Next Actions\n\n");
        for act in &pkg.suggested_next_actions {
            out.push_str(&format!("- **[Step {}]** {}: {}\n", act.step, act.action, act.detail));
        }
        out.push('\n');
    }

    out
}

fn render_pr_review(pkg: &ContextPackage) -> String {
    let mut out = String::new();
    out.push_str(&format!("# PR Review Context: {}\n\n", pkg.title));
    out.push_str(&format!("Target: `{}` ({})\n\n", pkg.target_id, pkg.target_kind));
    out.push_str("## Change Overview\n\n");
    out.push_str(pkg.description.as_deref().unwrap_or(&pkg.summary));
    out.push_str("\n\n## Linked Tickets, Docs & Commits\n\n");
    for rel in &pkg.related_artifacts {
        out.push_str(&format!(
            "- `[{}]` {} (`{}`)\n",
            rel.artifact.kind, rel.artifact.title, rel.artifact.source_id
        ));
    }
    out.push_str("\n## Missing Context / Gaps\n\n");
    for gap in &pkg.completeness.missing_categories {
        out.push_str(&format!("- {}\n", gap.category));
    }
    out
}

fn render_incident(pkg: &ContextPackage) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Incident Context: {}\n\n", pkg.title));
    out.push_str("## Symptoms & Details\n\n");
    out.push_str(pkg.description.as_deref().unwrap_or(&pkg.summary));
    out.push_str("\n\n## Recent Related Changes\n\n");
    for rel in &pkg.related_artifacts {
        out.push_str(&format!(
            "- `[{}]` **{}** (`{}`)\n",
            rel.artifact.kind, rel.artifact.title, rel.artifact.source_id
        ));
    }
    out
}

fn render_onboarding(pkg: &ContextPackage) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Onboarding Brief: {}\n\n", pkg.title));
    out.push_str(&render_default(pkg));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextBuilder, ContextOptions};
    use crate::domain::{ArtifactKind, KnowledgeArtifact};
    use crate::storage::Storage;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_render_context_markdown_templates() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().join("test.db")).unwrap();
        let art = KnowledgeArtifact {
            id: "ticket:PROJ-1".to_string(),
            kind: ArtifactKind::Ticket,
            title: "Implement OAuth2 Login".to_string(),
            summary: Some("Add OAuth2 support".to_string()),
            body: "Detailed spec for OAuth2".to_string(),
            provider: "jira".to_string(),
            source_id: "PROJ-1".to_string(),
            source_url: "https://jira.example.com/browse/PROJ-1".to_string(),
            repository: None,
            tags: vec![],
            relationships: vec![],
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: "chk1".to_string(),
            metadata: serde_json::json!({}),
        };
        storage.upsert_artifacts_batch(&[art]).unwrap();
        let builder = ContextBuilder::new(&storage);
        let pkg = builder
            .build(None, "PROJ-1", &ContextOptions::default())
            .unwrap();

        let md_default = render_context_markdown(&pkg, None);
        assert!(md_default.contains("# Implement OAuth2 Login (`PROJ-1`)"));

        let md_pr = render_context_markdown(&pkg, Some("pr-review"));
        assert!(md_pr.contains("# PR Review Context: Implement OAuth2 Login"));
    }
}
