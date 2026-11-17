use crate::cli::{parse_mcp_args, parse_mcp_env, McpSubcommands};
use crate::formatter;
use anyhow::Result;
use atlas_core::{Config, McpClient, McpHub, McpServerConfig, Storage, run_stdio_mcp_server};
use std::path::Path;

pub async fn handle(action: Option<McpSubcommands>, config_path: &Path) -> Result<()> {
            let action = action.unwrap_or(McpSubcommands::Serve);
            match action {
                McpSubcommands::Serve => {
                    let cfg = Config::load_from_path(config_path)?;
                    let storage = Storage::new(cfg.resolve_db_path())?;
                    let hub = McpHub::from_config(&cfg, storage)
                        .await?
                        .with_config_path(config_path.to_path_buf());
                    run_stdio_mcp_server(hub).await?;
                }

                McpSubcommands::List => {
                    let cfg = Config::load_from_path(config_path)?;
                    let mut servers: Vec<_> = cfg.mcp_servers.iter().collect();
                    servers.sort_by_key(|(k, _)| *k);

                    println!("┌─{0:─<20}─┬─{0:─<40}─┬─{0:─<15}─┬─{0:─<10}─┐", "");
                    println!("│ {:<20} │ {:<40} │ {:<15} │ {:<10} │", "MCP SERVER", "COMMAND", "PREFIX", "STATE");
                    println!("├─{0:─<20}─┼─{0:─<40}─┼─{0:─<15}─┼─{0:─<10}─┤", "");

                    if servers.is_empty() {
                        println!("│ {:<94} │", "(No MCP servers configured in config.toml)");
                    } else {
                        for (name, server_cfg) in servers {
                            let cmd_display = if server_cfg.args.is_empty() {
                                server_cfg.command.clone()
                            } else {
                                format!("{} {}", server_cfg.command, server_cfg.args.join(" "))
                            };
                            let prefix_display = server_cfg.prefix.as_deref().unwrap_or("-");
                            let state_display = if server_cfg.enabled.unwrap_or(true) {
                                "enabled"
                            } else {
                                "disabled"
                            };
                            println!(
                                "│ {:<20} │ {:<40} │ {:<15} │ {:<10} │",
                                formatter::safe_truncate(name, 20),
                                formatter::safe_truncate(&cmd_display, 40),
                                formatter::safe_truncate(prefix_display, 15),
                                state_display
                            );
                        }
                    }
                    println!("└─{0:─<20}─┴─{0:─<40}─┴─{0:─<15}─┴─{0:─<10}─┘", "");
                }

                McpSubcommands::Add {
                    name,
                    command,
                    args,
                    env,
                    prefix,
                    disabled,
                } => {
                    let mut cfg = Config::load_from_path(config_path)?;
                    let existing = cfg.mcp_servers.get(&name).cloned();

                    let parsed_args = parse_mcp_args(&args);
                    let final_args = if !args.is_empty() {
                        parsed_args
                    } else if let Some(existing_cfg) = &existing {
                        existing_cfg.args.clone()
                    } else {
                        Vec::new()
                    };

                    let mut env_map = if let Some(existing_cfg) = &existing {
                        existing_cfg.env.clone()
                    } else {
                        std::collections::HashMap::new()
                    };
                    for (k, v) in parse_mcp_env(&env) {
                        env_map.insert(k, v);
                    }

                    let final_prefix = if let Some(p) = prefix {
                        if p.is_empty() {
                            None
                        } else {
                            Some(p)
                        }
                    } else if let Some(existing_cfg) = &existing {
                        existing_cfg.prefix.clone()
                    } else {
                        None
                    };

                    let final_enabled = if disabled {
                        Some(false)
                    } else {
                        Some(true)
                    };

                    let existing_aliases = cfg
                        .mcp_servers
                        .get(&name)
                        .map(|s| s.aliases.clone())
                        .unwrap_or_default();

                    let new_server = McpServerConfig {
                        command: command.clone(),
                        args: final_args.clone(),
                        env: env_map,
                        enabled: final_enabled,
                        prefix: final_prefix.clone(),
                        aliases: existing_aliases,
                    };

                    cfg.mcp_servers.insert(name.clone(), new_server);
                    cfg.save_to_path(config_path)?;

                    println!("MCP server '{}' configured successfully!", name);
                    println!("  Command: {} {}", command, final_args.join(" "));
                    if let Some(ref p) = final_prefix {
                        println!("  Prefix:  {}", p);
                    }
                    if disabled {
                        println!("  State:   disabled");
                    } else {
                        println!("  State:   enabled");
                    }
                }

                McpSubcommands::Test { name } => {
                    let cfg = Config::load_from_path(config_path)?;
                    let server_cfg = match cfg.mcp_servers.get(&name) {
                        Some(c) => c,
                        None => {
                            eprintln!("Error: MCP server '{}' not found in configuration.", name);
                            anyhow::bail!("MCP server '{}' not found in config", name);
                        }
                    };

                    println!("Testing connectivity to MCP server '{}'...", name);
                    println!("  Command: {} {}", server_cfg.command, server_cfg.args.join(" "));
                    if let Some(ref pfx) = server_cfg.prefix {
                        println!("  Prefix:  {}", pfx);
                    }

                    let mut client = match McpClient::start(&name, server_cfg).await {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("✗ Failed to spawn MCP server '{}': {:#}", name, e);
                            anyhow::bail!("MCP server test failed: {}", e);
                        }
                    };

                    let init_result = match client.initialize().await {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("✗ Failed initialization handshake with MCP server '{}': {:#}", name, e);
                            let _ = client.close().await;
                            anyhow::bail!("MCP handshake failed: {}", e);
                        }
                    };

                    println!("✓ Successfully connected and initialized MCP server '{}'!", name);
                    if let Some(server_info) = init_result.get("serverInfo") {
                        let sname = server_info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let sversion = server_info.get("version").and_then(|v| v.as_str()).unwrap_or("");
                        if !sversion.is_empty() {
                            println!("  Remote server: {} (v{})", sname, sversion);
                        } else {
                            println!("  Remote server: {}", sname);
                        }
                    }

                    let tools = match client.list_tools().await {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("✗ Failed to list tools from MCP server '{}': {:#}", name, e);
                            let _ = client.close().await;
                            anyhow::bail!("Failed to list tools: {}", e);
                        }
                    };

                    let _ = client.close().await;

                    println!("\nDiscovered {} tool(s):", tools.len());
                    if tools.is_empty() {
                        println!("  (No tools registered by this server)");
                    } else {
                        println!("┌─{0:─<30}─┬─{0:─<54}─┐", "");
                        println!("│ {:<30} │ {:<54} │", "TOOL NAME", "DESCRIPTION");
                        println!("├─{0:─<30}─┼─{0:─<54}─┤", "");
                        for tool in &tools {
                            let tool_name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let desc = tool.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            let single_line_desc = desc.lines().next().unwrap_or("").trim();
                            println!(
                                "│ {:<30} │ {:<54} │",
                                formatter::safe_truncate(tool_name, 30),
                                formatter::safe_truncate(single_line_desc, 54),
                            );
                        }
                        println!("└─{0:─<30}─┴─{0:─<54}─┘", "");
                        if let Some(ref pfx) = server_cfg.prefix {
                            println!("\nNote: In the Atlas MCP Hub, these tools will be exposed with prefix '{}_'.", pfx);
                        }
                    }
                }

                McpSubcommands::Remove { name } => {
                    let mut cfg = Config::load_from_path(config_path)?;
                    if cfg.mcp_servers.remove(&name).is_some() {
                        cfg.save_to_path(config_path)?;
                        println!("MCP server '{}' removed successfully!", name);
                    } else {
                        println!("MCP server '{}' not found in configuration.", name);
                    }
                }

                McpSubcommands::Alias { server, from, to } => {
                    let mut cfg = Config::load_from_path(config_path)?;
                    let server_cfg = match cfg.mcp_servers.get_mut(&server) {
                        Some(c) => c,
                        None => {
                            eprintln!("Error: MCP server '{}' not found in configuration.", server);
                            anyhow::bail!("MCP server '{}' not found in config", server);
                        }
                    };

                    let (clean_to, node_opt) = atlas_core::mcp::resolver::parse_figma_target(&to);
                    let final_target = if let Some(node) = node_opt {
                        format!("{}:{}", clean_to, node)
                    } else {
                        clean_to
                    };

                    server_cfg.aliases.insert(from.clone(), final_target.clone());
                    cfg.save_to_path(config_path)?;

                    println!("✓ Alias mapped for MCP server '{}':", server);
                    println!("  '{}' -> '{}'", from, final_target);
                }
            }

    Ok(())
}
