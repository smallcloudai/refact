---
title: "Core Architecture & Entry Points"
created: 2025-12-17
tags: ["architecture", "core-modules", "patterns", "design", "rust", "lsp", "refact-agent", "configuration", "testing"]
---

## Core Architecture & Entry Points

### Main Entry Points

**main.rs** - Application bootstrap
- Initializes `GlobalContext` with all subsystems
- Spawns HTTP server (Axum) and LSP server (tower-lsp)
- Handles graceful shutdown and signal management
- Loads configuration from YAML files

**lsp.rs** - Language Server Protocol implementation
- Implements tower-lsp traits for IDE communication
- Handles document synchronization
- Manages workspace symbols and definitions
- Bridges LSP requests to internal services

**http.rs** - REST API server
- Axum-based HTTP server for IDE clients
- Endpoints for completion, chat, RAG, tools
- Streaming response support
- Request validation and error handling

### GlobalContext - The Central Hub

Located in `global_context.rs`, this is the "god object" that coordinates all subsystems:

**Key Responsibilities:**
- Shared mutable state (Arc<RwLock<>>)
- AST indexing service
- VecDB (vector database) management
- File watching and caching
- Model provider configuration
- Tool execution context
- Telemetry and analytics

**Access Pattern:**
```
HTTP/LSP Request → GlobalContext.read() → Service Layer → Response
                 → GlobalContext.write() → State Update
```

**Important Fields:**
- `ast_service` - AST indexing and symbol resolution
- `vecdb` - Vector database for semantic search
- `file_cache` - Completion and file caching
- `caps` - Model capabilities and providers
- `tool_executor` - Tool execution engine
- `background_tasks` - Async task management

### Dual Protocol Architecture

**HTTP Server (Axum)**
- Primary interface for IDE clients
- RESTful endpoints
- Streaming support for long operations
- CORS and authentication handling

**LSP Server (tower-lsp)**
- Secondary interface for IDE integration
- Document synchronization
- Workspace symbol queries
- Hover, definition, references

**Shared State:**
Both servers access the same `GlobalContext`, ensuring consistency across protocols.

---

## Core Modules

### AST Module (`src/ast/`)

**Purpose:** Parse and index code structure across multiple languages

**Key Components:**
- `AstIndexService` - Main indexing service
- Tree-sitter integration for 6+ languages (Rust, Python, JavaScript, TypeScript, Go, Java)
- Symbol definition and reference tracking
- Incremental indexing on file changes

**Key Functions:**
- `ast_definition()` - Find symbol definition
- `ast_references()` - Find all symbol usages
- `pick_up_changes()` - Incremental indexing

**Design Pattern:**
- Background task updates AST on file changes
- Caches results for performance
- Fallback to file content if AST unavailable

### VecDB Module (`src/vecdb/`)

**Purpose:** Vector database for semantic code search and RAG

**Key Components:**
- `VecDb` - Main vector database interface
- SQLite backend with vector extensions
- Markdown splitter for code chunking
- Embedding generation and storage

**Key Functions:**
- `search()` - Semantic similarity search
- `index()` - Add code to vector database
- `get_status()` - VecDB indexing status

**Design Pattern:**
- Lazy initialization on first use
- Background indexing of workspace files
- Fallback to keyword search if vectors unavailable
- Configurable embedding models

### AT Commands Module (`src/at_commands/`)

**Purpose:** Special IDE commands for context injection and tool execution

**Key Commands:**
- `@file` - Include file content
- `@knowledge` - Search knowledge base
- `@definition` - Find symbol definition
- `@references` - Find symbol usages
- `@web` - Web search integration
- `@tool` - Execute external tools

**Design Pattern:**
- Command parsing and validation
- Context gathering from various sources
- Result formatting for LLM consumption
- Authorization checks for sensitive operations

### Tools Module (`src/tools/`)

**Purpose:** Execute external tools and integrate with external services

**Key Tools:**
- `tool_ast_definition` - AST-based symbol lookup
- `tool_create_agents_md` - Generate project documentation
- `tool_web_search` - Web search integration
- `tool_execute_command` - Shell command execution
- Custom tool support via configuration

**Design Pattern:**
- Tool registry with metadata
- Authorization and permission checking
- Result validation and sanitization
- Error handling and fallbacks

### Integrations Module (`src/integrations/`)

**Purpose:** Third-party service integrations

**Key Integrations:**
- OpenAI API client
- Hugging Face integration
- Custom LLM endpoints
- Cloud service connections

**Design Pattern:**
- Provider abstraction layer
- Fallback chains for redundancy
- Rate limiting and caching
- Error recovery strategies

---

## Configuration System

### YAML-Driven Configuration

**Location:** `src/yaml_configs/`

**Key Features:**
- Auto-generated configuration files
- Checksum validation for integrity
- Hot-reload capability
- Hierarchical configuration

