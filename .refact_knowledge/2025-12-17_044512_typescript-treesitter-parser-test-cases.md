---
title: "TypeScript Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "typescript", "ts", "refact-agent"]
---

### TypeScript Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains test fixtures (sample TypeScript source files paired with their expected parser outputs) for validating the TypeScript Tree-sitter parser implementation in refact-agent's AST processing pipeline. It ensures accurate syntax tree generation, symbol extraction (declarations/definitions), and skeletonization (structure-preserving code stripping) specifically for TypeScript/JavaScript variants. These tests support critical agent features like `@ast-definition` (jump-to-definition), `@ast-reference` (find usages), code navigation via `@tree`, and AST-driven RAG/context retrieval. As part of the multi-language test suite under `cases/`, it follows an identical golden-file pattern to siblings (`cpp/`, `java/`, `js/`, `kotlin/`, `python/`, `rust/`), enabling consistent, automated validation loaded by `tests/ts.rs`. This builds upon the core AST indexer (`ast_indexer_thread.rs`) and parser registry (`parsers/parsers.rs` → `ts.rs`), providing TypeScript-specific edge cases like interfaces, generics, decorators, and type-only imports/exports.

**• Files:**  
```
ts/
├── main.ts                      # Sample entry-point source: tests modules, imports/exports, classes, interfaces, async functions
├── main.ts.json                 # Expected full AST parse or complete symbol table (functions, types, exports in JSON)
├── person.ts                    # Sample source: likely tests type definitions, interfaces, generics, or OOP patterns
├── person.ts.decl_json          # Expected declarations/symbol table (vars, types, methods with locations/scopes)
└── person.ts.skeleton           # Expected skeletonized output: structural code only (no impl details, comments, literals)
```
- **Organization pattern**: Matches all `cases/*/` dirs precisely—source files (`.ts`) pair with `.json`/`.decl_json` (parse/symbol golden results) and `.skeleton` (structure-only). Minimalist (2 sources), focused on core TS constructs vs. broader JS coverage in sibling `js/`. No subdirs; flat for simple test loading.

**• Architecture:**  
- **Role in layered architecture**: Test/data layer for the AST module (`src/ast/treesitter/parsers/`), validating the parser abstraction boundary in `parsers.ts.rs`. Uses Tree-sitter's incremental parsing via `tree-sitter` crate, integrated with `ast_instance_structs.rs` for typed nodes and `file_ast_markup.rs` for markup/symbol extraction.
- **Design patterns**: Golden-file testing (expected outputs as data-driven tests); cross-language uniformity (shared loader in `tests/*.rs`); separation of parse/symbol/skeleton concerns.
- **Data flow**: Fixtures → `ts.rs` test module → parser (`ts.rs`) → AST build → JSON serialization/symbol extraction → skeletonizer (`skeletonizer.rs`) → assert equality.
- **Relationships**: 
  - **Used by**: `tests/ts.rs` (test runner); indirectly `at_ast_definition.rs`, `at_ast_reference.rs`, `tool_ast_definition.rs` (via indexed AST DB).
  - **Uses**: Parser registry (`parsers/parsers.rs` → language_id.rs` for TS detection); AST structs (`structs.rs`).
  - **Integration**: Feeds `ast_db.rs` / `ast_indexer_thread.rs` for workspace indexing; powers HTTP/LSP endpoints (`http/routers/v1/ast.rs`).

**• Key Symbols (inferred from parser/tests context):**  
- **Parsers**: `get_typescript_parser()` in `parsers/ts.rs` (loads Tree-sitter TS grammar).
- **Test fns**: In `tests/ts.rs` – `test_parse()`, `test_declarations()`, `test_skeletonize()` (load fixtures, assert outputs).
- **AST Types**: `AstInstance`, `SymbolDecl` (from `ast_instance_structs.rs` / `structs.rs`).
- **Utilities**: `file_ast_markup.rs` (markup gen), `skeletonizer.rs` (structure extraction).

**• Integration:**  
Fits Refact Agent's AST-centric architecture by providing TS-specific validation for the multi-lang parser system. Unlike pure JS tests (`js/`), emphasizes type system handling (e.g., `person.ts` likely tests interfaces/types). Builds upon general AST patterns from `parse_common.rs` but specializes via `language_id.rs` (TS vs. JS detection). Dependencies flow upward to agent tools (`tools/tool_ast_*.rs`), `@at` commands (`at_ast_*.rs`), and indexing (`ast_parse_anything.rs`). Ensures reliability for IDE features (LSP code lens, go-to-def) and agentic reasoning over TS projects. No runtime deps—pure test data.
