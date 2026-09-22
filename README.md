<p align="center">
  <img src="docs/atlas-logo.svg" alt="Atlas Logo" width="120" height="120" />
</p>

# Atlas — Unified Engineering Context Engine

> **One engineering context. Infinite possibilities.**

**Atlas** is an open, local-first engineering context engine. It unifies scattered context across development toolchains (**GitHub**, **Jira**, **Confluence**) into a single, normalized engineering context graph with sub-millisecond local querying via SQLite & FTS5, a CLI (`atx`), a Desktop/Web application, and native **Model Context Protocol (MCP)** integration for AI assistants.

---

## 🌟 Key Features

- 🏗️ **Canonical Engineering Artifact Model (`KnowledgeArtifact`)**: Normalizes vendor-specific data into generic, vendor-neutral engineering artifacts (`Repository`, `Issue`, `PullRequest`, `PullRequestReview`, `ReviewComment`, `Commit`, `Release`, `Ticket`, `Document`).
- 🕸️ **Relationship Graph (`ArtifactRelationship`)**: First-class directed relationship graph (`owns`, `belongs_to`, `contains`, `references`, `parent_commit`) connecting artifacts across toolchains without hardcoded logic.
- 🔌 **14 Comprehensive Connectors**: Native support for **GitHub**, **GitLab**, **ClickUp**, **Linear**, **Notion**, **Swagger/OpenAPI**, **Markdown**, **Jira**, **Confluence**, **Asana**, **Azure DevOps**, **Bitbucket**, **Figma**, and **Local Git**.
- ⚡ **High-Performance Storage & Search (`atlas-core`)**: Built in Rust using SQLite & FTS5 for instant local BM25 full-text search and graph traversals.
- 💻 **Developer CLI (`atx`)**: Query artifacts (`search`, `artifact <id>`, `related <id>`, `repository <repo>`), configure connectors (`config <provider>`), verify connectivity (`connector verify <id>`), check health (`connector doctor`), and run syncs with progress telemetry.
- 🛡️ **Resilience & Health Monitoring**: Automatic circuit breaking, retry budgets, exponential backoff, and P95 latency tracking for every connector.
- 🤖 **Model Context Protocol (MCP) Server**: Stdio MCP tools (`atx_search`, `atx_artifact`, `atx_related`, `atx_status`) enabling AI models (Claude, Cursor, Antigravity, etc.) to consume engineering context directly.

---

## 📦 Quick Installation

You can install and use the Atlas CLI (`atx`) directly without cloning this repository:

### Linux & macOS (One-Line Installer)
```bash
curl -fsSL https://raw.githubusercontent.com/tiofani03/atlas/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/tiofani03/atlas/main/install.ps1 | iex
```

### Via Cargo (For Rust Developers)
```bash
cargo install --git https://github.com/tiofani03/atlas atx
```

