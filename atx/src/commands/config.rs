use crate::cli::ConfigSubcommands;
use anyhow::Result;
use atlas_core::{Config, ConnectorConfig};
use std::path::Path;

pub async fn handle(action: ConfigSubcommands, config_path: &Path) -> Result<()> {
            let mut cfg = Config::load_from_path(config_path)?;

            match action {
                ConfigSubcommands::Jira {
                    id,
                    url,
                    email,
                    token,
                    token_env,
                    projects,
                    add_projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_default();
                    let final_email = email.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_projects {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !project_list.contains(&new_p) {
                                project_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "jira".to_string(),
                            instance_url: final_url,
                            email: final_email,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: project_list,
                            spaces: Vec::new(),
                            repos: Vec::new(),
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Jira connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Confluence {
                    id,
                    url,
                    email,
                    token,
                    token_env,
                    spaces,
                    add_spaces,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_default();
                    let final_email = email.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut space_list = if let Some(s) = spaces {
                        s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.spaces.clone()).unwrap_or_default()
                    };

                    if let Some(add_s) = add_spaces {
                        for new_s in add_s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) {
                            if !space_list.contains(&new_s) {
                                space_list.push(new_s);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "confluence".to_string(),
                            instance_url: final_url,
                            email: final_email,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: Vec::new(),
                            spaces: space_list,
                            repos: Vec::new(),
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Confluence connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Github {
                    id,
                    url,
                    token,
                    token_env,
                    repos,
                    add_repos,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.github.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut repo_list = if let Some(r) = repos {
                        r.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.repos.clone()).unwrap_or_default()
                    };

                    if let Some(add_r) = add_repos {
                        for new_r in add_r.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) {
                            if !repo_list.contains(&new_r) {
                                repo_list.push(new_r);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "github".to_string(),
                            instance_url: final_url,
                            email: String::new(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: Vec::new(),
                            spaces: Vec::new(),
                            repos: repo_list,
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("GitHub connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Gitlab {
                    id,
                    url,
                    token,
                    token_env,
                    projects,
                    add_projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://gitlab.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_projects {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !project_list.contains(&new_p) {
                                project_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "gitlab".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("GitLab connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Notion {
                    id,
                    token,
                    token_env,
                    database_ids,
                    page_ids,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let db_list: Vec<String> = database_ids
                        .map(|d| d.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.database_ids.clone()).unwrap_or_default());

                    let page_list: Vec<String> = page_ids
                        .map(|p| p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.page_ids.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "notion".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            database_ids: db_list,
                            page_ids: page_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Notion connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Linear {
                    id,
                    url,
                    token,
                    token_env,
                    sync_comments,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.linear.app/graphql".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_comments = sync_comments.or_else(|| existing.as_ref().and_then(|e| e.sync_comments)).or(Some(true));

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "linear".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            sync_comments: final_comments,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Linear connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Openapi {
                    id,
                    path,
                    paths,
                    add_paths,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "openapi".to_string(),
                            path: final_path,
                            paths: path_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("OpenAPI connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Asana {
                    id,
                    token,
                    token_env,
                    workspace,
                    projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "asana".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Asana connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::AzureDevops {
                    id,
                    url,
                    token,
                    token_env,
                    org,
                    projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://dev.azure.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_org = org.or_else(|| existing.as_ref().and_then(|e| e.organization.clone()));

                    let project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "azure_devops".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            organization: final_org,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Azure DevOps connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Bitbucket {
                    id,
                    url,
                    username,
                    token,
                    token_env,
                    workspace,
                    repos,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.bitbucket.org/2.0".to_string());
                    let final_user = username.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let repo_list = if let Some(r) = repos {
                        r.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.repos.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "bitbucket".to_string(),
                            instance_url: final_url,
                            email: final_user,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            repos: repo_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Bitbucket connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Figma {
                    id,
                    token,
                    token_env,
                    file_keys,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let keys_list: Vec<String> = file_keys
                        .map(|k| k.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.file_keys.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "figma".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            file_keys: keys_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Figma connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Clickup {
                    id,
                    url,
                    token,
                    token_env,
                    workspace,
                    spaces,
                    lists,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.clickup.com/api/v2".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let space_list = spaces
                        .map(|s| s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.spaces.clone()).unwrap_or_default());

                    let list_list = lists
                        .map(|l| l.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.lists.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "clickup".to_string(),
                            instance_url: final_url,
                            email: String::new(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            enabled: Some(true),
                            projects: Vec::new(),
                            spaces: space_list,
                            repos: Vec::new(),
                            lists: list_list,
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("ClickUp connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Markdown {
                    id,
                    path,
                    paths,
                    add_paths,
                    glob_patterns,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    let glob_list = glob_patterns
                        .map(|g| g.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.glob_patterns.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "markdown".to_string(),
                            path: final_path,
                            paths: path_list,
                            glob_patterns: glob_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Markdown connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::LocalGit {
                    id,
                    path,
                    paths,
                    add_paths,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "local_git".to_string(),
                            path: final_path,
                            paths: path_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(config_path)?;
                    println!("Local Git connector '{}' updated successfully!", id);
                }
            }

    Ok(())
}
