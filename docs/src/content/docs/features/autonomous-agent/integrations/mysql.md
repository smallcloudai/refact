---
title: MySQL Tool
description: Configure and use MySQL database integration
---

The MySQL Tool integration allows the AI model to interact with MySQL databases, enabling it to query, inspect data, and make changes. This tool can also integrate with Docker containers running MySQL servers.

## Basic Configurations

### Connection Settings
- **Host**: Specify the host to connect to, such as 127.0.0.1 or the name of a Docker container
- **Port**: Define the port used for the connection (Default: 3306)
- **User and Password**: Set the username and password for database access
  - Can be entered directly or referenced from environment variables (e.g., `$MYSQL_USER` and `$MYSQL_PASSWORD`)
- **Database**: Enter the name of the database you want the tool to connect to

### Actions
- **Test**: Verifies the connection and functionality of the MySQL integration
- **Look at the project, help me set it up**: Assists in configuring the tool by analyzing project settings

## Advanced Configuration

### MySQL Binary Path
- Specifies the path to the mysql binary
- Leave this field blank if mysql is available in the system's PATH
- If the binary is located elsewhere, provide the full path (e.g., `/usr/local/bin/mysql`)

### Confirmation Rules
Define command patterns to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation before execution
- **Deny**: Commands matching these patterns are automatically blocked