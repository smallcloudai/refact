---
title: GitHub Tool
description: Configure and use GitHub integration
---

The GitHub Tool integration enables interaction with GitHub repositories through the GitHub CLI. This integration supports various GitHub operations, including creating issues, managing pull requests, and more.

## Basic Configurations

### Scope of Configuration
You can configure the GitHub integration for:
- **Global**: Makes the integration available across all projects
- **Project-Specific**: Limits the integration to a single project, allowing for customized settings

### Personal Access Token
- The tool requires a GitHub Personal Access Token for authentication
- You can create a token directly from your GitHub account
- To enhance security, you can store the token in a secrets.yaml file and reference it with `$MY_SECRET_VARIABLE`

### Continue Setup
After choosing the scope and providing the token, proceed with the setup to finalize the integration.

## Advanced Configuration

### Actions
- Use the Test button to verify if the GitHub integration is functioning correctly

### Confirmation Rules
Define command patterns to control execution and safeguard critical operations:

#### Ask User
Commands requiring confirmation, examples:
- `gh * delete *`: Prompts confirmation for delete operations
- `gh * close *`: Prompts confirmation for closing issues or pull requests

#### Deny
Commands to block, example:
- `gh auth token *`: Blocks authentication token commands for security

### Token Management
- If the token is stored as an environment variable, ensure it is referenced correctly
- The system uses this token to authenticate GitHub operations without directly exposing it