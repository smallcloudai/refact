---
title: Command Line Service
description: Configure and use command-line services and background processes
---

The Command-Line Service integration allows you to adapt command-line tools that run as background processes, such as web servers or daemons. This tool provides options to monitor the service's startup, specify parameters, and manage interactions with the Refact.ai Agent.

## Basic Configurations

### Command Settings
- **Command**: Specify the command to execute
  - Use `%param_name%` notation to allow dynamic parameter substitution by the model
  - Example: `python -m http.server %port%`
- **Command Workdir**: Define the working directory for the command
  - If left empty, the workspace directory will be used by default
- **Description**: Provide a description to explain the purpose of this service

### Parameters
- Add parameters the model should fill out when using the tool
- Define a Name and Description for each parameter to guide the model

### Startup Configuration
- **Startup Wait Port**: Specify a port for the tool to monitor during startup
  - The service will wait for the port to become active as an indication that the process has started successfully
- **Startup Wait**: Set the maximum time (in seconds) the tool should wait for the service to start
  - If the process doesn't start within this time, it will be terminated
- **Startup Wait Keyword**: Define a keyword to monitor in the service's output
  - The service will wait until the keyword appears to confirm the startup

### Actions
- **Test**: Runs the command to verify its functionality
- **Auto Configure**: Assists in setting up the service by analyzing the context

### Confirmation Rules
Define rules to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation
- **Deny**: Commands matching these patterns are automatically blocked
  - Example: `sudo*`: Blocks commands requiring elevated privileges