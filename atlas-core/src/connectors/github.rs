use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use std::collections::HashSet;

pub struct GithubConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl GithubConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token().ok();

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Atlas-Engine/0.2.0"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        if let Some(t) = token {
            if !t.is_empty() {
                let auth_header = if t.starts_with("bearer ") || t.starts_with("token ") {
                    t
                } else {
                    format!("token {}", t)
                };
                if let Ok(val) = HeaderValue::from_str(&auth_header) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn extract_text_relationships(
        text: &str,
        source_id: &str,
        default_repo: &str,
    ) -> Vec<ArtifactRelationship> {
        let mut rels = Vec::new();
        let mut seen = HashSet::new();

        for word in text.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '#' && c != '/' && c != '-');
            if let Some(idx) = clean_word.find('#') {
                let repo_prefix = &clean_word[..idx];
                let num_part = &clean_word[idx + 1..];

                if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                    let target_id = if repo_prefix.is_empty() {
                        format!("{}#{}", default_repo, num_part)
                    } else if repo_prefix.contains('/') {
                        format!("{}#{}", repo_prefix, num_part)
                    } else {
                        continue;
                    };

                    if target_id != source_id && seen.insert(target_id.clone()) {
                        rels.push(ArtifactRelationship {
                            source_id: source_id.to_string(),
                            target_id,
                            relationship_type: "references".to_string(),
                        });
                    }
                }
            }
        }

        rels
    }
}

