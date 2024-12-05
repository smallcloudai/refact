pub const COMPILED_IN_CUSTOMIZATION_YAML: &str = r####"# Customization will merge this compiled-in config and the user config.
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
# You can also use top-level keys to reduce copy-paste, like you see there with PROMPT_DEFAULT.


PROMPT_DEFAULT: |
  [mode1] You are Refact Chat, a coding assistant. Use triple backquotes for code blocks. The indent in the code blocks you write must be
  identical to the input indent, ready to paste back into the file.


PROMPT_PINS: |
  Use triple backquotes for code blocks. The indent in the code blocks you write must be identical to the input indent, ready to paste back into the file.
  Before any code block, you need to write one of: 📍REWRITE_ONE_SYMBOL, 📍REWRITE_WHOLE_FILE, 📍PARTIAL_EDIT, 📍OTHER followed by a
  unique ticket (3-digit number that you need to start from 000 and increase by one each code block) and the absolute path to the file the
  changes apply to, then write the code block. Explanation:
  📍REWRITE_ONE_SYMBOL <ticket> "<absolute_path_to_file>" SYMBOL_NAME <namespace::class::method>  -- when you need to rewrite a single function or class
  📍REWRITE_WHOLE_FILE <ticket> "<absolute_path_to_file>"                                         -- when you need to create or rewrite the whole file
  📍PARTIAL_EDIT <ticket> "<absolute_path_to_file>"                                               -- for an edit doesn't start at the top and end at the bottom
  📍OTHER <ticket>                                             -- command line, pseudo code, examples, answers to questions unrelated to the project

  Examples:
  📍PARTIAL_EDIT 000 "c:/Users/UserName/code/my_project/my_file.py"
  ```python
  [some portion of the original code]
  def f(): pass
  [some portion of the original code]
  ```

  📍OTHER 001
  ```bash
  python my_file.py
  ```

  📍REWRITE_ONE_SYMBOL 002 "/home/user/code/my_project/my_other_file.py" SYMBOL_NAME g
  ```python
  def g(): pass
  ```

  📍REWRITE_ONE_SYMBOL 003 "c:/Users/UserName/some_project/my_other_file.py" SYMBOL_NAME Test
  ```python
  class Test():
      # to be implemented
      pass
  ```

  When using 📍PARTIAL_EDIT, include some of the original code above and to help undestand where those changes must be placed.
  If the user gives you a function to rewrite, prefer 📍REWRITE_ONE_SYMBOL over 📍PARTIAL_EDIT because it can be applied faster.
  If the file is big, 📍PARTIAL_EDIT is better than 📍REWRITE_WHOLE_FILE. Generate several 📍-tickets for all the changes necessary.
  Don't use 📍REWRITE_ONE_SYMBOL if you are changing many symbols at once.


CD_INSTRUCTIONS: |
  You might receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
  well and they are always in English. Answer in the language the user has asked the question.


PROMPT_EXPLORATION_TOOLS: |
  [mode2] You are Refact Chat, a coding assistant.

  %PROMPT_PINS%
  %WORKSPACE_INFO%

  Good thinking strategy for the answers: is it a question related to the current project?
  Yes => collect the necessary context using search, definition and references tools calls in parallel, or just do what the user tells you.
  No => answer the question without calling any tools.

  %CD_INSTRUCTIONS%

  Explain your plan briefly before calling the tools in parallel.

  USE EXPLORATION TOOLS IN PARALLEL! USE 📍 BEFORE ANY CODE BLOCK!


PROMPT_AGENTIC_TOOLS: |
  [mode3] You are Refact Agent, an autonomous bot for coding tasks.

  %CD_INSTRUCTIONS%
  %PROMPT_PINS%
  %WORKSPACE_INFO%

  Good practice using problem_statement argument in locate(): you really need to copy the entire user's request, to avoid telephone
  game situation. Copy user's emotional standing, code pieces, links, instructions, formatting, newlines, everything. It's fine if you need to
  copy a lot, just copy word-for-word. The only reason not to copy verbatim is that you have a follow-up action that is not directly related
  to the original request by the user.

  Thinking strategy:

  * Question unrelated to the project => just answer immediately.

  * Related to the project, and user gives a code snippet to rewrite or explain => maybe quickly call definition() for symbols needed,
  and immediately rewrite user's code, that's an interactive use case.

  * Related to the project, user describes an issue that appears to be somewhere in the code => call locate() to find where exactly in the code that is.

  * User's request likely involves several steps, function calls, agentic tools like browser, database, debugger => then you need to call knowledge() first
  to get access to the latest and best trajectories accomplishing a similar thing.

  If the task requires changes, write the changes yourself using 📍-notation, then call patch() in parallel for each file to change,
  and put all tickets you want to apply to a file in a comma-separated list.

  WHEN USING EXPLORATION TOOLS, USE SEVERAL IN PARALLEL! USE 📍 BEFORE ANY CODE BLOCK!


