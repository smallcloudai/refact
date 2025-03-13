---
title: Shell Tool
description: Configure and use the Shell Tool for command-line operations
---

The Shell Tool allows the Refact.ai Agent to execute any command-line tool with user confirmation directly from the chat interface. This integration is ideal for automating tasks while ensuring safety and user control.

## Basic Configurations

### Timeout
- Specifies the maximum time (in seconds) that a command is allowed to run
- If the command exceeds this time, it is automatically terminated
- Any output (stdout/stderr) is collected and presented to the user

### Confirmation Rules
Provides a safety mechanism for executing potentially destructive or sensitive commands:

#### Ask User
- Commands matching patterns in this list will prompt the user for confirmation before execution
- By default, the wildcard (*) matches all commands

#### Deny
- Commands matching patterns in this list will be automatically blocked
- Example: `sudo*` blocks commands requiring elevated privileges

Users can add or remove rows to customize these rules according to their preferences and security requirements.

## Advanced Configuration Options

### Output Filter
Controls how the output of executed commands is processed and displayed:

#### Basic Limits
- **Limit Lines**: Restricts the output to a specified number of lines (e.g., 100)
- **Limit Characters**: Restricts the output to a maximum number of characters (e.g., 10,000)

#### Output Processing
- **Valuable Top or Bottom**: Determines whether the tool prioritizes the start (top) or end (bottom) of the output for relevance
- **Grep**: Uses a regular expression (e.g., `(?i)error`) to filter and highlight specific content in the output
- **Grep Context Lines**: Defines the number of surrounding lines to include with matches from the grep filter
- **Remove from Output**: Allows for removing unwanted patterns or content from the displayed output

These settings help manage large or verbose outputs, focusing only on the most critical information.