---
title: "JavaScript Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "javascript", "js", "refact-agent"]
---

### JavaScript Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains test fixtures (sample JavaScript source files and their expected outputs) specifically for validating the JavaScript Tree-sitter parser implementation within refact-agent's AST system. It enables automated testing of syntax tree generation, symbol extraction (declarations, definitions), and skeletonization (stripping implementation details while preserving structure). These tests ensure the parser accurately handles real-world JavaScript code for features like "go to definition" (`at_ast_definition.rs`), "find references" (`at_ast_reference.rs`), code analysis, and agentic tools in the LSP server. The tests build upon the core AST module's incremental indexing (`ast_indexer_thread.rs`) and multi-language support by providing JavaScript-specific validation data, following the exact pattern of sibling directories like `cpp/`, `java/`, `python/`, etc.

**• Files:**  
```
js/
├── car.js                       # Sample JS source: likely tests object literals, functions, prototypes, or ES6+ features
├── car.js.decl_json             # Expected JSON output: extracted declarations/symbol table (functions, vars, exports)
├── car.js.skeleton              # Expected skeletonized version: structure-only (no function bodies/implementation details)
├── main.js                      # Sample JS source: entry point with modules, imports/exports, async functions, classes
└── main.js.json                 # Expected JSON output: full AST parse or complete symbol info
```
- **Organization pattern**: Identical to other language test dirs—each `.js` source pairs with `.json` (parse/symbol results) and `.skeleton` (structure-only) files. This enables consistent cross-language golden testing across `cases/` subdirectories (`cpp/`, `java/`, `kotlin/`, `python/`, `rust/`, `ts/`), loaded by `tests/js.rs`.

**• Architecture:**  
- **Design patterns**: Golden testing (compare parser output vs. expected files); language-isolated validation for scalable multi-lang support (7+ languages via `language_id.rs`); supports incremental parsing for live IDE reloads (`ast_parse_anything.rs`, `ast_indexer_thread.rs`). Fits refact-agent's layered architecture: AST parsers → `ast_structs.rs` nodes → tools/AT commands (`at_ast_*`) → HTTP/LSP handlers → agentic workflows.
- **Module relationships**: Consumed by `treesitter/parsers/tests/js.rs` for unit tests; feeds `parsers/js.rs` (language-specific queries/grammars); upstream from `file_ast_markup.rs` and `skeletonizer.rs`. Sibling to `ts/` (TypeScript, sharing JS grammar base).
- **Comparison to existing knowledge**: Directly analogous to documented Java/C++ cases (`person.java`/`circle.cpp` → `person.js`/`car.js` naming for "entity modeling"). Unlike broader `alt_testsuite/` (edge-case annotated files like `py_torture*.py`), this focuses on canonical parser validation. Builds upon "Tree-sitter integration for 6+ languages" (from project architecture knowledge) by extending to dynamic/scripting langs like JS. Introduces JS-specific challenges (hoisting, closures, dynamic imports) vs. static OO langs, enabling reliable incremental updates via `pick_up_changes()`.

**• Key Symbols:**  
- No runtime symbols (pure data fixtures), but validates outputs feeding core AST pipeline:
  | Symbol/Path                  | Purpose                                      |
  |------------------------------|----------------------------------------------|
  | `ast_structs.rs::Node`       | Parsed AST nodes from JS source              |
  | `language_id.rs::JavaScript` | Language enum for `.js` detection            |
  | `skeletonizer.rs`            | Generates `.skeleton` files (structure only) |
  | `parsers/js.rs`              | JS-specific Tree-sitter grammar/capture queries |
  | `tests/js.rs`                | Test runner loading these fixtures           |
  | `ast_db.rs`                  | Stores validated JS symbols in workspace DB  |

**• Integration:**  
- **Used by**: `treesitter/parsers/tests/js.rs` (direct test loader); indirectly powers `@ast-definition`/`@ast-reference` AT commands, code completion (`scratchpads/code_completion_*`), and RAG via `vecdb/*`.
- **Uses**: Tree-sitter JS grammar (external crate); `parse_common.rs` for shared parsing logic.
- **Communication**: Fixtures → test assertions → parser validation → runtime AST indexing (`ast_indexer_thread.rs`).
- **Dependencies**: Part of AST layer; no external deps beyond Tree-sitter. Extension point: Add new fixtures for JS edge cases (e.g., React JSX via queries in `parsers/js.rs`). In broader refact-agent flow: Workspace files → these parsers → `tools/tool_ast_*` → agent chat/tools.
