pub const COMPILED_IN_CUSTOMIZATION_YAML : &str = r#"# Customization will merge this compiled-in config and the user config.
#
# There are magic keys:
#    %ARGS%
#       expanded to arguments of a toolbox command, like this /command <ARGS>
#    %CODE_SELECTION%
#       plain text code that user has selected
#    %CURRENT_FILE%:%CURSOR_LINE%
#       expanded to file.ext:42
#       useful to form a "@file xxx" command that will insert the file text around the cursor
#
# You can also use top-level keys to reduce copy-paste, like you see there with DEFAULT_PROMPT.


DEFAULT_PROMPT: |
  Use backquotes for code blocks.
  Pay close attention to indent when editing code blocks: indent must be exactly the same as in the original code block.
  Write math expressions in a markdown style: $x^2$ when inside line; $$x^2$$ when in a new line;


DEFAULT_PROMPT_TOOLBOX: |
  You are a search agent. You need to actively search for the answer yourself, don't ask the user to do anything. The answer is most likely in the files and databases accessible using tool calls, not on the internet.

  When responding to a query, first provide a very brief explanation of your plan to use tools in parallel to answer the question, and then make several tool calls to gather more details.

  Minimize the number of steps, call up to 15 tools in parallel when exploring (ls, cat, search, definition, references, etc). Use only one tool when executing (run, compile, docker).

  IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST!


  Example 1

  User: "What is the weather like today in Paris and London?"
  Assistant: "Must be sunny in Paris and foggy in London."
  User: "don't hallucinate, use the tools"
  Assistant: "Sorry for the confusion, you are right, weather is real-time, and my best shot is to use the weather tool. I will use 2 calls in parallel."
  [Call weather "London"]
  [Call weather "Paris"]


  Example 2

  User: "What is MyClass"
  Assistant: "Let me find it first."
  [Call ls "."]
  Tool: folder1, folder2, folder3
  Assistant: "I see 3 folders, will make 3 calls in parallel to check what's inside."
  [Call ls "folder1"]
  [Call ls "folder2"]
  [Call ls "folder3"]
  Tool: ...
  Tool: ...
  Tool: ...
  Assistant: "I give up, I can't find a file relevant for MyClass 😕"
  User: "Look, it's my_class.cpp"
  Assistant: "Sorry for the confusion, there is in fact a file named `my_class.cpp` in `folder2` that must be relevant for MyClass."
  [Call cat "folder2/my_class.cpp"]
  Tool: ...
  Assistant: "MyClass does this and this"

NOTE_TO_SELF: |
  How many times user has corrected or directed you? Write "Number of correction points N".
  Then start each one with "---\n", describe what you (the assistant) did wrong, write "Mistake: ..."
  Write documentation to tools or the project in general that will help you next time, describe in detail how tools work, or what the project consists of, write "Documentation: ..."
  A good documentation for a tool describes what is it for, how it helps to answer user's question, what applicability criteia were discovered, what parameters work and how it will help the user.
  A good documentation for a project describes what folders, files are there, summarization of each file, classes. Start documentation for the project with project name.
  After describing all points, call note_to_self() in parallel for each actionable point, generate keywords that should include the relevant tools, specific files, dirs, and put documentation-like paragraphs into text.


system_prompts:
  default:
    text: "%DEFAULT_PROMPT%"
  default_tool:
    text: "%DEFAULT_PROMPT_TOOLBOX%"
  note_to_self:
    text: "%NOTE_TO_SELF%"

toolbox_commands:
  shorter:
    selection_needed: [1, 50]
    description: "Make code shorter"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nMake the code block below shorter:\n\n```\n%CODE_SELECTION%```\n"
  bugs:
    selection_needed: [1, 50]
    description: "Find and fix bugs"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nFind and fix bugs in the code block below:\n\n```\n%CODE_SELECTION%```\n"
  improve:
    selection_needed: [1, 50]
    description: "Rewrite code to improve it"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite the code block below to improve it:\n\n```\n%CODE_SELECTION%```\n"
  comment:
    selection_needed: [1, 50]
    description: "Comment each line"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nComment each line of the code block below:\n\n```\n%CODE_SELECTION%```\n"
  typehints:
    selection_needed: [1, 50]
    description: "Add type hints"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nAdd type hints to the code block below:\n\n```\n%CODE_SELECTION%```\n"
  naming:
    selection_needed: [1, 50]
    description: "Improve variable names"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nImprove variable names in the code block below:\n\n```\n%CODE_SELECTION%```\n"
  explain:
    selection_needed: [1, 50]
    description: "Explain code"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nExplain the code block below:\n\n```\n%CODE_SELECTION%```\n"
  summarize:
    selection_needed: [1, 50]
    description: "Summarize code in 1 paragraph"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nSummarize the code block below in 1 paragraph:\n\n```\n%CODE_SELECTION%```\n"
  typos:
    selection_needed: [1, 50]
    description: "Fix typos"
    messages:
    - role: "system"
      content: "%DEFAULT_PROMPT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite the code block below to fix typos, especially inside strings and comments:\n\n```\n%CODE_SELECTION%```\n"
  gen:
    selection_unwanted: true
    insert_at_cursor: true
    description: "Create new code, provide a description after the command"
    messages:
      - role: "system"
        content: "You are a fill-in-the middle model, analyze suffix and prefix, generate code that goes exactly between suffix and prefix. Never rewrite existing code. Watch indent level carefully. Never fix anything outside of your generated code. Stop after writing just one thing."
      - role: "user"
        content: "@file %CURRENT_FILE%:%CURSOR_LINE%-\n"
      - role: "user"
        content: "@file %CURRENT_FILE%:-%CURSOR_LINE%\n"
      - role: "user"
        content: "%ARGS%"
  edit:
    selection_needed: [1, 50]
    description: "Edit code, write instruction after the command"
    messages:
      - role: "system"
        content: "%DEFAULT_PROMPT%"
      - role: "user"
        content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRe-write the code block below, keep indent as in block below, don't add any code besides re-writing the code block below, make this edit: %ARGS%\n\n```\n%CODE_SELECTION%```\n"
  help:
    description: "Show available commands"
    messages: []

"#;


pub const COMPILED_IN_INITIAL_USER_YAML : &str = r#"# Customization will override the default config you can see at the bottom of this file, in the comments.
# You can find the default config by searching for COMPILED_IN_CUSTOMIZATION_YAML in `refact-lsp` repo.
# If your custom toolbox command is good and helps you a lot, you can post a PR changing the default for everybody.
#
# It's easy, just make your toolbox commands and system prompts by analogy and experiment!
#

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
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite this specific code block into a very inefficient and cryptic one, but still correct. Rename variables to misleading gibberish. Add unnecessary complexity. Make O(N) worse. Don't forget about bad formatting and random spaces.\n\n```\n%CODE_SELECTION%```\n"



# To help you write by analogy, the default config as was compiled-in at the time of the first run of refact-lsp:
#
"#;
