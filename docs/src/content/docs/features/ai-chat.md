---
title: AI Chat
description: A reference page for AI Chat.
---

You can ask questions about your code in the integrated AI chat, and it can provide you with answers about your code or generate new code for you based on the context of your current file.

### **Context Length**
Refact analyzes the code up to a certain length to provide suggestions.
Context length depends on the plan you have chosen for your account:
- **Free**: 8000 tokens
- **Pro**: 32000 tokens

## Modes of Operation

With Refact.ai, you have access to three distinct modes that enhance your interaction with the AI chat: **Quick**, **Explore**, and **Agent**.

### Quick Mode
In **Quick Mode**, the model responds instantly without accessing external tools. This mode is ideal for rapid interactions and quick queries. You can enrich your experience with the following @-commands:

- **@web**: Convert any webpage into plain text for quick summarization and interaction. Simply use `@web` followed by the URL (e.g., `@web https://refact.ai/`) to fetch and convert the content into text that can be used within your chat.
  
- **@search**: Quickly locate similar code or text within your workspace, directory, or file. Use `@search` followed by your query (e.g., `@search create table`) to find matching content for seamless exploration and interaction.

### Explore Mode
**Explore Mode** is more advanced than Quick Mode, utilizing exploration tools to gather context about the project before answering questions. This mode automatically employs @-commands such as:

- **@definition**: Fetch definitions of symbols within the codebase.
  
- **@reference**: Locate usages of specific functions or classes throughout the project.
  
- **@tree**: View the project structure and navigate through multiple files to understand the context better.

Explore Mode allows for a deeper understanding of the codebase, enabling the model to provide more informed answers.

### Agent Mode
**Agent Mode** introduces agent capabilities that significantly enhance the way you program. While responses may take longer, this mode offers higher-quality solutions for complex challenges. Key features include:

- **Contextual Awareness**: The model can analyze and understand the broader context of your code, leading to more relevant suggestions and solutions.
  
- **Task Automation**: Agent Mode can assist in automating repetitive tasks, allowing you to focus on more critical aspects of your project.

- **Complex Problem Solving**: It is designed to tackle intricate programming challenges by leveraging its understanding of the codebase and available resources.

While we are still refining Agent Mode, it already provides valuable assistance for developers looking to enhance their productivity and code quality.


## @-commands

This section outlines various commands that can be used in the AI chat. Below you can find information about functionality and usage of each command.

![Chat Commands](../../../assets/chat-commands.png)

#### `@help`

- **Description**: Provides information about available commands and their usage.
- **Usage**: Type `@help`.

#### `@file`

- **Description**: Attaches a file to the chat.
- **Usage**: 
  - To attach a whole file, use the command followed by the file name, e.g., `@file example.ext`.
  - To specify a particular section of a file, include the line numbers, e.g., `@file large_file.ext:42` or for a range, `@file large_file.ext:42-56`.

#### `@definition`

- **Description**: Retrieves the definition of a symbol.
- **Usage**: Type `@definition` followed by the symbol name, e.g., `@definition MyClass`.

#### `@references`

- **Description**: Returns references for a symbol, including usage examples.
- **Usage**: Type `@references` followed by the symbol name, e.g., `@references MyClass`.

#### `@symbols-at`

- **Description**: Searches for and adds symbols near a specified line in a file to the chat context.
- **Usage**: Specify both the file and the line number, e.g., `@symbols-at some_file.ext:42`.

#### `@search`

- **Description**: Find similar pieces of code or text using the vector database.
- **Usage**: Type `@search` followed by your query and scope, e.g., `@search "function definition" workspace`.

#### `@tree`

- **Description**: Get a files tree with symbols for the project. Use it to get familiar with the project, file names, and symbols.
- **Usage**: Type `@tree` followed by an optional path, e.g., `@tree some_directory/`.

#### `@web`

- **Description**: Fetch a web page and convert to readable plain text.
- **Usage**: Type `@web` followed by the URL, e.g., `@web http://example.com`.

## Chat Initialization Options

Upon starting a new chat, several options are available that mimic the above commands:

- `Search workspace`: Equivalent to using `@search`. It uses the entered query to perform a search. 
- `Attach current_file.ext`: Similar to the `@file` command. It attaches the file at the current cursor position (CURSOR_LINE), useful for dealing with large files.
- `Lookup symbols`: Corresponds to the `@symbols-at` command. It extracts symbols around the cursor position and searches them in the AST index.
- `Selected N lines`: Adds the currently selected lines as a snippet for analysis or modification. This is similar to embedding code within backticks ``` in the chat.

## Enabling commands

To use @-commands in the AI chat, you need to enable specific settings:
- `@search` - enable the `Enable vector database` checkbox under the `Refactai: Vecdb` section.
- `@definition`, `@file`, `@references`, `@symbols-at` - enable the `Enable syntax parsing` checkbox under the `Refactai: Ast` section.

Read more in the [Enabling RAG Documentation](https://docs.refact.ai/features/context/).