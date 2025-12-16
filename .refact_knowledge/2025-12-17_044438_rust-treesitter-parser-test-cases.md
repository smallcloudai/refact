---
title: "Rust Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "rust", "refact-agent"]
---

### Rust Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains Rust-specific test fixtures (source files paired with expected outputs) for validating the Tree-sitter parser implementation in refact-agent's AST system. It tests accurate parsing of Rust syntax into ASTs, extraction of symbols (declarations/definitions), and skeletonization (structural abstraction without implementation details). These golden tests ensure the parser supports Rust code analysis features like "go to definition" (`at_ast_definition.rs`), "find references" (`at_ast_reference.rs`), incremental indexing (`ast_indexer_thread.rs`), and agentic tooling. As part of a consistent cross-language testing strategy, it mirrors sibling directories (`cpp/`, `java/`, `js/`, `kotlin/`, `python/`, `ts/`), enabling uniform validation loaded by `tests/rust.rs`.

**• Files:**  
```
rust/
├── main.rs                      # Sample Rust entry point: tests modules, functions, traits, impls, enums
├── main.rs.json                 # Expected full AST parse or symbol table JSON (complete node structure/decls)
├── point.rs                     # Sample Rust module: likely tests structs, methods, generics, lifetimes
├── point.rs.decl_json          # Expected declarations JSON: extracted symbols (structs, fns, types, visibilities)
└── point.rs.skeleton           # Expected skeletonized source: structure-only (signatures without bodies)
```
- **Naming pattern**: `*.rs` sources → `*.rs.json` (full parse/symbols), `*.rs.decl_json` (decl-only), `*.skeleton` (abstraction). Pairs enable precise diff-based assertions in `tests/rust.rs`.
- **Organization**: Matches all `cases/*/` dirs—minimal, focused samples covering core language features (e.g., ownership, traits, async) without complexity.

**• Architecture:**  
- **Single Responsibility**: Pure data-driven testing—no logic, just inputs/outputs for parser black-box validation.
- **Pattern**: Golden file testing (source → expected AST/symbols/skeleton). Builds upon core AST pipeline: `parsers/rust.rs` → `file_ast_markup.rs`/`skeletonizer.rs` → `ast_instance_structs.rs`.
- **Relationships**: 
  - **Used by**: `treesitter/parsers/tests/rust.rs` (test runner loads/parses/compares).
  - **Uses**: None (static data); integrates with `language_id.rs`, `ast_structs.rs`.
  - **Cross-lang consistency**: Identical to JS (`car.js/main.js`), Python (`calculator.py`), etc.—"This follows the exact pattern from `js/`, `java/`, etc., introducing Rust-specific handling (e.g., lifetimes, impls) unlike simpler langs."
- **Layered fit**: Data layer → AST module (`src/ast/treesitter/`) → AT commands/tools → LSP/HTTP handlers.

**• Key Symbols:**  
- No code; fixtures validate parser-extracted symbols like `struct Point`, `fn main()`, `impl Point`, visibilities (`pub`), lifetimes (`'a`).
- Tests `ast_structs.rs` types: `AstInstance`, declaration nodes, spans.

**• Integration:**  
- **Data Flow**: `rust.rs` test reads files → invokes `parsers/rust.rs`/`skeletonizer.rs` → asserts against `.json`/`.skeleton`.
- **Dependencies**: Tree-sitter Rust grammar; shared utils (`utils.rs`, `structs.rs`).
- **Extension**: New fixtures added for grammar updates; supports multi-lang AST DB (`ast_db.rs`).
- **Unlike others**: Rust tests emphasize ownership/traits vs. JS prototypes or Python dynamics, but shares abstraction boundaries for uniform `at_*` tools. This completes the 7-lang suite (`cpp/java/js/kotlin/python/rust/ts`), enabling comprehensive parser reliability in the LSP agent.