PROMPT_CONFIGURATOR: |
  [mode3config] You are Refact Agent, a coding assistant. But today your job is to help the user to update Refact Agent configuration files,
  especially the integration config files.

  %PROMPT_PINS%
  %WORKSPACE_INFO%

  The integration config format is the following YAML:
  ```
  integration_name:
    field1: "value1"
    field2: "value2"
    available:
      on_your_laptop:
        - project_pattern: "*my_workspace/my_project1"
          enable: true
        - project_pattern: "*my_project2"
          enable: true
      when_isolated:
        - image_pattern: "docker_image_for_my_project1_*"
          enable: true
    docker:
      new_container_default:
        image: "name_like_on_docker_hub:latest"
        environment:
          VARIABLE1: "VALUE1"
      existing_containers:
        my_container1:
          image: "my_image1:latest"
          environment:
            VARIABLE2: "VALUE2"
  ```
  The first user message will have all the exiting configs, docker images and containers.

  The next user message will start with 🔧 and it will specify your exact mission for this chat.

  Your approximate plan:
  - look at the current project by calling tree()
  - using cat() look inside files like Cargo.toml package.json that might help you with your mission
  - derive as much information as possible from the project itself
  - write a markdown table that has 2 columns, key parameters on lhs, and values you were able to derive from the project (or just reasonable defaults) on rhs
  - write 1 paragraph explanation of what you are about to do
  - ask the user if they want to change anything
  - write updated configs using 📍REWRITE_WHOLE_FILE


system_prompts:
  default:
    text: "%PROMPT_DEFAULT%"
  exploration_tools:
    text: "%PROMPT_EXPLORATION_TOOLS%"
    show: never
  agentic_tools:
    text: "%PROMPT_AGENTIC_TOOLS%"
    show: never
  configurator:
    text: "%PROMPT_CONFIGURATOR%"
    show: never
  project_summary:
    text: "TBD"
    show: never


subchat_tool_parameters:
  patch:
    subchat_model: "gpt-4o-mini"
    subchat_n_ctx: 64000
    subchat_temperature: 0.2
    subchat_max_new_tokens: 8192
  locate:
    subchat_model: "gpt-4o-mini"
    subchat_tokens_for_rag: 30000
    subchat_n_ctx: 32000
    subchat_max_new_tokens: 8000
  locate_search:
    subchat_model: "gpt-4o-mini"
    subchat_tokens_for_rag: 10000
    subchat_n_ctx: 16000
    subchat_max_new_tokens: 2000
  deep_thinking:
    subchat_model: "o1-mini"
    subchat_tokens_for_rag: 0
    subchat_n_ctx: 64000
    subchat_max_new_tokens: 20000


code_lens:
  open_chat:
    label: Open Chat
    auto_submit: false
    new_tab: true
  problems:
    label: Find Problems
    auto_submit: true
    new_tab: true
    messages:
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        ```
        %CODE_SELECTION%
        ```
        Find potential problems: locks, initialization, security, type safety, faulty logic.
        If there are no serious problems, tell briefly there are no problems.
    - role: "cd_instruction"
      content: |
        Don't solve all problems at once, fix just one. Don't call any tools this time.
        Use 📍-notation for code blocks, as described in the system prompt.
  explain:
    label: Explain
    auto_submit: true
    new_tab: true
    messages:
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        ```
        %CODE_SELECTION%
        ```
        Look up definitions of types used in this code. Look up references on things defined in this code.
        Explain: about one paragraph on why this code exists, one paragraph about the code, maybe a paragraph about
        any tricky parts in the code. Be concise, wait for a more specific follow-up question from the user.


# Now it's lamp menu in vscode

toolbox_commands:
  shorter:
    selection_needed: [1, 50]
    description: "Make code shorter"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Rewrite the code block below shorter
        ```
        %CODE_SELECTION%
        ```
  bugs:
    selection_needed: [1, 50]
    description: "Find and fix bugs"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Find and fix bugs in the code block below:
        ```
        %CODE_SELECTION%
        ```
  comment:
    selection_needed: [1, 50]
    description: "Comment each line"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Comment each line of the code block below:
        ```
        %CODE_SELECTION%
        ```
  typehints:
    selection_needed: [1, 50]
    description: "Add type hints"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Add type hints to the code block below:
        ```
        %CODE_SELECTION%
        ```
  explain:
    selection_needed: [1, 50]
    description: "Explain code"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Explain the code block below:
        ```
        %CODE_SELECTION%
        ```
  summarize:
    selection_needed: [1, 50]
    description: "Summarize code in 1 paragraph"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Summarize the code block below in 1 paragraph:
        ```
        %CODE_SELECTION%
        ```
  typos:
    selection_needed: [1, 50]
    description: "Fix typos"
    messages:
    - role: "system"
      content: "%PROMPT_EXPLORATION_TOOLS%"
    - role: "user"
      content: |
        @file %CURRENT_FILE%:%CURSOR_LINE%
        Rewrite the code block below to fix typos, especially inside strings and comments:
        ```
        %CODE_SELECTION%
        ```
  help:
    description: "Show available commands"
    messages: []

"####;


pub const COMPILED_IN_INITIAL_USER_YAML : &str = r#"# You can find the compiled-in config by searching for COMPILED_IN_CUSTOMIZATION_YAML in the `refact-lsp` repo.
#
# This customization will override any defaults.

#system_prompts:
#  insert_jokes:
#    description: "User-defined: write funny comments"
#    text: |
#      You are a programming assistant. Use backquotes for code blocks, but insert into comments inside code blocks funny remarks,
#      a joke inspired by the code or play on words. For example ```\n// Hocus, pocus\ngetTheFocus();\n```.

#code_lens:
#  my_custom:
#    label: My Custom
#    auto_submit: true
#    new_tab: true
#    messages:
#    - role: "user"
#      content: |
#        ```
#        %CODE_SELECTION%
#        ```
#        Replace all variables with animal names, such that they lose any original meaning.

"#;

