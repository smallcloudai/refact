---
title: "Python Tree-sitter Parser Test Cases"
created: 2025-12-17
tags: ["architecture", "ast", "treesitter", "parsers", "tests", "python", "refact-agent"]
---

### Python Tree-sitter Parser Test Cases

**• Purpose:**  
This directory contains test fixtures (sample Python source files and their expected outputs) specifically for validating the Python Tree-sitter parser implementation within refact-agent's AST system. It enables automated testing of syntax tree generation, symbol extraction (declarations, definitions), and skeletonization (stripping implementation details while preserving structure). These tests ensure the parser accurately handles real-world Python code for features like "go to definition" (`at_ast_definition.rs`), "find references" (`at_ast_reference.rs`), code analysis, and agentic tools in the LSP server. The tests build upon the core AST module's incremental indexing (`ast_indexer_thread.rs`) and multi-language support by providing Python-specific validation data, following the exact pattern of sibling directories like `cpp/`, `java/`, `js/`, `kotlin/`, `rust/`, and `ts/`.

**• Files:**  
```
python/
├── calculator.py                # Sample Python source: likely tests arithmetic expressions, functions, classes, or decorators
├── calculator.py.decl_json      # Expected JSON output: extracted declarations/symbol table (functions, classes, globals)
├── calculator.py.skeleton       # Expected skeletonized version: structure-only (no function bodies/implementation details)
├── main.py                      # Sample Python source: entry point with imports, modules, comprehensions, or async code
└── main.py.json                 # Expected JSON output: full AST parse or complete symbol info
```
- **Organization pattern**: Identical to other language test dirs—each `.py` source pairs with `.json` (parse/symbol results) and `.skeleton` (structure-only) files. This enables consistent cross-language golden testing across `cases/` subdirectories, loaded by `tests/python.rs`.

**• Architecture:**  
- **Golden file testing pattern**: Follows a uniform "source + expected output" strategy across all languages, ensuring parser reliability via snapshot-style tests. The `parsers/python.rs` module uses these to validate Tree-sitter parsing against pre-computed `.json` (AST/symbols) and `.skeleton` (minified structure) baselines.
- **Fits into layered AST architecture**: Part of the `ast/treesitter/parsers/tests/` testing layer, which validates the parsing layer (`parsers/*.rs`) before feeding into higher layers like indexing (`ast_indexer_thread.rs`), @-commands (`at_ast_*`), and tools (`tool_ast_definition.rs`).
- **Design patterns**: Test-Driven Development (TDD) with golden files; language-agnostic test harness in `tests/*.rs` modules that discovers and runs cases dynamically.

**• Key Symbols:**  
- No runtime symbols (pure test data), but tests validate parser outputs for Python-specific Tree-sitter nodes like `function_definition`, `class_definition`, `arguments`, `parameters`, `async_function_definition`.
- Loaded by test functions in `tests/python.rs`, which likely call `parsers/python.rs` entrypoints like `parse_file()` or `extract_declarations()` and assert against `.json`/`.skeleton`.

**• Integration:**  
- **Used by**: `tests/python.rs` (test runner); indirectly supports `at_ast_definition.rs`, `at_ast_reference.rs`, `tool_ast_definition.rs`, and `ast_db.rs` by ensuring parser correctness.
- **Uses**: Tree-sitter Python grammar (via `parsers/python.rs`); core AST structs from `treesitter/structs.rs` and `ast_structs.rs`.
- **Relationships**: Mirrors sibling dirs (e.g., `java/`, `js/`), enabling unified test suite execution. Feeds into LSP handlers (`http/routers/v1/ast.rs`) for features like `/ast` endpoints. Part of broader AST pipeline: raw parse → symbol extraction → indexing → agent tools.
- **This builds upon**: Multi-language parser validation pattern seen in documented `java/` and `js/` cases, extending it to Python for comprehensive language coverage in refact-agent's AST system. Unlike generic tests, these focus on Python idioms (e.g., decorators, type hints, f-strings) critical for accurate code intelligence.
