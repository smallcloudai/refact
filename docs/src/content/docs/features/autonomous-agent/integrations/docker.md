---
title: Docker Tool
description: Configure and use Docker integration
---

The Docker Tool integration enables interaction with Docker containers, allowing the Refact.ai Agent to manage, query, and modify Docker environments. This tool supports local and remote Docker configurations.

## Basic Configurations

### Docker Settings
- **Docker CLI Path**: Specify the path to the Docker CLI executable
  - Default: `docker`
- **Label**: Define a label to identify the Docker containers managed by the tool
  - Example: `refact`

### Actions
- **Test**: Verifies the connection and functionality of the Docker integration

## Advanced Configuration

### Docker Connection
- **Docker Daemon Address**: Specify the address for connecting to the Docker daemon
  - Leave blank to use the default local daemon

### Remote Docker
Enable this option to connect to a remote Docker host using SSH:
- **SSH Host**: Specify the hostname or IP address of the remote Docker host
- **SSH User**: Define the user for the SSH connection
- **SSH Port**: Default: 22
- **SSH Identity File**: Provide the path to the SSH identity file for authentication

### Confirmation Rules
Define rules to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation
- **Deny**: Commands matching these patterns are automatically blocked
  - Examples:
    - `docker* rm *`: Blocks removal of containers
    - `docker* stop *`: Blocks stopping containers