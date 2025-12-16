---
title: "Tree-sitter Parser Test Cases Directory Analysis"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "test-cases", "refact-agent"]
---

### Tree-sitter Parser Test Cases Directory Analysis

**• Purpose:**  
This directory serves as the test data repository for the Tree-sitter parser implementations in the refact-agent's AST (Abstract Syntax Tree) subsystem. It contains language-specific sample source files, their corresponding parsed JSON outputs, and skeletonized representations. The primary goal is to validate parser accuracy, ensure consistent AST generation across languages, and test features like symbol extraction (declarations) and code skeletonization. These tests verify that Tree-sitter grammars correctly handle real-world code constructs for languages supported by the agent (C++, Java, JavaScript, Kotlin, Python, Rust, TypeScript), enabling reliable code analysis, completions, definitions/references (@ast_definition, @ast_reference), and indexing in the broader AST pipeline.

**• Files:**  
Organized by language in subdirectories (`cpp/`, `java/`, `js/`, `kotlin/`, `python/`, `rust/`, `ts/`), each containing minimal but representative code samples and their parser outputs:
- **Source files** (e.g., `main.cpp`, `circle.cpp`, `calculator.py`, `main.rs`): Simple, self-contained programs demonstrating key language features (classes, functions, imports, OOP constructs).
- **JSON dumps** (e.g., `main.cpp.json`, `person.java.decl_json`): Full AST serialization from Tree-sitter queries, capturing node hierarchies, spans, and metadata.
- **Declaration JSONs** (e.g., `circle.cpp.decl_json`, `person.kt.decl_json`): Extracted symbol tables focusing on definitions (functions, classes, variables).
- **Skeleton files** (e.g., `circle.cpp.skeleton`, `car.js.skeleton`): Simplified code representations stripping bodies/details, used for RAG/indexing previews or diff analysis.
  
| Language | Key Files | Purpose |
|----------|-----------|---------|
| C++ (`cpp/`) | `main.cpp`, `circle.cpp`, `.json`/`.decl_json`/`.skeleton` | Tests class/method parsing, includes. |
| Java (`java/`) | `main.java`, `person.java` + outputs | OOP inheritance, constructors. |
| JS (`js/`) | `main.js`, `car.js` + outputs | Prototypes, closures, modules. |
| Kotlin (`kotlin/`) | `main.kt`, `person.kt` + outputs (note: duplicate `person.kt.json`) | Coroutines, data classes, extensions. |
| Python (`python/`) | `main.py`, `calculator.py` + outputs | Functions, classes, comprehensions. |
| Rust (`rust/`) | `main.rs`, `point.rs` + outputs | Traits, structs, ownership patterns. |
| TS (`ts/`) | `main.ts`, `person.ts` + outputs | Interfaces, generics, type annotations. |

No raw test runner files here—these artifacts are consumed by corresponding test modules like `tests/cpp.rs`, `tests/python.rs` (in `parsers/tests/`), which load/parse/validate them.

**• Architecture:**  
- **Layered Testing Pattern**: Fits into the AST module's parse → query → index pipeline (`src/ast/treesitter/parsers.rs` orchestrates language-specific parsers like `cpp.rs`, `rust.rs`). Tests validate the "parse_anything" contract from `ast_parse_anything.rs`.
- **Data-Driven Testing**: Each language mirrors production parser modules (`parsers/{lang}.rs`), using identical Tree-sitter grammars. Follows golden-file pattern: source → expected JSON/skeleton.
- **Relationships**:
  - **Used by**: `treesitter/parsers/tests/{lang}.rs` (test runners), `skeletonizer.rs` (validates stripping logic), `ast_instance_structs.rs`/`file_ast_markup.rs` (AST node mapping).
  - **Uses**: Tree-sitter crates (via `language_id.rs`), query files for decls/skeletons.
  - **Integration**: Feeds into `ast_indexer_thread.rs`/`ast_db.rs` for workspace indexing; errors surface via `custom_error.rs`. Cross-references `alt_testsuite/` (more complex "torture" cases).
- **Patterns**: Repository pattern for test fixtures; language symmetry ensures uniform API (`structs.rs`). No runtime deps—pure validation.

**• Key Symbols:**  
(From consuming modules, inferred via structure:)
- `AstInstance`, `FileAstMarkup` (`ast_instance_structs.rs`): Structures validated against JSON.
- Parser fns: `parse_cpp()`, `skeletonize()` (`parsers/{lang}.rs`, `skeletonizer.rs`).
- Test utils: `load_test_case()`, `assert_ast_eq()` (in `parsers/tests/{lang}.rs`).
- Queries: Tree-sitter S-expression patterns for "decls", "skeleton" (in parser modules).

**• Integration:**  
- **Within AST**: Bottom of parse layer → top of indexing (`file_splitter.rs`, `chunk_utils.rs`). Builds upon `parse_common.rs`/`parse_python.rs` by providing concrete validation data.
- **Broader Agent**: Enables `@at_ast_definition`/`@at_ast_reference` (`at_commands/`), code completion RAG (`scratchpads/completon_rag.rs`), tools (`tool_ast_definition.rs`).
- **Cross-module**: Unlike general `tests/` (Python integration), this is Rust-unit focused. Complements `alt_testsuite/` (edge-case annotated files). Outputs feed VecDB indirectly via indexed skeletons.
- **Extension**: Easy to add languages (new dir + parser.rs + test.rs). Unlike VecDB tests (dynamic), these are static for parser fidelity.

This directory embodies "test as documentation/spec"—files double as minimal repros for parser bugs, making the AST subsystem robust for multi-language agentic workflows. Compares to existing knowledge by providing the concrete data behind previously documented per-language test cases (Python/JS/etc.).