### Pre-compiled Binaries
Standalone binaries for Linux (`x86_64`, `aarch64`), macOS (`Apple Silicon`, `Intel`), and Windows (`x64`) are automatically published to [GitHub Releases](https://github.com/tiofani03/atlas/releases).

---

## 📁 Repository Structure

```text
atlas/
├── atlas-core/              # Core engine library (Rust)
│   ├── src/connectors/      # Multi-source connectors
│   │   ├── github.rs        # GitHub connector
│   │   ├── gitlab.rs        # GitLab connector
│   │   ├── clickup.rs       # ClickUp connector
│   │   ├── linear.rs        # Linear connector
│   │   ├── notion.rs        # Notion connector (recursive block markdown)
│   │   ├── openapi.rs       # Swagger / OpenAPI connector (JSON & YAML)
│   │   ├── markdown.rs      # Local Markdown connector
│   │   ├── jira.rs          # Jira ticket connector
│   │   ├── confluence.rs    # Confluence document connector
│   │   ├── asana.rs         # Asana connector
│   │   ├── azure_devops.rs  # Azure DevOps connector
│   │   ├── bitbucket.rs     # Bitbucket connector
│   │   ├── figma.rs         # Figma connector
│   │   └── local_git.rs     # Local Git repository connector
│   ├── src/domain.rs        # Normalized domain models (KnowledgeArtifact, ArtifactKind)
│   ├── src/resilience/      # Retry policy, circuit breaker, bulkhead
│   ├── src/health/          # Health reports and scoring engine
│   ├── src/progress/        # Sync progress event bus & renderers
│   ├── src/storage.rs       # SQLite storage & graph database + FTS5 index
│   ├── src/sync.rs          # Incremental sync engine
│   └── src/mcp.rs           # Model Context Protocol stdio server
├── atx/                     # CLI binary tool (`atx`)
│   └── src/main.rs          # CLI entry point
├── atlas-desktop/           # Desktop & Web Application
│   ├── backend/             # Axum REST API server (Rust)
│   └── frontend/            # React 19 UI (TypeScript, Vite, TailwindCSS v4)
├── Cargo.toml               # Workspace manifest
└── README.md                # Project documentation
```

---

## 🛠️ Requirements

- **Rust**: `1.75+` (with `cargo`)
- **Node.js**: `v20+` (and `npm` or `pnpm`)

---

## 🚀 Getting Started

### Build the Rust Workspace

Clone the repository and compile the workspace binaries:

```bash
# Build debug binaries
cargo build

# Or build optimized release binaries
cargo build --release
```

---

## 🌐 Launching the Web UI & Interactive Graph

Atlas comes with a fully embedded, zero-dependency Web Application featuring an **Interactive Graph Visualizer**, **Knowledge Explorer**, **Live Connector Settings**, and **MCP Hub**.

```bash
# Launch the Web UI and automatically open your default browser
atx ui

# Or use the shortcut alias
atx web

# Specify a custom port or run headless (no browser auto-open)
atx ui --port 8080
atx ui --no-open
```

The Web UI runs on **`http://localhost:31415`** by default.

### Run, Stop, Restart, and Check Status

`atx ui` runs the Axum backend and the embedded web UI together. The commands below use the binary built from the current checkout, so they include the latest local changes:

```bash
# Build the current binary
make desktop-build

# Update the `atx` command installed in PATH
make install-cli

# Run in the foreground (Ctrl+C to stop)
make desktop

# Restart in the background
make desktop-restart

# Check that the backend and web UI are up
make desktop-status

# Stop the service
make desktop-stop

# View detached-process logs
tail -f /tmp/atlas-ui.log
```

The detached service is available at **`http://localhost:31415`**. The separate Vite development server is started by `./dev.sh` and uses port `31420`.

### 🌟 Key Web UI Features:
- 🕸️ **Interactive Graph Visualizer**: Visualize relationships across PRs, Commits, Jira Tickets, and Figma Specs with Dagre auto-layout, depth expansion, and metadata drawer.
- 📑 **Knowledge Explorer**: Seamlessly switch between `Table` and `Graph` views to filter, search, and inspect engineering artifacts.
- 🔌 **Live Connector Management**: Configure and validate 14+ connectors (GitHub, Jira, Confluence, Local Git, Notion, OpenAPI, etc.) with real-time path/credential checks.
- 🤖 **MCP Hub Visualizer**: Configure, monitor, and test upstream Model Context Protocol tools.

---

## 💻 CLI Usage (`atx`)

### Launch Web UI
```bash
atx ui            # Open Web UI in browser
atx web           # Alias for 'atx ui'
```

### Initialize Atlas Context Engine
Create local configuration (`~/.config/atlas/config.toml`) and database storage:
```bash
atx init
```

### Configure Connectors

All connectors can be configured using `atx config <provider>`:

```bash
# Configure GitHub
atx config github github-main --token-env GITHUB_TOKEN --repos "owner/repo1,owner/repo2"

# Configure Local Git Repository
atx config local-git my-repo --path /path/to/repo

# Configure Jira & Confluence
atx config jira jira-main --url https://company.atlassian.net --email user@example.com --token-env JIRA_API_TOKEN --projects "PAY,DEV"
atx config confluence conf-main --url https://company.atlassian.net --email user@example.com --token-env CONFLUENCE_API_TOKEN --spaces "ENG,ARCH"

# Configure GitLab
atx config gitlab gitlab-main --url https://gitlab.com --token-env GITLAB_TOKEN --repos "owner/repo"

# Configure ClickUp, Linear, Notion
atx config clickup clickup-main --token-env CLICKUP_TOKEN --workspace "123456"
atx config linear linear-main --token-env LINEAR_API_KEY
atx config notion notion-main --token-env NOTION_TOKEN

# Configure Swagger / OpenAPI & Local Markdown Documentation
atx config openapi api-spec --path ./openapi.yaml
atx config markdown local-docs --path ./docs
```

### Verify Connectors & Check Health

```bash
# Test live connectivity for a connector
atx connector verify github-main
atx connector verify my-repo

# View connector health monitoring report & P95 latency
atx connector doctor

# Comprehensive system diagnostic (SQLite WAL, integrity, disk size)
atx doctor
```

### Synchronize Knowledge

```bash
# Sync all connectors
atx sync

# Sync a specific connector
atx sync --connector github-main

# Force a full resync ignoring watermarks
atx sync --full
```

### Query Context Graph & Search

**Search Context Graph:**
```bash
# Full-text BM25 search
atx search "payment API"

# Filter by artifact kind, tag, or repository
atx search --kind pull_request --repo "owner/repo"

# Output as JSON
atx search "auth" --json
```

**Inspect Artifact Details:**
```bash
atx artifact owner/repo#42
atx artifact PROJ-123
```

**Traverse Related Artifact Graph:**
```bash
atx related owner/repo#42
```

**AI Execution Briefing (`atx context`):**
```bash
# Build token-optimized AI context briefing with 2-hop graph
atx context PROJ-123 --depth 2

# Include Figma design tokens and layout context
atx context PROJ-123 --figma "https://www.figma.com/file/sample-figma-key"

# Profile retrieval latency breakdown across stages
atx context PROJ-123 --profile
```

**Deep-Dive Graph Lineage Explanation (`atx explain`):**
```bash
# Explain relationships and commit ancestry for an artifact
atx explain owner/repo#42 --all

# Focus on a specific subsystem
atx explain owner/repo#42 --subsystem atlas-core
```

**Browse & Read Indexed Documentation (`atx docs`, `atx doc`, `atx cat`):**
```bash
# List all indexed documentation (Markdown, Confluence, Notion, ADRs)
atx docs

# Filter documents by keyword
atx docs --query "architecture"

# View document content formatted in terminal
atx doc "Architecture Overview"

# Stream raw document markdown (piping to glow or bat)
atx doc "Architecture Overview" --raw | glow

# Print full content of any artifact (ticket, PR, or doc)
atx cat PROJ-123
```

### Storage & Graph Status
Check context graph statistics and database size:
```bash
atx status
```

---

## 🔒 Source Code Separation & Future Connectors

- **GitHub Connector Scope**: Ingests collaboration and process metadata (repositories, issues, pull requests, reviews, review comments, commit metadata, releases). It **does not store source code or file diffs**.
- **Future Git Connector**: A dedicated local Git connector will index source files, AST symbol definitions (via Tree-sitter), functions, classes, and call graphs.

---

## 🤖 AI Assistant Integration (MCP Server)

Atlas includes a native **Stdio MCP Server** allowing AI models to retrieve engineering context dynamically.

```json
{
  "mcpServers": {
    "atlas": {
      "command": "/path/to/atlas/target/release/atx",
      "args": ["mcp"]
    }
  }
}
```

### MCP Hub & Figma Clone Resolution

Atlas can expose configured upstream MCP servers through one gateway:

```bash
# Register Figma MCP once
atx mcp add figma --command npx --args "-y,mcp-figma" --prefix figma --env FIGMA_PERSONAL_ACCESS_TOKEN=$FIGMA_PERSONAL_ACCESS_TOKEN

# Inspect, test, or start the aggregated gateway
atx mcp list
atx mcp test figma
atx mcp
```

For a project-specific working clone, create `.atlas/figma.toml`:

```toml
[figma]
file_key = "sample_figma_file_key"
node_id = "1:2"

# Optional ticket-to-clone overrides
[aliases]
"PROJ-123" = "sample_figma_file_key"
```

Then `atx context PROJ-123` and MCP `atx_context` include the resolved Figma design metadata. Figma file candidates already indexed by Atlas or cached by the Figma MCP are also considered when no explicit override exists.

---

## 🌐 REST API Reference

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/api/status` | Retrieve local storage statistics and connector statuses |
| `GET` | `/api/connectors` | List all configured connectors |
| `POST` | `/api/connectors/github` | Save or update GitHub connector configuration |
| `POST` | `/api/connectors/jira` | Save or update Jira connector configuration |
| `POST` | `/api/connectors/confluence` | Save or update Confluence connector configuration |
| `POST` | `/api/connectors/local_git` | Save or update Local Git repository connector configuration |
| `POST` | `/api/connectors/validate` | Real-time live verification of connector paths or credentials |
| `POST` | `/api/sync` | Trigger background sync engine |
| `GET` | `/api/search` | Search indexed artifacts with query params (`query`, `kind`, `tag`, `repository`) |
| `GET` | `/api/objects/:id` | Get details for a specific canonical artifact |
| `GET` | `/api/graph/:id` | Fetch subgraph topology around an artifact (`?depth=1|2&limit=150`) |
| `GET` | `/api/graph/recent` | List recently indexed seed artifacts for graph exploration |

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more details.
