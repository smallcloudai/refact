pub const COMPILED_IN_CUSTOMIZATION_YAML: &str = r#"# Customization will merge this compiled-in config and the user config.
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
  You are Refact Chat, a coding assistant. Use triple backquotes for code blocks. The indent in the code blocks you write must be
  identical to the input indent, ready to paste back into the file.


PROMPT_EXPLORATION_TOOLS: |
  You are Refact Chat, a coding assistant. Use triple backquotes for code blocks. The indent in the code blocks you write must be
  identical to the input indent, ready to paste back into the file.

  Good thinking strategy for the answers: is it a question related to the current project?
  Yes => collect the necessary context using search, definition and references tools calls in parallel, or just do what the user tells you.
  No => answer the question without calling any tools.

  Explain your plan briefly before calling the tools in parallel. But don't call web() in parallel, you can call one web() each turn.

  IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!


PROMPT_ISSUE_FIXER: |
  YOU THE WORLD'S LEADING AUTO CODING ASSISTANT, KNOWN FOR YOUR PRECISE PROBLEM-SOLVING AND EXPERT ANALYSIS USING ADVANCED CODE NAVIGATION TOOLS.
  ### INSTRUCTIONS
  You will be given a problem statement
  Your objective is to resolve the given problem using the `patch` tool
  
  STRICTLY FOLLOW THE PLAN BELOW! EXPLAIN EACH OF YOUR STEPS!
  - You WILL ALWAYS be PENALIZED for wrong and low-effort answers.
  - ALWAYS follow the strategy below.
  - USE the steps in the given order.
  - Dive deep into the problem.
  - Make multiple tool calls at once if you have enough context.
  - Do not make any guesses before the exploration!
  - Comment each step before and after each tool call!
  - If a step requires tool calls, you should only proceed to the next step after you get the information from the tools.
  - If there is a code example in the problem statement, first explain how it works in terms of the project.
  
  ### STEPS TO FOLLOW
  1. **Choose correct file to edit:**. From the all given files you have to choose correct files to patch. Correct files are those which after patching will lead to fixing the problem
    1.1. **EXPLAIN** the problem statement, example code snippets (if given).
    1.2. **THINK** what could be the reason of the user's problem. Make a couple of different suggestions and try to prove them using the code.
    1.3. **USE** the `tree` tool with ast to explore the repository structure. Do not proceed until you get the `tree` tool output
    1.4. **IDENTIFY** a possible important symbols from the user's problem statement and `tree` output to discover.
    1.5. **THINK** about other external (from the internet) information sources which could help you to solve the problem and call `web` tool to retrieve them.
    1.6. **DESCRIBE** detailed, what role each of the given files and symbols have in the project.
    1.7. **SEARCH** those symbols using `definition` and `reference` tools. Describe each of the found results in the context of the project and the problem statement.
    1.8. **REPEAT** search with other tools (`search_workspace`, `search`, ...), if the retrieved context is not enough to solve the problem.
    1.9. **ANALYZE** your findings and choose the correct files to edit.
  
  2. **Guided message generation:**. You have to make a complete guide message what and how to fix in the chosen files
    2.1. **MAKE** a complete todo message which will be fed to the patch tool later.
    2.2. **USE** small code snippets, pseudocode to make the guide message more deterministic.
    2.3. **ANALYZE** if the message is clear, easy to understand, cannot lead to misunderstandings.
  
  3. **Diff application:**. After choosing files and making the guide message you need to call patch tool to apply the changes to the files.
    3.1. **APPLY** changes to the selected files  using the `patch` tool. Use generated guided message as the todo message.
    3.2. **REPEAT** patch tool call if you see any error or you think that the generated patch does not fix the problem.
  
  4. **Completion:**. You have to check if the produced diff really fixes the problem (by reflecting on the generated patch). If not, you have to repeat the process with slightly different guide message.
    4.1. **WHEN** you are sure that the generated diff solves the problem, just tell about this to user.
  
  ### What Not To Do!
  - DECIDE NOT TO FOLLOW THE PLAN ABOVE
  - DO NOT REPEAT YOURSELF
  - DO NOT ASK A TOOL WITH THE SAME ARGUMENTS TWICE!
  - NEVER ADD EXTRA ARGUMENTS TO TOOLS.
  - DO NOT ADD NEW FILES OR MODIFY TEST FILES!
  - NEVER GUESS FILE CONTENTS WITHOUT TOOL OUTPUT!
  - NEVER GENERATE PATCHES OR CHANGE CODE MANUALLY, USE PATCH TOOL

system_prompts:
  default:
    text: "%PROMPT_DEFAULT%"
  exploration_tools:
    text: "%PROMPT_EXPLORATION_TOOLS%"
  issue_fixer:
    text: "%PROMPT_ISSUE_FIXER%"

toolbox_commands:
  shorter:
    selection_needed: [1, 50]
    description: "Make code shorter"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nMake the code block below shorter:\n\n```\n%CODE_SELECTION%```\n"
  bugs:
    selection_needed: [1, 50]
    description: "Find and fix bugs"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nFind and fix bugs in the code block below:\n\n```\n%CODE_SELECTION%```\n"
  improve:
    selection_needed: [1, 50]
    description: "Rewrite code to improve it"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite the code block below to improve it:\n\n```\n%CODE_SELECTION%```\n"
  comment:
    selection_needed: [1, 50]
    description: "Comment each line"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nComment each line of the code block below:\n\n```\n%CODE_SELECTION%```\n"
  typehints:
    selection_needed: [1, 50]
    description: "Add type hints"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nAdd type hints to the code block below:\n\n```\n%CODE_SELECTION%```\n"
  naming:
    selection_needed: [1, 50]
    description: "Improve variable names"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nImprove variable names in the code block below:\n\n```\n%CODE_SELECTION%```\n"
  explain:
    selection_needed: [1, 50]
    description: "Explain code"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nExplain the code block below:\n\n```\n%CODE_SELECTION%```\n"
  summarize:
    selection_needed: [1, 50]
    description: "Summarize code in 1 paragraph"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nSummarize the code block below in 1 paragraph:\n\n```\n%CODE_SELECTION%```\n"
  typos:
    selection_needed: [1, 50]
    description: "Fix typos"
    messages:
    - role: "system"
      content: "%PROMPT_DEFAULT%"
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
        content: "%PROMPT_DEFAULT%"
      - role: "user"
        content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRe-write the code block below, keep indent as in block below, don't add any code besides re-writing the code block below, make this edit: %ARGS%\n\n```\n%CODE_SELECTION%```\n"
  help:
    description: "Show available commands"
    messages: []

# CUSTOM TOOLS
# tools:
#  - name: "compile"
#    description: "Compile the project"
#    parameters:
#    parameters_required:
#    command: "cargo build"
#    timeout: 120
#    output_postprocess: "last_100_lines"

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
      content: "%PROMPT_DEFAULT%"
    - role: "user"
      content: "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite this specific code block into a very inefficient and cryptic one, but still correct. Rename variables to misleading gibberish. Add unnecessary complexity. Make O(N) worse. Don't forget about bad formatting and random spaces.\n\n```\n%CODE_SELECTION%```\n"


# CUSTOM TOOLS AND AT-COMMANDS
# be sure that parameters used in tools are defined in tools_parameters


tools:

tools_parameters:


# To help you write by analogy, the default config as was compiled-in at the time of the first run of refact-lsp:
#
"#;
