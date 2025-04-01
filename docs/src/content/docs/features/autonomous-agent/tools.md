---
title: Agent Tools
description: Overview of available tools and use cases for the autonomous Refact.ai Agent
---

The Refact.ai Agent is designed to operate autonomously, extending its capabilities beyond simple integrations. While integrations allow external services to be connected, Agent Tools empower the agent to perform key operations using built-in functionalities. Below is an overview of the available Agent Tools, typical use cases, and how they can enhance your development workflow.

## Core Tools

The Agent has access to several powerful tools that help it understand and modify your codebase:

### Context and Search Tools

- **search**  
  Find similar pieces of code or text using vector database.  
  *Use Case:* When you ask the Agent to modify code, it uses this tool to find similar patterns across your codebase to maintain consistency.

- **definition**  
  Read definition of symbols in the project using AST.  
  *Use Case:* The Agent uses this to understand function signatures, class structures, and type definitions when working with your code.

- **references**  
  Find usages of symbols within the project using AST.  
  *Use Case:* Before modifying a function or class, the Agent checks all its usages to ensure changes won't break existing code.

- **tree**  
  Get a files tree with symbols for the project.  
  *Use Case:* The Agent uses this to understand project structure and locate relevant files.

### File Operations

- **cat**  
  Read multiple files and understand their content.  
  *Use Case:* The Agent can read and analyze multiple files at once, including images and skeletonized code views.

- **locate**  
  Find relevant files for a specific task.  
  *Use Case:* When given a task, the Agent can quickly identify which files need to be modified.

### Code Modification

- **patch**  
  Apply changes to files in a controlled manner.  
  *Use Case:* The Agent uses this to make actual changes to your codebase, with your approval.

### Planning and Analysis

- **think**  
  Analyze complex problems and create execution plans using o3 mini reasoning model.  
  *Use Case:* Before making changes, the Agent plans out the steps needed to complete a task successfully. The o3 mini model helps break down complex problems into manageable steps and create a clear execution strategy.

### Web Interaction

- **web**  
  Fetch and read web pages in plain text format.  
  *Use Case:* The Agent can read documentation, specifications, or other web resources to help solve problems.

## How Agent Tools Work Together

The Agent combines these tools strategically to complete complex tasks. Here's a typical workflow:

1. **Understanding Phase**
   - Uses `tree` to understand project structure
   - Uses `locate` to find relevant files
   - Uses `search` to find similar patterns
   - Uses `definition` and `references` to understand code context

2. **Planning Phase**
   - Uses `think` to create a detailed plan
   - Uses `web` if external documentation is needed

3. **Execution Phase**
   - Uses `cat` to read necessary files
   - Creates changes using `patch` tool
   - Verifies changes using `search` and `references`

## Best Practices

When working with the Agent, consider these tips:

- Let the Agent gather context before making changes
- Review proposed patches carefully before approving
- Use the Agent's planning capabilities for complex tasks
- Provide clear, specific instructions for best results

## Next Steps

Once you're familiar with the core tools, you might want to explore:

- [Agent Overview](../overview) - Learn more about the Agent's capabilities
- [Getting Started](../getting-started) - Start using the Agent effectively
- [Integrations](../integrations) - Connect with external services and tools

For specific integration guides and advanced usage scenarios, refer to our detailed documentation sections.