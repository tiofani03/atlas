# Atlas — Unified Engineering Context Engine

> **One engineering context. Infinite possibilities.**

**Atlas** is an open, local-first engineering context engine. It unifies scattered context across development toolchains (**GitHub**, **Jira**, **Confluence**) into a single, normalized engineering context graph with sub-millisecond local querying via SQLite & FTS5, a CLI (`atx`), a Desktop/Web application, and native **Model Context Protocol (MCP)** integration for AI assistants.

---

## 🌟 Key Features

- 🏗️ **Canonical Engineering Artifact Model (`KnowledgeArtifact`)**: Normalizes vendor-specific data into generic, vendor-neutral engineering artifacts (`Repository`, `Issue`, `PullRequest`, `PullRequestReview`, `ReviewComment`, `Commit`, `Release`, `Ticket`, `Document`).
- 🕸️ **Relationship Graph (`ArtifactRelationship`)**: First-class directed relationship graph (`owns`, `belongs_to`, `contains`, `references`, `parent_commit`) connecting artifacts across toolchains without hardcoded logic.
- 🐙 **Expanded GitHub Connector**: Synchronizes collaboration metadata across repositories, issues, pull requests, reviews, review comments, commits (metadata only—no source code), and releases with watermark incremental syncing.
- ⚡ **High-Performance Storage & Search (`atlas-core`)**: Built in Rust using SQLite & FTS5 for instant local BM25 full-text search and graph traversals.
- 💻 **Developer CLI (`atx`)**: Query artifacts (`search`, `artifact <id>`, `related <id>`, `repository <repo>`), configure connectors, trigger background syncs, and check context graph status.
- 🤖 **Model Context Protocol (MCP) Server**: Stdio MCP tools (`atx_search`, `atx_artifact`, `atx_related`, `atx_status`) enabling AI models (Claude, Cursor, Antigravity, etc.) to consume engineering context directly.

---

## 📁 Repository Structure

```text
atlas/
├── atlas-core/              # Core engine library (Rust)
│   ├── src/connectors/      # Multi-source connectors (GitHub, Jira, Confluence)
│   │   ├── github.rs        # GitHub connector (Repo, Issue, PR, Review, Comment, Commit, Release)
│   │   ├── jira.rs          # Jira ticket connector
│   │   └── confluence.rs    # Confluence document connector
│   ├── src/domain.rs        # Normalized domain models (KnowledgeArtifact, ArtifactKind, ArtifactRelationship)
│   ├── src/storage.rs       # SQLite storage & graph database + FTS5 index
│   ├── src/sync.rs          # Incremental sync engine
│   └── src/mcp.rs           # Model Context Protocol stdio server
├── atx/                     # CLI binary tool (`atx`)
│   └── src/main.rs          # CLI entry point (search, artifact, related, repository, sync, status)
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

## 💻 CLI Usage (`atx`)

### Initialize Atlas Context Engine
Create local configuration (`~/.config/atlas/config.toml`) and database storage:
```bash
cargo run --bin atx -- init
```

### Configure Connectors

**Configure GitHub:**
```bash
cargo run --bin atx -- config github --token-env GITHUB_TOKEN --repos "owner/repo1,owner/repo2"
```

**Configure Jira:**
```bash
cargo run --bin atx -- config jira --url https://company.atlassian.net --email user@example.com --token-env JIRA_API_TOKEN --projects "PAY,DEV"
```

**Configure Confluence:**
```bash
cargo run --bin atx -- config confluence --url https://company.atlassian.net --email user@example.com --token-env CONFLUENCE_API_TOKEN --spaces "ENG,ARCH"
```

### Synchronize Knowledge
Sync configured connectors into local SQLite storage:
```bash
# Sync all connectors
cargo run --bin atx -- sync

# Sync a specific connector
cargo run --bin atx -- sync --connector github-main

# Force a full resync
cargo run --bin atx -- sync --full
```

### Query Context Graph & Search

**Search Context Graph:**
```bash
# Full-text search
cargo run --bin atx -- search "payment API"

# Filter by artifact kind, tag, or repository
cargo run --bin atx -- search --kind pull_request --repo "owner/repo"

# Output as JSON
cargo run --bin atx -- search "auth" --json
```

**Inspect Artifact Details:**
```bash
cargo run --bin atx -- artifact owner/repo#42
```

**Traverse Related Artifact Graph:**
```bash
cargo run --bin atx -- related owner/repo#42
```

**List Repository Artifacts:**
```bash
cargo run --bin atx -- repository owner/repo
```

### Storage & Graph Status
Check context graph statistics:
```bash
cargo run --bin atx -- status
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

---

## 🌐 REST API Reference

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/api/status` | Retrieve local storage statistics and connector statuses |
| `GET` | `/api/connectors` | List all configured connectors |
| `POST` | `/api/connectors/github` | Save or update GitHub connector configuration |
| `POST` | `/api/connectors/jira` | Save or update Jira connector configuration |
| `POST` | `/api/connectors/confluence` | Save or update Confluence connector configuration |
| `POST` | `/api/sync` | Trigger background sync engine |
| `GET` | `/api/search` | Search indexed artifacts with query params (`query`, `kind`, `tag`, `repository`) |
| `GET` | `/api/objects/:id` | Get details for a specific canonical artifact |

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more details.

