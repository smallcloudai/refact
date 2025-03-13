---
title: AI Toolbox
description: A page about functions in the Refact AI Toolbox
---

Refact.ai Toolbox is a set of functions that can be used to 
refactor, analyze, and improve your code.

The Toolbox can be accessed by pressing `Alt+T`. It will open a modal 
window associated with a specific line of code or a block of code.

At the moment, AI Toolbox only works with VS Code.

The following commands are available in the Toolbox:
- `/bugs` - Find and fix bugs in your code
- `/comment` - Comment each line
- `/edit` - Edit code. You need to write instructions after the command
- `/explain` - Explain code
- `/gen` - Create new code. You need to add a description after the command
- `/improve` - Rewrite this specific code block to improve it
- `/naming` - Improve variables, functions, classes, etc.
- `/shorter` - Make code shorter
- `/summarize` - Summarize code in one paragraph
- `/typehints` - Add type hints
- `/typos` - Find and fix typos in code


## Custom toolbox commands

You can also create your own toolbox commands. To do that, you need to press 
`Alt+T` and type `/help` in the Refact.ai console. At the bottom, you will see a 
button called `Customize toolbox`. Press it and the `customization.yaml` 
file will open.

![Refact Toolbox](../../../assets/toolbox.png)

In the file, you will find a `toolbox_commands` section. You can add your
custom commands there.

All of the newly created commands should follow the same format:

```yaml
description: "User-defined: translate to horrible code"
    selection_needed: [1, 50]
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite this specific code block into a very inefficient and cryptic one, but still correct. Rename variables to misleading gibberish. Add unnecessary complexity. Make O(N) worse. Don't forget about bad formatting and random spaces.\n\n```\n%CODE_SELECTION%```\n"
```

To break it down, you need to provide the following information:
- `description` - a user-defined description of the command
- `selection_needed` - a list of line numbers that the command is valid for
- `content` - Inside the `message` for the `user` role, the exact prompt that will be used when the command is executed

Once you finish, save the file, and your custom toolbox command will be available.
When entering the `/help` command, you will see your custom command in the list of available commands.


![Refact Toolbox](../../../assets/custom_command.png)

:::note
When using toolbox commands without the `/` symbol, the command will be executed in the chat interface. Also, the command will be executed in the toolbox window.
:::
