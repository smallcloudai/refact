---
title: PostgreSQL Tool
description: Configure and use PostgreSQL database integration
---

The PostgreSQL Tool integration allows the Refact.ai Agent to interact with Postgres databases, enabling it to query, inspect data, and make changes. This tool can also integrate with Docker containers running Postgres servers.

## Basic Configurations

### Connection Settings
- **Host**: Specify the host to connect to, such as 127.0.0.1 or the name of a Docker container
- **Port**: Define the port used for the connection (Default: 5432)
- **User and Password**: Set the username and password for database access
  - Can be entered directly or referenced from environment variables (e.g., `$POSTGRES_USER` and `$POSTGRES_PASSWORD`)
- **Database**: Enter the name of the database you want the tool to connect to

### Actions
- **Test**: Verifies the connection and functionality of the Postgres integration
- **Look at the project, help me set it up**: Assists in configuring the tool by analyzing project settings

## Advanced Configuration

### PSQL Binary Path
- Specifies the path to the psql binary
- Leave this field blank if psql is available in the system's PATH
- If the binary is located elsewhere, provide the full path (e.g., `/usr/local/bin/psql`)

### Confirmation Rules
Define command patterns to control execution:
- **Ask User**: Commands requiring confirmation, example:
  - `psql*[!SELECT]*`: Prompts for confirmation for commands other than SELECT
- **Deny**: Commands matching these patterns are automatically blocked