**Configuration Files:**
- `providers.yaml` - Model provider definitions
- `capabilities.yaml` - Feature capabilities
- `tools.yaml` - Tool definitions and permissions
- `integrations.yaml` - Integration settings

**Design Pattern:**
- Configuration as code
- Validation on load
- Graceful degradation on missing configs
- Environment variable overrides

---

## Performance & Caching

### Completion Cache (`src/completion_cache.rs`)

**Purpose:** Cache completion results for repeated queries

**Strategy:**
- LRU cache with configurable size
- Invalidation on file changes
- Workspace-aware caching

### File Correction Cache (`src/files_correction_cache.rs`)

**Purpose:** Cache file correction results

**Strategy:**
- Persistent cache with TTL
- Invalidation on file modifications

### Background Tasks (`src/background_tasks.rs`)

**Purpose:** Async operations without blocking main thread

**Key Tasks:**
- AST indexing
- VecDB updates
- File watching
- Telemetry collection

**Design Pattern:**
- Tokio-based async runtime
- Task prioritization
- Graceful shutdown handling

---

## Testing Infrastructure

### Test Organization

**Location:** `refact-agent/engine/tests/`

**Test Types:**
1. **Integration Tests** - Python scripts testing HTTP/LSP endpoints
2. **Unit Tests** - Rust tests in source files
3. **Examples** - Executable examples in `examples/`

### Key Test Files

- `test01_completion_edge_cases.py` - Completion edge cases
- `test02_completion_with_rag.py` - RAG integration
- `test03_at_commands_completion.py` - @command testing
- `test04_completion_lsp.py` - LSP protocol testing
- `test05_is_openai_compatible.py` - OpenAI API compatibility
- `test12_tools_authorize_calls.py` - Tool authorization
- `test13_vision.py` - Vision/image capabilities

### Test Patterns

- Python test scripts use HTTP client
- LSP tests use `lsp_connect.py` helper
- Test data in `tests/test13_data/`
- Emergency test data in `tests/emergency_frog_situation/`

---

## Key Design Patterns

### 1. Layered Fallback Pattern

```
Request → Cache → VecDB → AST → Model Inference
         (fast)  (semantic) (structural) (comprehensive)
```

### 2. Background Task Pattern

```
Main Thread (HTTP/LSP) ← Shared State → Background Threads (Indexing/Watching)
```

### 3. Provider Abstraction

```
GlobalContext → Capabilities → Provider Selection → Model Inference
```

### 4. Command Execution Pattern

```
@command → Parser → Validator → Executor → Formatter → Response
```

### 5. Error Recovery Pattern

```
Try Primary → Catch Error → Try Fallback → Log & Return Default
```

---

## Important Utilities

### File Management (`src/files_*.rs`)

- `files_in_workspace.rs` - Discover workspace files
- `files_blocklist.rs` - Filter blocked files
- `files_correction.rs` - Fix file paths and content
- `file_filter.rs` - Apply filtering rules

### Privacy & Security (`src/privacy.rs`)

- Sensitive data masking
- Privacy-aware logging
- Data sanitization

### Telemetry (`src/telemetry/`)

- Usage tracking
- Analytics collection
- Privacy-compliant reporting

### Utilities

- `tokens.rs` - Token counting and management
- `json_utils.rs` - JSON parsing helpers
- `fuzzy_search.rs` - Fuzzy matching
- `nicer_logs.rs` - Enhanced logging

---

## Dependencies & Technology Stack

### Core Dependencies

- **tower-lsp** - LSP server implementation
- **axum** - HTTP server framework
- **tokio** - Async runtime
- **tree-sitter** - Code parsing (6+ languages)
- **sqlite-vec** - Vector database
- **serde** - Serialization
- **reqwest** - HTTP client

### Language Support

- Rust
- Python
- JavaScript/TypeScript
- Go
- Java
- C/C++

### External Services

- OpenAI API
- Hugging Face
- Custom LLM endpoints
- Web search APIs
- Cloud services

---

## Development Workflow

### Building

```bash
cd refact-agent/engine
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test
python tests/test01_completion_edge_cases.py
```

### Running

```bash
cargo run --bin refact-lsp
```

### Configuration

- YAML files in `src/yaml_configs/`
- Environment variables for overrides
- Hot-reload on file changes

---

## Current State & Branches

**Active Branch:** `debug_fixes_pt_42`

**Main Branches:**
- `main` - Stable release
- `dev` - Development
- `cloud-subchats` - Cloud features
- `main-stable-2` - Previous stable

**Staged Changes:** 2 files
**Modified Files:** 23 files

---

## Key Insights

1. **Scalability**: Dual protocol (HTTP + LSP) allows flexible IDE integration
2. **Performance**: Multi-layer caching and background indexing minimize latency
3. **Extensibility**: YAML-driven configuration enables easy feature addition
4. **Reliability**: Fallback chains and error recovery ensure graceful degradation
5. **Maintainability**: Clear separation of concerns across modules
6. **Testing**: Comprehensive integration tests validate functionality

