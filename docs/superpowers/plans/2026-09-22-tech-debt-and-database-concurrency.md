# Tech Debt & Database Concurrency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate dead legacy code, fix context calculation logic bugs, eliminate all clippy compiler warnings, resolve code duplication in desktop sync, and prevent SQLite concurrency lockouts via `busy_timeout` with automated concurrency tests.

**Architecture:** Purge obsolete non-workspace root `src/` files. In `atlas-core`, configure SQLite WAL connections with `PRAGMA busy_timeout = 5000;` and verify with concurrent multi-thread read/write tests. Correct hypothesis impact calculations in `context.rs`, resolve all 42 clippy warnings with idiomatic Rust patterns, and reuse canonical `ConnectorInstance::build` in `atlas-desktop/backend`.

**Tech Stack:** Rust 1.75+, rusqlite (WAL mode), Tokio, cargo clippy, cargo test.

**Spec:** In-chat bounded design approved by user on 2026-09-22.

## Global Constraints

- Must maintain documentation integrity and preserve unrelated existing code/comments.
- Workspace tests (`cargo test --workspace`) must pass 100%.
- Workspace clippy (`cargo clippy --workspace -- -D warnings`) must compile cleanly with 0 warnings.
- Frontend build (`npm run build` in `atlas-desktop/frontend`) must continue to succeed.

---

### Task 1: Purge Legacy Unused Root `src/` Directory

**Files:**
- Delete: `src/config.rs`, `src/domain.rs`, `src/main.rs`, `src/mcp.rs`, `src/storage.rs`, `src/sync.rs`, `src/connectors/mod.rs`, `src/connectors/jira.rs`, `src/connectors/confluence.rs`

**Interfaces:**
- Consumes: None (root `src/` is not a workspace member of `Cargo.toml`).
- Produces: Clean repository tree without zombie legacy code.

- [ ] **Step 1: Check git status of root `src/`**
```bash
git status -- src/
```

- [ ] **Step 2: Remove root `src/` from git tracking**
```bash
git rm -r src/
```

- [ ] **Step 3: Verify workspace build and tests still pass**
```bash
cargo check --workspace
```

- [ ] **Step 4: Commit removal of legacy root files**
```bash
git commit -m "chore: remove obsolete pre-workspace root src directory"
```

---

### Task 2: Add SQLite Busy Timeout and Concurrent Read/Write Regression Test

**Files:**
- Modify: `atlas-core/src/storage.rs:47-56`
- Modify: `atlas-core/tests/storage_tests.rs`

**Interfaces:**
- Consumes: `Storage::get_connection()`
- Produces: SQLite connections configured with `busy_timeout = 5000` to avoid immediate `SQLITE_BUSY` errors during concurrent access.

- [ ] **Step 1: Write concurrent read/write test in `atlas-core/tests/storage_tests.rs`**
Add `test_concurrent_reads_during_batch_write` that executes a batch insert in one thread while multiple background threads read from the database concurrently.

- [ ] **Step 2: Add `PRAGMA busy_timeout = 5000;` to `atlas-core/src/storage.rs`**
Update `get_connection()`:
```rust
conn.execute_batch(
    "PRAGMA journal_mode = WAL;
     PRAGMA foreign_keys = ON;
     PRAGMA synchronous = NORMAL;
     PRAGMA busy_timeout = 5000;",
)?;
```

- [ ] **Step 3: Run storage tests to verify passing**
```bash
cargo test -p atlas-core --test storage_tests
```

- [ ] **Step 4: Commit SQLite busy_timeout changes**
```bash
git add atlas-core/src/storage.rs atlas-core/tests/storage_tests.rs
git commit -m "fix(storage): set PRAGMA busy_timeout to 5000ms and add concurrency test"
```

---

### Task 3: Fix Copy-Paste Logic Bug in Context Engine

**Files:**
- Modify: `atlas-core/src/context.rs:2204-2211` and `atlas-core/src/context.rs:2451-2456`

**Interfaces:**
- Consumes: `ImplementationHypothesis` builder in `atlas-core/src/context.rs`
- Produces: Correct differentiation between `"Low"`, `"Medium"`, and `"High"` implementation impact levels.

- [ ] **Step 1: Check existing context builder tests**
```bash
cargo test -p atlas-core --test context_builder_tests
```