#[async_trait::async_trait]
impl Connector for GithubConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "github"
    }

    async fn verify(&self) -> Result<String> {
        use anyhow::Context;
        let base_url = if self.config.instance_url.is_empty() {
            "https://api.github.com".to_string()
        } else {
            self.config.instance_url.trim_end_matches('/').to_string()
        };

        let user_url = format!("{}/user", base_url);
        let resp = self.client.get(&user_url).send().await.context("Failed to connect to GitHub API")?;
        if resp.status().is_success() {
            let user_val: Value = resp.json().await.unwrap_or_default();
            let login = user_val["login"].as_str().unwrap_or("authenticated user");
            Ok(format!("Connected to GitHub as @{}.", login))
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("GitHub verification failed: 401 Unauthorized. Check your API token.");
        } else {
            let rate_limit_url = format!("{}/rate_limit", base_url);
            if let Ok(rl_resp) = self.client.get(&rate_limit_url).send().await {
                if rl_resp.status().is_success() {
                    return Ok("Connected to GitHub API successfully.".to_string());
                }
            }
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub verification failed (status {}): {}", status, text);
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let base_url = if self.config.instance_url.is_empty() {
            "https://api.github.com".to_string()
        } else {
            self.config.instance_url.trim_end_matches('/').to_string()
        };

        let mut all_artifacts = Vec::new();

        for repo in &self.config.repos {
            let repo_name = repo.trim();
            if repo_name.is_empty() {
                continue;
            }

            // -------------------------------------------------------------
            // 1. Repository Artifact
            // -------------------------------------------------------------
            let repo_url = format!("{}/repos/{}", base_url, repo_name);
            if let Ok(resp) = self.client.get(&repo_url).send().await {
                if resp.status().is_success() {
                    if let Ok(repo_val) = resp.json::<Value>().await {
                        let desc = repo_val
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let html_url = repo_val
                            .get("html_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let visibility = repo_val
                            .get("visibility")
                            .and_then(|v| v.as_str())
                            .unwrap_or("public");
                        let default_branch = repo_val
                            .get("default_branch")
                            .and_then(|v| v.as_str())
                            .unwrap_or("main");

                        let mut tags = vec![
                            format!("repo:{}", repo_name),
                            format!("visibility:{}", visibility),
                            "type:repository".to_string(),
                        ];

                        if let Some(topics) = repo_val.get("topics").and_then(|v| v.as_array()) {
                            for t in topics {
                                if let Some(top) = t.as_str() {
                                    tags.push(format!("topic:{}", top));
                                }
                            }
                        }

                        let updated_at_str = repo_val
                            .get("updated_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());

                        let created_at_str = repo_val
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let created_at = DateTime::parse_from_rfc3339(created_at_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok();

                        let id = KnowledgeArtifact::generate_id("github", &base_url, repo_name);
                        let checksum = KnowledgeArtifact::compute_checksum(
                            repo_name,
                            Some(&desc),
                            &desc,
                            &tags,
                        );

                        let repo_artifact = KnowledgeArtifact {
                            id,
                            kind: ArtifactKind::Repository,
                            title: repo_name.to_string(),
                            summary: if desc.is_empty() { None } else { Some(desc.clone()) },
                            body: format!(
                                "Repository: {}\nDefault Branch: {}\nVisibility: {}\nDescription: {}",
                                repo_name, default_branch, visibility, desc
                            ),
                            provider: "github".to_string(),
                            source_id: repo_name.to_string(),
                            source_url: html_url,
                            repository: Some(repo_name.to_string()),
                            tags,
                            relationships: Vec::new(),
                            created_at,
                            updated_at,
                            synced_at: Utc::now(),
                            checksum,
                            metadata: repo_val,
                        };

                        all_artifacts.push(repo_artifact);
                    }
                }
            }

            // -------------------------------------------------------------
            // 2. Issues Artifacts
            // -------------------------------------------------------------
            let mut page = 1;
            let per_page = 100;
            loop {
                let mut url = format!(
                    "{}/repos/{}/issues?state=all&per_page={}&page={}",
                    base_url, repo_name, per_page, page
                );
                if let Some(since_dt) = since {
                    url.push_str(&format!("&since={}", since_dt.to_rfc3339()));
                }

                let resp = match self.client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let items: Vec<Value> = match resp.json().await {
                    Ok(i) => i,
                    Err(_) => break,
                };

                if items.is_empty() {
                    break;
                }
                let len = items.len();

                for item in items {
                    // Skip pull requests here as they are processed in PR loop
                    if item.get("pull_request").is_some() {
                        continue;
                    }

                    let number = item.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let body = item.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let state = item.get("state").and_then(|v| v.as_str()).unwrap_or("open");

                    let updated_at_str = item.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
                    let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let created_at_str = item.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                    let created_at = DateTime::parse_from_rfc3339(created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok();

                    let mut tags = vec![
                        format!("state:{}", state),
                        "type:issue".to_string(),
                        format!("repo:{}", repo_name),
                    ];

                    if let Some(labels) = item.get("labels").and_then(|v| v.as_array()) {
                        for label in labels {
                            if let Some(name) = label.get("name").and_then(|v| v.as_str()) {
                                tags.push(name.to_string());
                            }
                        }
                    }

                    let source_id = format!("{}#{}", repo_name, number);
                    let id = KnowledgeArtifact::generate_id("github", &base_url, &source_id);

                    let summary = if body.chars().count() > 300 {
                        Some(body.chars().take(300).collect::<String>() + "...")
                    } else if !body.is_empty() {
                        Some(body.clone())
                    } else {
                        None
                    };

                    let mut relationships = Vec::new();
                    relationships.push(ArtifactRelationship {
                        source_id: repo_name.to_string(),
                        target_id: source_id.clone(),
                        relationship_type: "owns".to_string(),
                    });
                    relationships.extend(Self::extract_text_relationships(&body, &source_id, repo_name));

                    let checksum = KnowledgeArtifact::compute_checksum(&title, summary.as_deref(), &body, &tags);

                    all_artifacts.push(KnowledgeArtifact {
                        id,
                        kind: ArtifactKind::Issue,
                        title,
                        summary,
                        body,
                        provider: "github".to_string(),
                        source_id,
                        source_url: html_url,
                        repository: Some(repo_name.to_string()),
                        tags,
                        relationships,
                        created_at,
                        updated_at,
                        synced_at: Utc::now(),
                        checksum,
                        metadata: item,
                    });
                }

                if len < per_page {
                    break;
                }
                page += 1;
            }

            // -------------------------------------------------------------
            // 3. Pull Requests Artifacts & Reviews
            // -------------------------------------------------------------
            let mut pr_page = 1;
            loop {
                let url = format!(
                    "{}/repos/{}/pulls?state=all&sort=updated&direction=desc&per_page={}&page={}",
                    base_url, repo_name, per_page, pr_page
                );

                let resp = match self.client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let prs: Vec<Value> = match resp.json().await {
                    Ok(items) => items,
                    Err(_) => break,
                };

                if prs.is_empty() {
                    break;
                }
                let len = prs.len();
                let mut reached_watermark = false;

                for item in prs {
                    let number = item.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let body = item.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let state = item.get("state").and_then(|v| v.as_str()).unwrap_or("open");
                    let is_draft = item.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_merged = item.get("merged_at").map(|v| !v.is_null()).unwrap_or(false);

                    let updated_at_str = item.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
                    let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    if let Some(since_dt) = since {
                        if updated_at < since_dt {
                            reached_watermark = true;
                            break;
                        }
                    }

                    let created_at_str = item.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                    let created_at = DateTime::parse_from_rfc3339(created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok();

                    let mut tags = vec![
                        format!("state:{}", state),
                        "type:pull_request".to_string(),
                        format!("repo:{}", repo_name),
                    ];
                    if is_merged {
                        tags.push("merged".to_string());
                    }
                    if is_draft {
                        tags.push("draft".to_string());
                    }

                    if let Some(labels) = item.get("labels").and_then(|v| v.as_array()) {
                        for label in labels {
                            if let Some(name) = label.get("name").and_then(|v| v.as_str()) {
                                tags.push(name.to_string());
                            }
                        }
                    }

                    let source_id = format!("{}#{}", repo_name, number);
                    let id = KnowledgeArtifact::generate_id("github", &base_url, &source_id);

                    let summary = if body.chars().count() > 300 {
                        Some(body.chars().take(300).collect::<String>() + "...")
                    } else if !body.is_empty() {
                        Some(body.clone())
                    } else {
                        None
                    };

                    let mut relationships = Vec::new();
                    relationships.push(ArtifactRelationship {
                        source_id: repo_name.to_string(),
                        target_id: source_id.clone(),
                        relationship_type: "owns".to_string(),
                    });
                    relationships.extend(Self::extract_text_relationships(&body, &source_id, repo_name));

                    let checksum = KnowledgeArtifact::compute_checksum(&title, summary.as_deref(), &body, &tags);

                    let mut pr_meta = item.clone();
                    if let Some(obj) = pr_meta.as_object_mut() {
                        obj.insert("artifact_type".to_string(), serde_json::json!("pull_request"));
                        obj.insert("repository".to_string(), serde_json::json!(repo_name));
                        obj.insert("provider".to_string(), serde_json::json!("github"));
                        obj.insert("number".to_string(), serde_json::json!(number));
                        obj.insert("title".to_string(), serde_json::json!(title));
                        obj.insert("state".to_string(), serde_json::json!(state));
                        if let Some(author) = item.get("user").and_then(|u| u.get("login")).and_then(|v| v.as_str()) {
                            obj.insert("author".to_string(), serde_json::json!(author));
                        }
                        if let Some(branch) = item.get("head").and_then(|h| h.get("ref")).and_then(|v| v.as_str()) {
                            obj.insert("branch".to_string(), serde_json::json!(branch));
                        }
                    }

                    all_artifacts.push(KnowledgeArtifact {
                        id,
                        kind: ArtifactKind::PullRequest,
                        title,
                        summary,
                        body,
                        provider: "github".to_string(),
                        source_id: source_id.clone(),
                        source_url: html_url.clone(),
                        repository: Some(repo_name.to_string()),
                        tags,
                        relationships,
                        created_at,
                        updated_at,
                        synced_at: Utc::now(),
                        checksum,
                        metadata: pr_meta,
                    });


                    // ---------------------------------------------------------
                    // 3b. Pull Request Reviews (Only for open or recent PRs to avoid HTTP sub-request explosion)
                    // ---------------------------------------------------------
                    if state == "open" || pr_page == 1 {
                        let reviews_url = format!("{}/repos/{}/pulls/{}/reviews", base_url, repo_name, number);
                        if let Ok(r_resp) = self.client.get(&reviews_url).send().await {
                        if r_resp.status().is_success() {
                            if let Ok(reviews) = r_resp.json::<Vec<Value>>().await {
                                for rev in reviews {
                                    let rev_id = rev.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let reviewer = rev.get("user").and_then(|u| u.get("login")).and_then(|v| v.as_str()).unwrap_or("unknown");
                                    let rev_state = rev.get("state").and_then(|v| v.as_str()).unwrap_or("COMMENTED");
                                    let rev_body = rev.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let rev_html_url = rev.get("html_url").and_then(|v| v.as_str()).unwrap_or(&html_url).to_string();

                                    let submitted_at_str = rev.get("submitted_at").and_then(|v| v.as_str()).unwrap_or("");
                                    let rev_updated = DateTime::parse_from_rfc3339(submitted_at_str)
                                        .map(|dt| dt.with_timezone(&Utc))
                                        .unwrap_or(updated_at);

                                    let rev_source_id = format!("{}#{}/reviews/{}", repo_name, number, rev_id);
                                    let rev_id_key = KnowledgeArtifact::generate_id("github", &base_url, &rev_source_id);

                                    let rev_title = format!("Review on {}#{} by {}", repo_name, number, reviewer);
                                    let rev_summary = if rev_body.chars().count() > 200 {
                                        Some(rev_body.chars().take(200).collect::<String>() + "...")
                                    } else if !rev_body.is_empty() {
                                        Some(rev_body.clone())
                                    } else {
                                        Some(format!("State: {}", rev_state))
                                    };

                                    let rev_tags = vec![
                                        format!("repo:{}", repo_name),
                                        format!("state:{}", rev_state.to_lowercase()),
                                        "type:pull_request_review".to_string(),
                                    ];

                                    let rev_rels = vec![ArtifactRelationship {
                                        source_id: rev_source_id.clone(),
                                        target_id: source_id.clone(),
                                        relationship_type: "belongs_to".to_string(),
                                    }];

                                    let rev_checksum = KnowledgeArtifact::compute_checksum(&rev_title, rev_summary.as_deref(), &rev_body, &rev_tags);

                                    all_artifacts.push(KnowledgeArtifact {
                                        id: rev_id_key,
                                        kind: ArtifactKind::PullRequestReview,
                                        title: rev_title,
                                        summary: rev_summary,
                                        body: rev_body,
                                        provider: "github".to_string(),
                                        source_id: rev_source_id,
                                        source_url: rev_html_url,
                                        repository: Some(repo_name.to_string()),
                                        tags: rev_tags,
                                        relationships: rev_rels,
                                        created_at: Some(rev_updated),
                                        updated_at: rev_updated,
                                        synced_at: Utc::now(),
                                        checksum: rev_checksum,
                                        metadata: rev,
                                    });
                                }
                            }
                        }
                    }
                }
                }

                if reached_watermark || len < per_page {
                    break;
                }
                pr_page += 1;
            }

            // -------------------------------------------------------------
            // 4. Review Comments Artifacts
            // -------------------------------------------------------------
            let mut comment_page = 1;
            loop {
                let mut url = format!(
                    "{}/repos/{}/pulls/comments?sort=updated&direction=desc&per_page={}&page={}",
                    base_url, repo_name, per_page, comment_page
                );
                if let Some(since_dt) = since {
                    url.push_str(&format!("&since={}", since_dt.to_rfc3339()));
                }

                let resp = match self.client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let comments: Vec<Value> = match resp.json().await {
                    Ok(c) => c,
                    Err(_) => break,
                };

                if comments.is_empty() {
                    break;
                }
                let len = comments.len();

                for comment in comments {
                    let comment_id = comment.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                    let body = comment.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let path = comment.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let line = comment.get("line").or_else(|| comment.get("original_line")).and_then(|v| v.as_i64()).unwrap_or(0);
                    let author = comment.get("user").and_then(|u| u.get("login")).and_then(|v| v.as_str()).unwrap_or("unknown");
                    let html_url = comment.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let pr_url = comment.get("pull_request_url").and_then(|v| v.as_str()).unwrap_or("");

                    let updated_at_str = comment.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
                    let updated_at = DateTime::parse_from_rfc3339(updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let created_at_str = comment.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                    let created_at = DateTime::parse_from_rfc3339(created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok();

                    let source_id = format!("{}/comments/{}", repo_name, comment_id);
                    let id = KnowledgeArtifact::generate_id("github", &base_url, &source_id);
                    let title = format!("Comment on {}:{} by {}", path, line, author);

                    let summary = if body.chars().count() > 200 {
                        Some(body.chars().take(200).collect::<String>() + "...")
                    } else if !body.is_empty() {
                        Some(body.clone())
                    } else {
                        None
                    };

                    let tags = vec![
                        format!("repo:{}", repo_name),
                        "type:review_comment".to_string(),
                    ];

                    let mut relationships = Vec::new();
                    if let Some(idx) = pr_url.rfind('/') {
                        let pr_num = &pr_url[idx + 1..];
                        if pr_num.chars().all(|c| c.is_ascii_digit()) {
                            relationships.push(ArtifactRelationship {
                                source_id: source_id.clone(),
                                target_id: format!("{}#{}", repo_name, pr_num),
                                relationship_type: "belongs_to".to_string(),
                            });
                        }
                    }

                    let checksum = KnowledgeArtifact::compute_checksum(&title, summary.as_deref(), &body, &tags);

                    all_artifacts.push(KnowledgeArtifact {
                        id,
                        kind: ArtifactKind::ReviewComment,
                        title,
                        summary,
                        body,
                        provider: "github".to_string(),
                        source_id,
                        source_url: html_url,
                        repository: Some(repo_name.to_string()),
                        tags,
                        relationships,
                        created_at,
                        updated_at,
                        synced_at: Utc::now(),
                        checksum,
                        metadata: comment,
                    });
                }

                if len < per_page {
                    break;
                }
                comment_page += 1;
            }

            // -------------------------------------------------------------
            // 5. Commits Artifacts (Metadata Only - NO source code)
            // -------------------------------------------------------------
            let mut commit_page = 1;
            loop {
                let mut url = format!(
                    "{}/repos/{}/commits?per_page={}&page={}",
                    base_url, repo_name, per_page, commit_page
                );
                if let Some(since_dt) = since {
                    url.push_str(&format!("&since={}", since_dt.to_rfc3339()));
                }

                let resp = match self.client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let commits: Vec<Value> = match resp.json().await {
                    Ok(c) => c,
                    Err(_) => break,
                };

                if commits.is_empty() {
                    break;
                }
                let len = commits.len();

                for item in commits {
                    let sha = item.get("sha").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if sha.is_empty() {
                        continue;
                    }

                    let commit_obj = item.get("commit").unwrap_or(&Value::Null);
                    let message = commit_obj.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let headline = message.lines().next().unwrap_or("").to_string();
                    let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let author_date_str = commit_obj
                        .get("author")
                        .and_then(|a| a.get("date"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let commit_time = DateTime::parse_from_rfc3339(author_date_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let source_id = format!("{}@{}", repo_name, sha);
                    let id = KnowledgeArtifact::generate_id("github", &base_url, &source_id);

                    let tags = vec![
                        format!("repo:{}", repo_name),
                        "type:commit".to_string(),
                    ];

                    let mut relationships = Vec::new();
                    relationships.push(ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: repo_name.to_string(),
                        relationship_type: "belongs_to".to_string(),
                    });

                    if let Some(parents) = item.get("parents").and_then(|v| v.as_array()) {
                        for p in parents {
                            if let Some(parent_sha) = p.get("sha").and_then(|v| v.as_str()) {
                                if parent_sha != sha {
                                    relationships.push(ArtifactRelationship {
                                        source_id: source_id.clone(),
                                        target_id: format!("{}@{}", repo_name, parent_sha),
                                        relationship_type: "parent_commit".to_string(),
                                    });
                                }
                            }
                        }
                    }

                    let checksum = KnowledgeArtifact::compute_checksum(&headline, Some(&headline), &message, &tags);

                    all_artifacts.push(KnowledgeArtifact {
                        id,
                        kind: ArtifactKind::Commit,
                        title: headline.clone(),
                        summary: if headline.is_empty() { None } else { Some(headline) },
                        body: message,
                        provider: "github".to_string(),
                        source_id,
                        source_url: html_url,
                        repository: Some(repo_name.to_string()),
                        tags,
                        relationships,
                        created_at: Some(commit_time),
                        updated_at: commit_time,
                        synced_at: Utc::now(),
                        checksum,
                        metadata: item,
                    });
                }

                if len < per_page {
                    break;
                }
                commit_page += 1;
            }

            // -------------------------------------------------------------
            // 6. Releases Artifacts
            // -------------------------------------------------------------
            let mut rel_page = 1;
            loop {
                let url = format!(
                    "{}/repos/{}/releases?per_page={}&page={}",
                    base_url, repo_name, per_page, rel_page
                );

                let resp = match self.client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let releases: Vec<Value> = match resp.json().await {
                    Ok(r) => r,
                    Err(_) => break,
                };

                if releases.is_empty() {
                    break;
                }
                let len = releases.len();

                for item in releases {
                    let tag_name = item.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if tag_name.is_empty() {
                        continue;
                    }

                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or(&tag_name).to_string();
                    let notes = item.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let target_commitish = item.get("target_commitish").and_then(|v| v.as_str()).unwrap_or("");

                    let published_at_str = item.get("published_at").and_then(|v| v.as_str()).unwrap_or("");
                    let published_at = DateTime::parse_from_rfc3339(published_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let source_id = format!("{}/releases/{}", repo_name, tag_name);
                    let id = KnowledgeArtifact::generate_id("github", &base_url, &source_id);

                    let summary = if notes.chars().count() > 300 {
                        Some(notes.chars().take(300).collect::<String>() + "...")
                    } else if !notes.is_empty() {
                        Some(notes.clone())
                    } else {
                        Some(format!("Release tag: {}", tag_name))
                    };

                    let tags = vec![
                        format!("repo:{}", repo_name),
                        format!("tag:{}", tag_name),
                        "type:release".to_string(),
                    ];

                    let mut relationships = Vec::new();
                    relationships.push(ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: repo_name.to_string(),
                        relationship_type: "belongs_to".to_string(),
                    });

                    if !target_commitish.is_empty() {
                        relationships.push(ArtifactRelationship {
                            source_id: source_id.clone(),
                            target_id: format!("{}@{}", repo_name, target_commitish),
                            relationship_type: "contains".to_string(),
                        });
                    }

                    let checksum = KnowledgeArtifact::compute_checksum(&name, summary.as_deref(), &notes, &tags);

                    all_artifacts.push(KnowledgeArtifact {
                        id,
                        kind: ArtifactKind::Release,
                        title: name,
                        summary,
                        body: notes,
                        provider: "github".to_string(),
                        source_id,
                        source_url: html_url,
                        repository: Some(repo_name.to_string()),
                        tags,
                        relationships,
                        created_at: Some(published_at),
                        updated_at: published_at,
                        synced_at: Utc::now(),
                        checksum,
                        metadata: item,
                    });
                }

                if len < per_page {
                    break;
                }
                rel_page += 1;
            }
        }

        Ok(all_artifacts)
    }
}

