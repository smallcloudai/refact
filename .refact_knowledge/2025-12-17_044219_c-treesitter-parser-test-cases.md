---
title: "C++ Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "cpp", "refact-agent"]
---

### C++ Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains test fixtures (sample C++ source files and their expected outputs) specifically for validating the C++ Tree-sitter parser implementation within the refact-agent's AST system. It enables automated testing of syntax tree generation, symbol extraction (declarations, definitions), and skeletonization (stripping implementation details while preserving structure). These tests ensure the parser accurately handles real-world C++ code for features like "go to definition" (`ast_definition`), "find references" (`ast_references`), and code analysis in the LSP server. The tests build upon the core AST module's incremental indexing and multi-language support by providing language-specific validation data.

**• Files:**  
```
cpp/
├── circle.cpp                    # Sample C++ source: likely tests class/struct definitions, methods
├── circle.cpp.decl_json          # Expected JSON output: extracted declarations/symbol table
├── circle.cpp.skeleton           # Expected skeletonized version: structure-only (no bodies/implementation)
├── main.cpp                      # Sample C++ source: entry point, function calls, includes
└── main.cpp.json                 # Expected JSON output: full AST parse or symbol info
```
- **Organization pattern**: Each test case pairs a `.cpp` source file with companion `.json` (parse/symbol results) and `.skeleton` (structure-only) files. This mirrors test setups in sibling directories (`java/`, `python/`, etc.), enabling consistent cross-language validation.
- **Naming convention**: `filename.lang[.variant].{json|decl_json|skeleton}` – clear, machine-readable, focused on parser outputs.
- Notable: Directory reported as "empty" in file read, but structure confirms 5 fixture files present for targeted C++ testing.

**• Architecture:**  
- **Role in AST pipeline**: Part of `src/ast/treesitter/parsers/tests/cases/` hierarchy, consumed by `tests/cpp.rs` (test runner module). Tests invoke Tree-sitter's C++ grammar (`parsers/cpp.rs`) to parse files, then validate against golden `.json`/`.skeleton` outputs.
- **Design patterns**:
  - **Golden file testing**: Compare runtime parser output vs. pre-approved fixtures for regression-proofing.
  - **Modular language isolation**: Per-language subdirs allow independent grammar evolution without affecting others (e.g., Rust/Python parsers unchanged).
  - **Multi-output validation**: Tests three concerns simultaneously – raw AST (`json`), symbols (`decl_json`), structure (`skeleton`) – covering the full AST-to-analysis flow.
- **Data flow**: `file_ast_markup.rs` or `skeletonizer.rs` processes `.cpp` → generates AST/symbols → serializes to JSON → `tests/cpp.rs` asserts equality.
- **Fits layered architecture**: Bottom layer (Tree-sitter parsing) → tested here → feeds `ast_db.rs`/`ast_indexer_thread.rs` for background indexing → used by `at_ast_definition.rs`/`at_ast_reference.rs`.
- **Error handling**: Implicit via test failures; likely uses `anyhow` or custom `custom_error.rs` for parse errors.
- **Extension points**: Easy to add new C++ edge cases (templates, lambdas, STL) without code changes.

**• Key Symbols (inferred from test consumers):**  
- From `ast_instance_structs.rs`/`structs.rs`: `AstInstance`, `SymbolDecl`, `SkeletonNode` – parsed/validated here.
- Parser entry: `parsers/cpp.rs::parse_cpp()` or similar – Tree-sitter query capture for C++ nodes.
- Test harness: `tests/cpp.rs` likely exports `test_cpp_cases()` calling `language_id.rs::Cpp`, `file_ast_markup.rs::markup_file()`.
- Cross-references: Relies on `parse_common.rs`, `utils.rs`; outputs feed `ast_structs.rs::Node`.

**• Integration:**  
- **Used by**: `src/ast/treesitter/parsers/tests/cpp.rs` (direct test runner); indirectly powers `at_ast_definition.rs`, `tool_ast_definition.rs`, `ast_indexer_thread.rs` for live IDE queries.
- **Uses from others**: Tree-sitter grammars (`parsers/cpp.rs`), shared utils (`treesitter/utils.rs`, `chunk_utils.rs`), `language_id.rs` for C++ detection.
- **Relationships**:
  | Depends On          | Used By                     | Communication |
  |---------------------|-----------------------------|---------------|
  | `parsers/cpp.rs`   | `tests/cpp.rs`             | File paths, parse results |
  | `skeletonizer.rs`  | `ast_db.rs`                | Skeleton strings |
  | `language_id.rs`   | `ast_parse_anything.rs`    | Language enum (Cpp) |
- **Comes after**: General `alt_testsuite/` (annotated complex cases like `cpp_goat_library.cpp`); more focused than multi-lang `tests.ts`.
- **Comparison to existing knowledge**: Builds upon AST module's "Tree-sitter integration for 6+ languages" (core arch doc) by providing C++-specific fixtures. Unlike broader `ast_indexer_thread.rs` (runtime indexing), this is pure parser validation. Introduces language-specific golden testing pattern seen across `js/`, `rust/`, etc., enabling "pick_up_changes()" incremental updates with confidence.

This test suite ensures C++ parsing reliability in the agent's multi-language AST system, critical for production IDE features like symbol navigation.
