---
title: PDB Tool
description: Configure PDB
---

## PDB Tool

The PDB Tool integration allows interaction with the Python debugger for inspecting variables and exploring program execution. It provides functionality for debugging Python scripts and applications.

### Configurations

#### Python Interpreter Path
- Specifies the path to the Python interpreter
- Leave this field empty to use the default python3 command
- If the Python executable is located in a non-standard directory, provide its full path

#### Actions
- Use the Test button to verify if the PDB integration is functioning correctly

#### Confirmation Rules
Define command patterns to control execution:
- **Ask User**: Commands matching these patterns will prompt the user for confirmation before execution
- **Deny**: Commands matching these patterns are automatically blocked