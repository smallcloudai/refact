---
title: "Java Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "java", "refact-agent"]
---

### Java Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains test fixtures (sample Java source files and their expected outputs) specifically for validating the Java Tree-sitter parser implementation within refact-agent's AST system. It enables automated testing of syntax tree generation, symbol extraction (declarations, definitions), and skeletonization (stripping implementation details while preserving structure). These tests ensure the parser accurately handles real-world Java code for features like "go to definition" (`at_ast_definition.rs`), "find references" (`at_ast_reference.rs`), and code analysis in the LSP server. The tests build upon the core AST module's incremental indexing (`ast_indexer_thread.rs`) and multi-language support by providing Java-specific validation data, mirroring the pattern seen in sibling directories like `cpp/`, `python/`, etc.

**• Files:**  
```
java/
├── main.java                    # Sample Java source: likely entry point with class definitions, methods, imports
├── main.java.json               # Expected JSON output: full AST parse or symbol info
├── person.java                  # Sample Java source: tests class/struct definitions, fields, constructors
├── person.java.decl_json        # Expected JSON output: extracted declarations/symbol table
└── person.java.skeleton         # Expected skeletonized version: structure-only (no bodies/implementation)
```
- **Organization pattern**: Each test case pairs a `.java` source file with companion `.json` (parse/symbol results) and `.skeleton` (structure-only) files. This enables consistent cross-language validation across `cases/` subdirectories (`cpp/`, `js/`, `kotlin/`, `python/`, `rust/`, `ts/`).
- **Naming convention**: `filename.lang[.variant].{json|decl_json|skeleton}` – clear, machine-readable, focused on parser outputs (e.g., `decl_json` for symbol tables, `skeleton` for structural stripping).
- **Notable details**: Simple, focused examples (`main` + `person`) cover core Java constructs like classes, methods, and fields. Directory reported as "empty" in file read, but structure confirms 5 fixture files present for targeted Java testing.

**• Architecture:**  
- **Module role**: Part of the AST subsystem (`src/ast/treesitter/parsers/tests/`), which uses golden-file testing (source + expected outputs) to validate Tree-sitter parsers. Follows a layered pattern: raw Tree-sitter grammars (`parsers/java.rs`) → parse/symbol extraction (`file_ast_markup.rs`, `ast_instance_structs.rs`) → skeletonization (`skeletonizer.rs`) → indexing (`ast_db.rs`).
- **Design patterns**: Golden testing (compare actual vs. expected outputs); language-specific isolation for multi-lang support; incremental parsing validation to support live IDE updates via `ast_parse_anything.rs`.
- **Relationships**: Sibling to `cpp/`, `kotlin/` (OO languages with similar class/method structures). Fits into refact-agent's layered architecture: AST layer feeds tools/AT commands → HTTP/LSP handlers → agentic features.
- **Comes after**: Broader `alt_testsuite/` (complex annotated cases); more focused than multi-lang `tests.rs`.
- **Comparison to existing knowledge**: Directly analogous to C++ test cases (from knowledge base), which use identical structure (`circle.cpp`/`main.cpp` → `person.java`/`main.java`). Builds upon AST module's "Tree-sitter integration for 6+ languages" by adding Java-specific fixtures. Unlike runtime indexing (`ast_indexer_thread.rs`), this is pure parser validation via `tests/java.rs`. Introduces consistent golden testing pattern across OO languages, enabling reliable "pick_up_changes()" incremental updates.

**• Key Symbols:**  
- No runtime symbols (pure data files), but validates parser outputs feeding:
  | Symbol/Path              | Purpose                          |
  |--------------------------|----------------------------------|
  | `ast_structs.rs::Node`  | Stores parsed AST nodes         |
  | `language_id.rs::Java`  | Language enum for detection     |
  | `skeletonizer.rs`       | Generates `.skeleton` files     |
  | `parsers/java.rs`       | Tree-sitter grammar/query defs  |

**• Integration:**  
- **Used by**: `src/ast/treesitter/parsers/tests/java.rs` (direct test runner loads these files, parses, compares JSON/skeletons); indirectly powers `at_ast_definition.rs`, `tool_ast_definition.rs`, `ast_indexer_thread.rs` for live IDE queries (e.g., go-to-definition in Java projects).
- **Uses from others**: Tree-sitter grammars (`parsers/java.rs`), shared utils (`treesitter/utils.rs`, `chunk_utils.rs`, `parse_common.rs`), `language_id.rs` for Java detection.
- **Relationships**:
  | Depends On            | Used By                       | Communication          |
  |-----------------------|-------------------------------|------------------------|
  | `parsers/java.rs`    | `tests/java.rs`              | File paths, parse results |
  | `skeletonizer.rs`    | `ast_db.rs`                  | Skeleton strings      |
  | `language_id.rs`     | `ast_parse_anything.rs`      | Language enum (Java)  |
  | `file_ast_markup.rs` | `at_ast_reference.rs`        | Symbol tables (decl_json) |
- **Data flow**: Fixtures → `tests/java.rs` (parse → serialize → assert_eq!) → confidence in `ast_db.rs` insertion → runtime queries via LSP/HTTP (`v1/ast.rs`).
- **Cross-cutting**: Error handling via parse failures in tests; supports multi-language AST index used by VecDB (`vecdb/`) and agent tools (`tools/tool_ast_definition.rs`).

This test suite ensures Java parsing reliability in the agent's multi-language AST system, critical for production IDE features like symbol navigation in Java/Kotlin projects.
