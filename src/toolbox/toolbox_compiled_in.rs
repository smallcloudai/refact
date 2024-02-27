pub const COMPILED_IN_CUSTOMIZATION_YAML : &str = r#"
# Customization will merge this compiled-in config and the user config.
#
# There are magic keys:
#    %ARGS%
#       expanded to arguments of a toolbox command, like this /command <ARGS>
#    %CODE_SELECTION%
#       plain text code that user has selected
#    %CURRENT_FILE_PATH_COLON_CURSOR%
#       expanded to file.ext:42
#       useful to form a "@file xxx" command that will insert the file text around the cursor
#
# You can also use top-level keys to reduce copy-paste, like you see there with DEFAULT_PROMPT.

DEFAULT_PROMPT: "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response."

system_prompts:
  default:
    text: "%DEFAULT_PROMPT%"

toolbox_commands:
  shorter:
    selection_needed: [1, 50]
    description: "Make code shorter"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nMake this specific code block shorter:\n\n```\n%CODE_SELECTION%```\n"
  bugs:
    selection_needed: [1, 50]
    description: "Find and fix bugs"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nFind and fix bugs in this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  improve:
    selection_needed: [1, 50]
    description: "Rewrite this specific code block of code to improve it"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block of code to improve it:\n\n```\n%CODE_SELECTION%```\n"
  comment:
    selection_needed: [1, 50]
    description: "Comment each line"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nComment each line of this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  typehints:
    selection_needed: [1, 50]
    description: "Add type hints"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nAdd type hints to this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  naming:
    selection_needed: [1, 50]
    description: "Improve variable names"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nImprove variable names in this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  explain:
    selection_needed: [1, 50]
    description: "Explain code"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nExplain this specific code block:\n\n```\n%CODE_SELECTION%```\n"
  summarize:
    selection_needed: [1, 50]
    description: "Summarize code in 1 paragraph"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nSummarize this specific code block in 1 paragraph:\n\n```\n%CODE_SELECTION%```\n"
  typos:
    selection_needed: [1, 50]
    description: "Fix typos"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block to fix typos, especially inside strings and comments:\n\n```\n%CODE_SELECTION%```\n"
"#;


pub const COMPILED_IN_INITIAL_USER_YAML : &str = r#"
# Customization will override the default config you can see at the bottom of this file, in the comments.
# You can find the default config by searching for COMPILED_IN_CUSTOMIZATION_YAML in `refact-lsp` repo.
# If your custom command is good, you can post a PR changing the default for everybody.
#
# It's easy, just make your commands by analogy and experiment!

system_prompts:
  write_pseudo_code:
    description: "User-defined: write pseudo code"
    text: "You are a programming assistant. Use backquotes for code blocks, but write pseudo code in comments instead of code. Replace real code offered by the user with pseudo code when you rewrite it."
  insert_jokes:
    description: "User-defined: write funny comments"
    text: "You are a programming assistant. Use backquotes for code blocks, but insert into comments inside code blocks funny remarks, a joke inspired by the code or play on words. For example ```\n// Hocus, pocus\ngetTheFocus();\n```."

toolbox_commands:
   user0:
    description: "User-defined: translate to horrible code"
    selection_needed: [1, 50]
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block into a very inefficient and cryptic one, but still correct:\n\n```\n%CODE_SELECTION%```\n
"#;
