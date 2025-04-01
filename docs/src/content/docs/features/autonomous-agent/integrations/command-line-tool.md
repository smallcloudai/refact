---
title: Command Line Tool
description: Configure and use custom command-line tools
---

The Command-Line Tool integration allows you to add and adapt any command-line tool for use by the Refact.ai Agent. This tool can be configured with specific parameters, commands, and restrictions, enabling tailored functionality for your workflows.

## Basic Configurations

### Command Settings
- **Command**: Specify the command to execute
  - Use `%param_name%` notation to allow dynamic parameter substitution by the model
  - Example: `echo %message%`
- **Command Workdir**: Define the working directory for the command
  - If left empty, the workspace directory will be used by default
- **Description**: Provide a description to explain the purpose of this command
  - This helps the Refact.ai Agent understand when and why to use the tool

### Parameters
- Add parameters the model should fill out when using the tool
- Define a Name and Description for each parameter to guide the model

### Timeout
- Set the maximum time (in seconds) the command is allowed to run
- If the command exceeds this duration, it will be terminated, and its output (stdout/stderr) will be returned

### Actions
- **Test**: Runs the command to verify its functionality
- **Auto Configure**: Assists in setting up the command by analyzing the context and suggesting configurations

## Advanced Configuration

### Output Filter
Manage how the command's output is processed and displayed:
- Limit the number of lines or characters in the output
- Prioritize output from the start (top) or end (bottom)
- Use regular expressions (regex) to extract relevant portions of the output
- Remove unwanted parts of the output for cleaner results

### Confirmation Rules
Define rules to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation before execution
- **Deny**: Commands matching these patterns are automatically blocked
  - Example: `sudo*`: Blocks commands requiring elevated privileges