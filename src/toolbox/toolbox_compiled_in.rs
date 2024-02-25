pub const COMPILED_IN_TOOLBOX_YAML : &str = r#"
# Toolbox will merge this compiled-in config and the user config.
#
# There are magic keys:
#    %ARGS%
#       expanded to arguments of a command, like this /command <ARGS>
#    %CODE_SELECTION%
#       plain text code that user has selected
#    %CODE_AROUND_CURSOR_JSON%
#       Json that has the current file, possibly cut (if it's large)
#       The json format is suitable to attach to a role="context_file" message
#    %CODE_INSERT_HERE_JSON%
#       Json that has the current file, cursor position marked with "|INSERT-HERE|" in the text
#

SYSTEM_PROMPT: "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response."

commands:
  shorter:
    selection_needed: [1, 50]
    description: "Make code shorter"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nMake this specific code block shorter:\n\n```\n%CODE_SELECTION%```\n"
  bugs:
    selection_needed: [1, 50]
    description: "Find and fix bugs"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nFind and fix bugs in this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  "improve":
    selection_needed: [1, 50]
    description: "Rewrite this specific code block of code to improve it"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block of code to improve it:\n\n```\n%CODE_SELECTION%```\n"
  comment:
    selection_needed: [1, 50]
    description: "Comment each line"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nComment each line of this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  typehints:
    selection_needed: [1, 50]
    description: "Add type hints"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nAdd type hints to this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  naming:
    selection_needed: [1, 50]
    description: "Improve variable names"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nImprove variable names in this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  explain:
    selection_needed: [1, 50]
    description: "Explain code"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nExplain this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  summarize:
    selection_needed: [1, 50]
    description: "Summarize code in 1 paragraph"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nSummarize this specific code block in 1 paragraph:\n\n```\n%CODE_SELECTION%```\n"
  typos:
    selection_needed: [1, 50]
    description: "Fix typos"
    messages:
    - role: "system"
      content: "%SYSTEM_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block to fix typos, especially inside strings and comments:\n\n```\n%CODE_SELECTION%```\n"
"#;
