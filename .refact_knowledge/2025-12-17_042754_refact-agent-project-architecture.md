---
title: "Refact Agent Project Architecture"
created: 2025-12-17
tags: ["architecture", "project-structure", "rust", "lsp", "refact-agent"]
---

## Refact Agent Project Architecture

### Project Overview
Refact Agent is a Rust-based LSP (Language Server Protocol) server designed to integrate with IDEs like VSCode and JetBrains. It maintains up-to-date AST (Abstract Syntax Tree) and VecDB (Vector Database) indexes for efficient code completion and project analysis.

### Core Components

#### 1. **refact-agent/engine** (Main Rust Application)
The heart of the project, containing:
- **src/main.rs** - Entry point for the LSP server
- **src/lsp.rs** - LSP protocol implementation
- **src/http.rs** - HTTP server for IDE communication
- **src/background_tasks.rs** - Async background task management
- **src/global_context.rs** - Shared application state

#### 2. **Key Modules**

**AST Module** (`src/ast/`)
- Handles Abstract Syntax Tree parsing and indexing
- Supports multiple programming languages
- Provides symbol definitions and references

**VecDB Module** (`src/vecdb/`)
- Vector database for semantic code search
- Markdown splitting and indexing
- Efficient similarity-based code retrieval

**AT Commands** (`src/at_commands/`)
- Special commands for IDE integration (e.g., @knowledge, @file)
- Command parsing and execution
- Context-aware completions

**Tools** (`src/tools/`)
- External tool integrations (browsers, databases, debuggers)
- Tool authorization and execution
- Tool result processing

**Integrations** (`src/integrations/`)
- Third-party service integrations
- API clients and handlers

**HTTP Module** (`src/http/`)
- REST API endpoints for IDE clients
- Request/response handling
- Streaming support for long-running operations

#### 3. **Supporting Systems**

**Completion Cache** (`src/completion_cache.rs`)
- Caches completion results for performance
- Invalidation strategies

**File Management** (`src/files_*.rs`)
- File filtering and blocklisting
- Workspace file discovery
- File correction and caching

**Telemetry** (`src/telemetry/`)
- Usage tracking and analytics
- Privacy-aware data collection

**Postprocessing** (`src/postprocessing/`)
- Result formatting and enhancement
- Output optimization

### Project Structure

```
refact-agent/
├── engine/
│   ├── src/
│   │   ├── agentic/          # Agentic features
│   │   ├── ast/              # AST parsing and indexing
│   │   ├── at_commands/      # IDE command handlers
│   │   ├── caps/             # Capabilities management
│   │   ├── cloud/            # Cloud integration
│   │   ├── dashboard/        # Dashboard features
│   │   ├── git/              # Git integration
│   │   ├── http/             # HTTP server
│   │   ├── integrations/     # External integrations
│   │   ├── postprocessing/   # Result processing
│   │   ├── scratchpads/      # Scratchpad management
│   │   ├── telemetry/        # Analytics
│   │   ├── tools/            # Tool integrations
│   │   ├── vecdb/            # Vector database
│   │   ├── yaml_configs/     # Configuration handling
│   │   └── main.rs, lsp.rs, http.rs, etc.
│   ├── tests/                # Python test scripts
│   ├── examples/             # Usage examples
│   └── Cargo.toml
├── gui/                      # TypeScript/React frontend
└── python_binding_and_cmdline/  # Python bindings
```

### Key Technologies

- **Language**: Rust (backend), TypeScript/React (frontend)
- **Protocol**: LSP (Language Server Protocol)
- **Database**: SQLite with vector extensions
- **APIs**: REST, GraphQL
- **Testing**: Python test scripts, Rust unit tests

### Development Workflow

1. **Building**: `cargo build` in `refact-agent/engine`
2. **Testing**: Python test scripts in `tests/` directory
3. **Running**: LSP server runs as background process
4. **Configuration**: YAML-based configuration system

### Important Files

- `Cargo.toml` - Rust dependencies and workspace configuration
- `known_models.json` - Supported AI models configuration
- `build.rs` - Build script for code generation
- `rustfmt.toml` - Code formatting rules

### Integration Points

- **IDEs**: VSCode, JetBrains (via LSP)
- **External APIs**: OpenAI, Hugging Face, custom endpoints
- **Databases**: SQLite, Vector databases
- **Version Control**: Git integration
- **Cloud Services**: Cloud-based features and authentication

### Current Branch
- **Active**: `debug_fixes_pt_42`
- **Main branches**: `main`, `dev`, `cloud-subchats`
- **Staging**: 2 files, Modified: 23 files

### Testing Infrastructure

- **Unit Tests**: Rust tests in source files
- **Integration Tests**: Python scripts in `tests/` directory
- **Examples**: Executable examples in `examples/` directory
- **Test Data**: Sample data in `tests/test13_data/`

### Performance Considerations

- Completion caching for reduced latency
- Async background tasks for non-blocking operations
- Vector database for efficient semantic search
- Streaming responses for large outputs

