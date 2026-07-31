# Atlas Vision

> **Build the universal engineering context platform.**

Modern software engineering is fragmented.

A single feature is rarely contained within a single tool.

Requirements live in Jira.
Architecture decisions live in Confluence.
Source code lives in Git.
Pull requests live in GitHub.
Specifications live in OpenAPI.
Documentation lives in Markdown.
Knowledge is scattered across countless systems.

Developers spend a significant amount of time switching between tools, reconstructing context before they can write a single line of code.

AI assistants face the same problem.

Every coding agent repeatedly fetches information from multiple systems, receives inconsistent schemas, duplicates requests, and wastes valuable context windows.

Atlas exists to eliminate that fragmentation.

Rather than replacing existing tools, Atlas continuously collects, normalizes, connects, and enriches engineering knowledge into a single, reliable source of engineering context.

This context can then be consumed consistently by developers, IDEs, AI assistants, automation platforms, and future engineering tools.

Atlas is the infrastructure between engineering systems and engineering work.

---

# What Atlas Is

Atlas is an Engineering Context Platform.

Its responsibilities are to:

* Collect engineering knowledge from many systems.
* Normalize different vendor-specific schemas into a canonical model.
* Preserve relationships between artifacts.
* Build complete engineering context.
* Deliver that context through stable interfaces.

Atlas enables software teams to spend less time searching for information and more time building software.

---

# What Atlas Is Not

Atlas is not:

* a project management platform
* a documentation platform
* a source code hosting platform
* an AI chatbot
* an AI coding agent
* a workflow automation platform

Atlas complements these systems instead of replacing them.

AI models generate content.

Workflow engines execute tasks.

Atlas provides the engineering context they rely on.

---

# Principles

## Engineering Context First

Knowledge alone is not enough.

Atlas exists to assemble complete engineering context by connecting related artifacts across systems.

Understanding relationships is more valuable than indexing isolated documents.

---

## Local First

Engineering knowledge belongs to the developer.

Atlas should continue working even when cloud services are unavailable.

Previously synchronized knowledge should remain accessible offline.

---

## CLI First

Every capability should be available through the command line.

Desktop applications, web interfaces, IDE plugins, and AI integrations are consumers—not the primary interface.

---

## Vendor Neutral

Atlas should integrate with engineering ecosystems without becoming dependent on any vendor.

Connectors are replaceable.

The canonical model remains stable.

---

## AI Agnostic

Atlas should never depend on a specific AI provider.

Whether the consumer is Codex, Claude, Gemini, OpenAI, Ollama, or future models should not matter.

Atlas provides context.

Consumers decide how to use it.

---

## Composable

Every capability should be reusable.

The same engineering context should power:

* CLI commands
* Desktop applications
* REST APIs
* MCP servers
* IDE integrations
* Automation pipelines
* AI workflows
* Future tooling

without duplicating business logic.

---

## Simplicity Over Complexity

Prefer simple, maintainable architecture over unnecessary abstraction.

Every new subsystem must justify its existence.

---

## Open Foundation

Atlas should become a platform that developers and organizations can extend through connectors, integrations, workflows, and tooling without modifying the core engine.

---

# Long-Term Direction

Atlas aims to become the common engineering context layer shared by every engineering tool.

Instead of every AI assistant, IDE, or automation platform implementing its own integrations with Jira, Confluence, GitHub, GitLab, Notion, and countless other systems, they integrate with Atlas once.

Atlas becomes the single source of engineering context.

Everything else builds on top of it.

---

# Vision Statement

> **One engineering context. Infinite possibilities.**

Engineering tools will continue to evolve.

AI models will continue to change.

Protocols will come and go.

But software teams will always need reliable engineering context.

Atlas exists to provide that foundation.
