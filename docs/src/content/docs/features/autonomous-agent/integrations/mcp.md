---
title: MCP Server
description: Configure and use Model Context Protocol server integration
---

The MCP (Model Context Protocol) Server integration enables the Refact.ai Agent to connect to local or remote MCP servers, enhancing the agent's context understanding and capabilities. This integration supports various MCP server implementations and configurations.

## What is MCP?

MCP (Model Context Protocol) is a protocol designed to provide structured context to language models. It allows for standardized communication between applications and language models, enabling more effective context management and improved model responses.

Learn more about MCP at [Anthropic's Model Context Protocol documentation](https://www.anthropic.com/news/model-context-protocol).

## Basic Configurations

### Command Settings
- **Command**: Specify the MCP command to execute
  - Examples:
    - `npx -y <some-mcp-server>`
    - `/my/path/venv/python -m <some-mcp-server>`
    - `docker run -i --rm <some-mcp-image>`
  - On Windows, use `npx.cmd` or `npm.cmd` instead of `npx` or `npm`

### Environment Variables
- Add environment variables required by the MCP server
- Define variable names and values to configure server behavior

### Actions
- **Test**: Verifies the connection and functionality of the MCP server integration
- **Open mcp_test.yaml**: Opens the configuration file for editing

## Advanced Configuration

### Confirmation Rules
Define rules to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation
  - Use this for potentially sensitive operations
- **Deny**: Commands matching these patterns are automatically blocked
  - Use this to prevent execution of potentially harmful commands



## Security Considerations

- Always verify the source and security of MCP servers before connecting
- Use confirmation rules to prevent execution of potentially harmful commands
- Consider using environment variables to pass sensitive configuration rather than command-line arguments
- For production environments, consider setting up access controls for your MCP servers

## Troubleshooting

- If the connection fails, verify that the MCP server is running and accessible
- Check that any required environment variables are correctly set
- Ensure that the command path is correct and executable
- For Docker-based servers, verify that Docker is running and the image is available