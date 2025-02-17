import { CustomPromptsResponse, SystemPrompts } from "../services/refact";

export const SYSTEM_PROMPTS: SystemPrompts = {
  write_pseudo_code: {
    description: "User-defined: write pseudo code",
    text: "You are a programming assistant. Use backquotes for code blocks, but write pseudo code in comments instead of code. Replace real code offered by the user with pseudo code when you rewrite it.",
  },
  default: {
    description: "",
    text: "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
  },
  insert_jokes: {
    description: "User-defined: write funny comments",
    text: "You are a programming assistant. Use backquotes for code blocks, but insert into comments inside code blocks funny remarks, a joke inspired by the code or play on words. For example ```\n// Hocus, pocus\ngetTheFocus();\n```.",
  },
} as const;

export const CUSTOM_PROMPTS_RESPONSE: CustomPromptsResponse = {
  system_prompts: SYSTEM_PROMPTS,
  toolbox_commands: {
    explain: {
      description: "Explain code",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nExplain this specific code block:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    user0: {
      description: "User-defined: translate to horrible code",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE_PATH_COLON_CURSOR%\nRewrite this specific code block into a very inefficient and cryptic one, but still correct:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    typos: {
      description: "Fix typos",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite this specific code block to fix typos, especially inside strings and comments:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    comment: {
      description: "Comment each line",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nComment each line of this specific code block:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    shorter: {
      description: "Make code shorter",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nMake this specific code block shorter:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    naming: {
      description: "Improve variable names",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nImprove variable names in this specific code block:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    summarize: {
      description: "Summarize code in 1 paragraph",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nSummarize this specific code block in 1 paragraph:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    edit: {
      description: "Edit code, write instruction after the command",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nRe-write this specific code block, making this edit: %ARGS%\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    typehints: {
      description: "Add type hints",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nAdd type hints to this specific code block:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    improve: {
      description: "Rewrite this specific code block of code to improve it",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nRewrite this specific code block of code to improve it:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    bugs: {
      description: "Find and fix bugs",
      messages: [
        {
          role: "system",
          content:
            "You are a programming assistant. Use backquotes for code blocks, give links to documentation at the end of the response.",
        },
        {
          role: "user",
          content:
            "@file %CURRENT_FILE%:%CURSOR_LINE%\nFind and fix bugs in this specific code block:\n\n```\n%CODE_SELECTION%```\n",
        },
      ],
      selection_needed: [1, 50],
      selection_unwanted: false,
      insert_at_cursor: false,
    },
    gen: {
      description: "Create new code, provide a description after the command",
      messages: [
        {
          role: "system",
          content:
            "You are a fill-in-the middle model, analyze suffix and prefix, generate code that goes exactly between suffix and prefix. Never rewrite existing code. Watch indent level carefully. Never fix anything outside of your generated code. Stop after writing just one thing.",
        },
        {
          role: "user",
          content: "@file %CURRENT_FILE%:%CURSOR_LINE%-\n",
        },
        {
          role: "user",
          content: "@file %CURRENT_FILE%:-%CURSOR_LINE%\n",
        },
        {
          role: "user",
          content: "%ARGS%",
        },
      ],
      selection_needed: [],
      selection_unwanted: true,
      insert_at_cursor: true,
    },
  },
};
