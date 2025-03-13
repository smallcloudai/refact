---
title: GitLab Tool
description: Configure and use GitLab integration
---

The GitLab Tool integration allows interaction with GitLab repositories using the GitLab CLI. This integration supports various GitLab operations, such as creating issues, managing merge requests, and more.

## Basic Configurations

### Glab Token
- The tool requires a GitLab Personal Access Token for authentication
- You can generate a token directly from your GitLab account
- To enhance security, store the token in a secrets.yaml file and reference it with `$MY_SECRET_VARIABLE`

### Actions
- Use the Test button to verify if the GitLab integration is functioning correctly

## Advanced Configuration

### Glab Binary Path
- Specifies the path to the GitLab CLI binary (glab)
- Leave this field empty to use the default glab command if it is available in your system's PATH
- On Windows, if you experience issues, install glab via Chocolatey, Winget, or from the official GitLab website

### Confirmation Rules
Define command patterns to control execution and safeguard critical operations:

#### Ask User
Commands requiring confirmation, example:
- `glab * delete *`: Prompts confirmation for delete operations

#### Deny
Commands to block, example:
- `glab auth token *`: Blocks authentication token commands for security