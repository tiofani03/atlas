# Atlas — Unified Engineering Knowledge Engine

**Atlas** is a high-performance, developer-first engineering knowledge platform. It unifies scattered context across development toolchains (such as **Jira** and **Confluence**) into a single, structured local knowledge base with fast BM25 full-text search, a powerful CLI (`atx`), a modern Desktop/Web application, and native **Model Context Protocol (MCP)** integration for AI assistants.

---

## 🌟 Key Features

- 🔄 **Multi-Source Connectors**: Seamlessly ingest tickets, documentation, requirements, and specifications from **Jira** and **Confluence** with incremental sync support.
- ⚡ **High-Performance Core (`atlas-core`)**: Built in Rust using SQLite & FTS (BM25 search algorithm) for sub-millisecond local querying.
- 💻 **Developer CLI (`atx`)**: Direct, command-line access to initialize databases, configure credentials, trigger background syncs, search objects, and inspect workspace health.
- 🖥️ **Modern Desktop / Web App (`atlas-desktop`)**:
  - **Backend**: Lightweight Axum REST API server providing standard endpoints.
  - **Frontend**: Responsive React 19 + TypeScript + Vite + TailwindCSS app featuring dynamic dashboards, knowledge explorers, connector managers, AI chat, and artifact viewers.
- 🤖 **Model Context Protocol (MCP) Server**: Exposes stdio MCP tools (`search_knowledge`, `get_knowledge_object`, `sync_knowledge`) enabling AI agents (e.g. Claude Desktop, Cursor, Antigravity) to query your team's knowledge directly.

---

## 📁 Repository Structure

```text
atlas/
├── atlas-core/              # Core engine library (Rust)
│   ├── src/connectors/      # Connectors (Jira, Confluence)
│   ├── src/domain.rs        # Core data models (KnowledgeObject, ObjectType, etc.)
│   ├── src/storage.rs       # SQLite storage & FTS index management
│   ├── src/sync.rs          # Incremental sync engine
│   └── src/mcp.rs           # Model Context Protocol stdio server
├── atx/                     # CLI binary tool (`atx`)
│   └── src/main.rs          # CLI entry point and command subcommands
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

### 1. Build the Rust Workspace

Clone the repository and compile the workspace binaries:

```bash
# Build debug binaries
cargo build

# Or build optimized release binaries
cargo build --release
```

After building, the binaries will be available in `target/release/` (`atx` and `atlas-desktop-backend`).

---

## 💻 CLI Usage (`atx`)

The `atx` CLI is the primary command-line driver for Atlas.

### Initialize Atlas Directory
Create local configuration (`~/.config/atlas/config.toml`) and database storage:
```bash
cargo run --bin atx -- init
```

### Configure Connectors

**Configure Jira:**
```bash
cargo run --bin atx -- config jira --url https://your-company.atlassian.net --email user@example.com --token-env JIRA_API_TOKEN --projects "PAY,DEV"
```

**Configure Confluence:**
```bash
cargo run --bin atx -- config confluence --url https://your-company.atlassian.net --email user@example.com --token-env CONFLUENCE_API_TOKEN --spaces "ENG,ARCH"
```

### Synchronize Knowledge
Sync configured connectors into local SQLite storage:
```bash
# Sync all connectors
cargo run --bin atx -- sync

# Sync a specific connector
cargo run --bin atx -- sync --connector jira-main

# Force a full resync
cargo run --bin atx -- sync --full
```

### Search Knowledge Base
Perform BM25 full-text searches across tickets and documentation:
```bash
# Text query search
cargo run --bin atx -- search "authentication JWT"

# Filter by object type or tag
cargo run --bin atx -- search --object-type ticket --tag API

# Output as JSON
cargo run --bin atx -- search "database schema" --json
```

### Storage Status
Check system stats and connector health:
```bash
cargo run --bin atx -- status
```

---

## 🖥️ Running Desktop Application

### 1. Start Desktop Backend (REST API)

```bash
cargo run --bin atlas-desktop-backend
```
The REST server will run on `http://127.0.0.1:3001`.

### 2. Start Desktop Frontend (Vite UI)

```bash
cd atlas-desktop/frontend
npm install
npm run dev
```
Open your browser at `http://localhost:5173`.

---

## 🤖 AI Assistant Integration (MCP Server)

Atlas includes a native **Stdio MCP Server** allowing AI models to retrieve context dynamically.

### Run MCP Server Manually
```bash
cargo run --bin atx -- mcp
```

### Claude Desktop Configuration Example
Add the following snippet to your `claude_desktop_config.json`:

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
| `POST` | `/api/connectors/jira` | Save or update Jira connector configuration |
| `POST` | `/api/connectors/confluence` | Save or update Confluence connector configuration |
| `POST` | `/api/connectors/validate` | Test connectivity & credentials |
| `POST` | `/api/sync` | Trigger background sync engine |
| `GET` | `/api/sync/status` | Check current sync operation status |
| `GET` | `/api/search` | Search indexed objects with query params (`q`, `type`, `tag`) |
| `GET` | `/api/objects/:id` | Get details for a specific knowledge object |

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more details.