- [ ] **Step 2: Correct logic in `atlas-core/src/context.rs`**
Update line ~2204:
```rust
let impact = if !repos.is_empty() && (!prs.is_empty() || !adrs.is_empty()) {
    "Low"
} else if !repos.is_empty() {
    "Medium"
} else {
    "High"
};
```
Update line ~2451 with matching consistent logic:
```rust
let impact = if !repos.is_empty() && (!adrs.is_empty() || !apis.is_empty()) {
    "Low"
} else if !repos.is_empty() {
    "Medium"
} else {
    "High"
};
```

- [ ] **Step 3: Run context builder tests to verify passing**
```bash
cargo test -p atlas-core --test context_builder_tests
```

- [ ] **Step 4: Commit logic fix**
```bash
git add atlas-core/src/context.rs
git commit -m "fix(context): fix duplicate identical if branches for impact calculation"
```

---

### Task 4: Fix All Clippy Lints and Refactor Function Argument Signatures

**Files:**
- Modify: `atlas-core/src/storage.rs`
- Modify: `atlas-core/src/mcp/hub.rs`
- Modify: `atlas-core/src/context.rs`

**Interfaces:**
- Consumes: Internal helper functions in `atlas-core`
- Produces: Clean compilation under `cargo clippy --workspace -- -D warnings`.

- [ ] **Step 1: Fix `unnecessary_sort_by` in `atlas-core/src/mcp/hub.rs:312`**
Change:
```rust
sorted_servers.sort_by_key(|b| std::cmp::Reverse(b.prefix.len()));
```

- [ ] **Step 2: Fix `manual_flatten` and `unnecessary_map_or` in `atlas-core/src/storage.rs`**
- Replace `for r in rows { if let Ok(art) = r { results.push(art); } }` with `for art in rows.flatten() { results.push(art); }`.
- Simplify `map_or(true, ...)` with `is_none_or(...)` or explicit pattern match.
- Add `#[allow(clippy::too_many_arguments)]` or bundle parameters for `search_fts_paginated` and `search_like_fallback` to satisfy clippy while preserving public API compatibility.

- [ ] **Step 3: Fix `too_many_arguments` in `atlas-core/src/context.rs:2474`**
Bundle or annotate `build_dependency_aware_queue`.

- [ ] **Step 4: Run clippy with `-D warnings` across entire workspace**
```bash
cargo clippy --workspace -- -D warnings
```
Ensure exit code 0 and 0 warnings.

- [ ] **Step 5: Run full test suite**
```bash
cargo test --workspace
```

- [ ] **Step 6: Commit clippy fixes**
```bash
git add atlas-core/src/
git commit -m "chore: fix all clippy warnings and satisfy -D warnings across workspace"
```

---

### Task 5: Eliminate Duplicate Connector Match in Desktop Backend Sync

**Files:**
- Modify: `atlas-desktop/backend/src/handlers/sync.rs:77-100`

**Interfaces:**
- Consumes: `ConnectorInstance::build(id, &connector_cfg)` from `atlas-core`
- Produces: DRY connector instantiation in `atlas-desktop-backend`.

- [ ] **Step 1: Inspect `atlas-desktop/backend/src/handlers/sync.rs`**
Verify the manual `match connector_cfg.provider.as_str()` block.

- [ ] **Step 2: Replace manual match with `ConnectorInstance::build`**
```rust
let conn_instance = match ConnectorInstance::build(&id, &connector_cfg) {
    Ok(ci) => ci,
    Err(e) => {
        tracing::warn!("Failed to build connector {}: {}", id, e);
        continue;
    }
};
```
Clean up unnecessary individual connector imports in `atlas-desktop/backend/src/handlers/sync.rs`.

- [ ] **Step 3: Run backend unit tests**
```bash
cargo test -p atlas-desktop-backend
```

- [ ] **Step 4: Commit desktop backend sync refactor**
```bash
git add atlas-desktop/backend/src/handlers/sync.rs
git commit -m "refactor(backend): use canonical ConnectorInstance::build in sync handler"
```

---

### Task 6: Workspace Full Verification

**Files:**
- Verify: Full workspace Rust build and tests
- Verify: Frontend TypeScript build

- [ ] **Step 1: Run full workspace test suite**
```bash
cargo test --workspace
```

- [ ] **Step 2: Run workspace clippy with `-D warnings`**
```bash
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: Build frontend**
```bash
cd atlas-desktop/frontend && npm run build
```

- [ ] **Step 4: Check git status and diff**
```bash
git status
```
