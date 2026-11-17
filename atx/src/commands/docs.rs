use crate::formatter;
use anyhow::Result;
use atlas_core::Storage;

pub fn list_or_search_docs(storage: &Storage, query: Option<&str>) -> Result<()> {
    let artifacts = if let Some(q) = query {
        if !q.trim().is_empty() {
            storage.search_fts(q, Some("document"), None, None, 50)?
        } else {
            storage.query_structured(Some("document"), None, None, 100)?
        }
    } else {
        storage.query_structured(Some("document"), None, None, 100)?
    };

    if artifacts.is_empty() {
        if let Some(q) = query {
            println!("No documents matching '{}' found in context graph.", q);
        } else {
            println!("No documents indexed yet. Run 'atx sync' to synchronize documents from connectors.");
        }
        return Ok(());
    }

    println!("Found {} indexed document(s):\n", artifacts.len());
    println!("┌─{0:─<4}─┬─{0:─<40}─┬─{0:─<35}─┬─{0:─<12}─┐", "");
    println!("│ {:<4} │ {:<40} │ {:<35} │ {:<12} │", "#", "DOCUMENT TITLE", "PATH / SOURCE ID", "PROVIDER");
    println!("├─{0:─<4}─┼─{0:─<40}─┼─{0:─<35}─┼─{0:─<12}─┤", "");

    for (idx, art) in artifacts.iter().enumerate() {
        let title = formatter::safe_truncate(&art.title, 40);
        let path = formatter::safe_truncate(&art.source_id, 35);
        let provider = formatter::safe_truncate(&art.provider, 12);
        println!("│ {:<4} │ {:<40} │ {:<35} │ {:<12} │", idx + 1, title, path, provider);
    }
    println!("└─{0:─<4}─┴─{0:─<40}─┴─{0:─<35}─┴─{0:─<12}─┘", "");
    println!("\n💡 Tip: Run 'atx doc <title_or_path>' to read the full document in terminal.");
    println!("        Use 'atx doc <title_or_path> --raw' to output clean markdown (pipeable to glow/pager).");

    Ok(())
}

pub fn display_doc(storage: &Storage, target: &str, raw: bool, json_output: bool) -> Result<()> {
    // 1. Try alias resolution first
    let mut candidates = storage.resolve_artifact_by_alias(target)?;

    // 2. If no alias resolution, try search in documents
    if candidates.is_empty() {
        candidates = storage.search_fts(target, Some("document"), None, None, 10)?;
    }

    // 3. If still empty, search across all artifact kinds
    if candidates.is_empty() {
        candidates = storage.search_fts(target, None, None, None, 10)?;
    }

    if candidates.is_empty() {
        println!("Artifact or document '{}' not found in context graph.", target);
        println!("Run 'atx docs' to browse documents, or 'atx search \"{}\"' to search globally.", target);
        return Ok(());
    }

    // Check if one candidate is an exact match on source_id or title
    let target_trimmed = target.trim();
    let exact_match = candidates.iter().position(|c| {
        c.source_id.eq_ignore_ascii_case(target_trimmed)
            || c.source_id.ends_with(target_trimmed)
            || c.title.eq_ignore_ascii_case(target_trimmed)
    });

    let selected = if candidates.len() == 1 {
        &candidates[0]
    } else if let Some(idx) = exact_match {
        &candidates[idx]
    } else {
        println!("Found {} artifacts matching '{}':\n", candidates.len(), target);
        for (idx, c) in candidates.iter().enumerate() {
            println!("{:2}. [{}] {} ({})", idx + 1, c.kind.to_string().to_uppercase(), c.title, c.source_id);
        }
        println!("\nSpecify exact ID or path: 'atx cat <id>' or 'atx doc <path>'");
        return Ok(());
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(selected)?);
        return Ok(());
    }

    if raw {
        print!("{}", selected.body);
        if !selected.body.ends_with('\n') {
            println!();
        }
        return Ok(());
    }

    // Formatted terminal document view
    let divider = "═".repeat(70);
    println!("{}", divider);
    let kind_upper = selected.kind.to_string().to_uppercase();
    let kind_icon = match selected.kind {
        atlas_core::ArtifactKind::Ticket => "🎫",
        atlas_core::ArtifactKind::PullRequest => "🔀",
        atlas_core::ArtifactKind::Commit => "💾",
        atlas_core::ArtifactKind::Issue => "⚠️",
        atlas_core::ArtifactKind::Design => "🎨",
        _ => "📄",
    };
    println!("{} [{}] {}", kind_icon, kind_upper, selected.title);
    println!("ID: {} | Provider: {}", selected.source_id, selected.provider);
    if let Some(ref repo) = selected.repository {
        println!("Repository: {}", repo);
    }
    if !selected.source_url.is_empty() {
        println!("URL: {}", selected.source_url);
    }
    println!("{}", divider);
    println!();
    println!("{}", selected.body);
    println!();
    println!("{}", "─".repeat(70));
    println!("💡 Hint: Use 'atx cat \"{}\" --raw' for pure content without headers.", target);

    Ok(())
}
