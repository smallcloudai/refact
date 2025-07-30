import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ChatContent } from ".";
import { Provider } from "react-redux";
import { RootState, setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { MarkdownMessage } from "../../__fixtures__/markdown";
import type { BaseMessage } from "../../services/refact";
// TODO: update fixtures
import {
  CHAT_FUNCTIONS_MESSAGES,
  CHAT_WITH_DIFF_ACTIONS,
  CHAT_WITH_DIFFS,
  FROG_CHAT,
  LARGE_DIFF,
  CHAT_WITH_MULTI_MODAL,
  CHAT_CONFIG_THREAD,
  STUB_LINKS_FOR_CHAT_RESPONSE,
  CHAT_WITH_TEXTDOC,
  MARKDOWN_ISSUE,
} from "../../__fixtures__";
import { http, HttpResponse } from "msw";
import { CHAT_LINKS_URL } from "../../services/refact/consts";
import {
  goodPing,
  goodUser,
  noCommandPreview,
  noCompletions,
  noTools,
} from "../../__fixtures__/msw";

const TEXT_DOC_UPDATE = {
  waitingBranches: [],
  streamingBranches: [],
  loading: false,
  messages: {
    "J7CJxOiP5F:100:1:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 1,
      ftm_prev_alt: 100,
      ftm_role: "user",
      ftm_content:
        "@file refact-agent/engine/tests/emergency_frog_situation/frog.py \nadd a kiss method to frog\n",
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579150.154098,
      ftm_user_preferences: {
        model: "claude-3-7-sonnet-20250219",
        tools: [
          {
            name: "search_symbol_definition",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "symbols",
                type: "string",
                description:
                  "Comma-separated list of symbols to search for (functions, methods, classes, type aliases). No spaces allowed in symbol names.",
              },
            ],
            description: "Find definition of a symbol in the project using AST",
            display_name: "Definition",
            experimental: false,
            parameters_required: ["symbols"],
          },
          {
            name: "search_symbol_usages",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "symbols",
                type: "string",
                description:
                  "Comma-separated list of symbols to search for (functions, methods, classes, type aliases). No spaces allowed in symbol names.",
              },
            ],
            description: "Find usages of a symbol within a project using AST",
            display_name: "References",
            experimental: false,
            parameters_required: ["symbols"],
          },
          {
            name: "tree",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "path",
                type: "string",
                description:
                  "An absolute path to get files tree for. Do not pass it if you need a full project tree.",
              },
              {
                name: "use_ast",
                type: "boolean",
                description:
                  "If true, for each file an array of AST symbols will appear as well as its filename",
              },
            ],
            description:
              "Get a files tree with symbols for the project. Use it to get familiar with the project, file names and symbols",
            display_name: "Tree",
            experimental: false,
            parameters_required: [],
          },
          {
            name: "cat",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "paths",
                type: "string",
                description:
                  "Comma separated file names or directories: dir1/file1.ext,dir3/dir4.",
              },
            ],
            description:
              "Like cat in console, but better: it can read multiple files and images. Prefer to open full files.",
            display_name: "Cat",
            experimental: false,
            parameters_required: ["paths"],
          },
          {
            name: "search_pattern",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "pattern",
                type: "string",
                description:
                  "The pattern is used to search for matching file/folder names/paths, and also for matching text inside files. Use (?i) at the start for case-insensitive search.",
              },
              {
                name: "scope",
                type: "string",
                description:
                  "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file.",
              },
            ],
            description:
              "Search for files and folders whose names or paths match the given regular expression pattern, and also search for text matches inside files using the same pattern. Reports both path matches and text matches in separate sections.",
            display_name: "Regex Search",
            experimental: false,
            parameters_required: ["pattern", "scope"],
          },
          {
            name: "create_textdoc",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "path",
                type: "string",
                description: "Absolute path to new file.",
              },
              {
                name: "content",
                type: "string",
                description: "The initial text or code.",
              },
            ],
            description:
              "Creates a new text document or code or completely replaces the content of an existing document. Avoid trailing spaces and tabs.",
            display_name: "Create Text Document",
            experimental: false,
            parameters_required: ["path", "content"],
          },
          {
            name: "update_textdoc",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "path",
                type: "string",
                description: "Absolute path to the file to change.",
              },
              {
                name: "old_str",
                type: "string",
                description:
                  "The exact text that needs to be updated. Use update_textdoc_regex if you need pattern matching.",
              },
              {
                name: "replacement",
                type: "string",
                description: "The new text that will replace the old text.",
              },
              {
                name: "multiple",
                type: "boolean",
                description:
                  "If true, applies the replacement to all occurrences; if false, only the first occurrence is replaced.",
              },
            ],
            description:
              "Updates an existing document by replacing specific text, use this if file already exists. Optimized for large files or small changes where simple string replacement is sufficient. Avoid trailing spaces and tabs.",
            display_name: "Update Text Document",
            experimental: false,
            parameters_required: ["path", "old_str", "replacement"],
          },
          {
            name: "update_textdoc_regex",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "path",
                type: "string",
                description: "Absolute path to the file to change.",
              },
              {
                name: "pattern",
                type: "string",
                description:
                  "A regex pattern to match the text that needs to be updated. Prefer simpler regexes for better performance.",
              },
              {
                name: "replacement",
                type: "string",
                description:
                  "The new text that will replace the matched pattern.",
              },
              {
                name: "multiple",
                type: "boolean",
                description:
                  "If true, applies the replacement to all occurrences; if false, only the first occurrence is replaced.",
              },
            ],
            description:
              "Updates an existing document using regex pattern matching. Ideal when changes can be expressed as a regular expression or when you need to match variable text patterns. Avoid trailing spaces and tabs.",
            display_name: "Update Text Document with Regex",
            experimental: false,
            parameters_required: ["path", "pattern", "replacement"],
          },
          {
            name: "rm",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "path",
                type: "string",
                description:
                  "Absolute or relative path of the file or directory to delete.",
              },
              {
                name: "recursive",
                type: "boolean",
                description:
                  "If true and target is a directory, delete recursively. Defaults to false.",
              },
              {
                name: "dry_run",
                type: "boolean",
                description:
                  "If true, only report what would be done without deleting.",
              },
              {
                name: "max_depth",
                type: "number",
                description: "(Optional) Maximum depth (currently unused).",
              },
            ],
            description:
              "Deletes a file or directory. Use recursive=true for directories. Set dry_run=true to preview without deletion.",
            display_name: "rm",
            experimental: false,
            parameters_required: ["path"],
          },
          {
            name: "mv",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: false,
            parameters: [
              {
                name: "source",
                type: "string",
                description: "Path of the file or directory to move.",
              },
              {
                name: "destination",
                type: "string",
                description:
                  "Target path where the file or directory should be placed.",
              },
              {
                name: "overwrite",
                type: "boolean",
                description:
                  "If true and target exists, replace it. Defaults to false.",
              },
            ],
            description:
              "Moves or renames files and directories. If a simple rename fails due to a cross-device error and the source is a file, it falls back to copying and deleting. Use overwrite=true to replace an existing target.",
            display_name: "mv",
            experimental: false,
            parameters_required: ["source", "destination"],
          },
          {
            name: "strategic_planning",
            source: {
              config_path: "/Users/marc/.config/refact/builtin_tools.yaml",
              source_type: "builtin",
            },
            agentic: true,
            parameters: [
              {
                name: "important_paths",
                type: "string",
                description:
                  "Comma-separated list of all filenames which are required to be considered for resolving the problem. More files - better, include them even if you are not sure.",
              },
            ],
            description:
              "Strategically plan a solution for a complex problem or create a comprehensive approach.",
            display_name: "Strategic Planning",
            experimental: false,
            parameters_required: ["important_paths"],
          },
          {
            name: "cmdline_cargo_check",
            source: {
              config_path:
                "/Users/marc/Projects/refact/.refact/integrations.d/cmdline_cargo_check.yaml",
              source_type: "integration",
            },
            agentic: true,
            parameters: [
              {
                name: "additional_params",
                type: "string",
                description:
                  "Additional parameters for cargo check, such as --workspace or --all-features",
              },
              {
                name: "project_path",
                type: "string",
                description:
                  "Absolute path to the project, the rust stuff is at refact/refact-agent/engine/Cargo.toml for the Refact project, so use ../refact/refact-agent/engine",
              },
            ],
            description:
              "Run cargo check to verify Rust code compilation without producing an executable",
            display_name: "cmdline_cargo_check",
            experimental: false,
            parameters_required: ["additional_params", "project_path"],
          },
          {
            name: "service_webserver",
            source: {
              config_path: "",
              source_type: "integration",
            },
            agentic: true,
            parameters: [
              {
                name: "action",
                type: "string",
                description: "Action to perform: start, restart, stop, status",
              },
            ],
            description: "",
            display_name: "service_webserver",
            experimental: false,
            parameters_required: [],
          },
          {
            name: "postgres",
            source: {
              config_path:
                "/Users/marc/.config/refact/integrations.d/postgres.yaml",
              source_type: "integration",
            },
            agentic: true,
            parameters: [
              {
                name: "query",
                type: "string",
                description:
                  "Don't forget semicolon at the end, examples:\nSELECT * FROM table_name;\nCREATE INDEX my_index_users_email ON my_users (email);",
              },
            ],
            description:
              "PostgreSQL integration, can run a single query per call.",
            display_name: "PostgreSQL",
            experimental: false,
            parameters_required: ["query"],
          },
          {
            name: "shell",
            source: {
              config_path:
                "/Users/marc/.config/refact/integrations.d/shell.yaml",
              source_type: "integration",
            },
            agentic: true,
            parameters: [
              {
                name: "command",
                type: "string",
                description: "shell command to execute",
              },
              {
                name: "workdir",
                type: "string",
                description: "workdir for the command",
              },
            ],
            description:
              'Execute a single command, using the "sh" on unix-like systems and "powershell.exe" on windows. Use it for one-time tasks like dependencies installation. Don\'t call this unless you have to. Not suitable for regular work because it requires a confirmation at each step.',
            display_name: "Shell",
            experimental: false,
            parameters_required: ["command", "workdir"],
          },
          {
            name: "docker",
            source: {
              config_path:
                "/Users/marc/.config/refact/integrations.d/docker.yaml",
              source_type: "integration",
            },
            agentic: true,
            parameters: [
              {
                name: "command",
                type: "string",
                description: "Examples: docker images",
              },
            ],
            description:
              "Access to docker cli, in a non-interactive way, don't open a shell.",
            display_name: "Docker CLI",
            experimental: true,
            parameters_required: ["command"],
          },
        ],
      },
    },
    "J7CJxOiP5F:100:0:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 0,
      ftm_prev_alt: 100,
      ftm_role: "system",
      ftm_content:
        "You are a fully autonomous agent for coding tasks.\nYour task is to identify and solve the problem by directly changing files in the given project.\nYou must follow the strategy, step by step in the given order without skipping.\nYou must confirm the plan with the user before proceeding!\n\n1. Explore the Problem\n- Call available tools to find relevant files.\n\n2. Draft the Solution Plan\n- Identify the root cause and sketch the required code changes (files to touch, functions to edit, tests to add).\n  - Propose the changes to the user\n    • the suspected root cause\n    • the exact files/functions to modify or create\n    • the new or updated tests to add\n    • the expected outcome and success criteria\n\n\n**BEST PRACTICES**\n- You might receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate well and they are always in English. Answer in the language the user has asked the question.\n- When running on user's laptop, you most likely have the shell() tool. It's for one-time dependency installations, or doing whatever user is asking you to do. Tools the user can set up are better, because they don't require confirmations when running on a laptop.\nWhen doing something for the project using shell() tool, offer the user to make a cmdline_* tool after you have successfully run\nthe shell() call. But double-check that it doesn't already exist, and it is actually typical for this kind of project. You can offer\nthis by writing:\n\n🧩SETTINGS:cmdline_cargo_check\n\nfrom a new line, that will open (when clicked) a wizard that creates `cargo check` (in this example) command line tool.\n\nIn a similar way, service_* tools work. The difference is cmdline_* is designed for non-interactive blocking commands that immediately return text in stdout/stderr, and service_* is designed for blocking background commands, such as hypercorn server that runs forever until you hit Ctrl+C.\nHere is another example:\n\n🧩SETTINGS:service_hypercorn\n\nThe current IDE workspace has these project directories:\n/Users/marc/Projects/refact\n\nThere is no active file currently open in the IDE.\nThe project is under git version control, located at:\n/Users/marc/Projects/refact\n\nThe Refact Agent project is a Rust-based executable designed to integrate seamlessly with IDEs like VSCode and JetBrains. Its primary function is to maintain up-to-date AST and VecDB indexes, ensuring efficient code completion and project analysis. The agent acts as an LSP server, providing tools for code completion, chat functionalities, and integration with various external tools such as browsers, databases, and debuggers. It supports multiple programming languages for AST capabilities and can be used both as a standalone command-line tool and within a Python program.\nThe project is structured with a main Rust source directory src/ containing modules for background tasks, integrations, HTTP handling, and more. The tests/ directory includes various test scripts mostly written in python, while the examples/ directory provides usage examples.\n\n\nBefore any action, try to gather existing knowledge:\n  - Call the `knowledge()` tool to get initial information about the project and the task.\n  - This tool gives you access to memories, and external data, example trajectories (🗃️) to help understand and solve the task.\nAlways Learn and Record. Use `create_knowledge()` to:\n  - Important coding patterns,\n  - Key decisions and their reasons,\n  - Effective strategies,\n  - Insights about the project's structure and dependencies,\n  - When the task is finished to record useful insights.\n  - Take every opportunity to build and enrich your knowledge base—don’t wait for instructions.\n\nThere are some pre-existing core memories:\n",
      ftm_tool_calls: {},
      ftm_call_id: "",
      ftm_usage: {},
      ftm_created_ts: 1752579150.715106,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:2:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 2,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579155.120208,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:3:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 3,
      ftm_prev_alt: 100,
      ftm_role: "title",
      ftm_content: "Add Kiss Method to Frog Class in frog.py",
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: {
        coins: 552,
        tokens_prompt: 104,
        pp1000t_prompt: 3000,
        tokens_cache_read: 0,
        tokens_completion: 16,
        pp1000t_cache_read: 300,
        pp1000t_completion: 15000,
        tokens_prompt_text: 0,
        tokens_prompt_audio: 0,
        tokens_prompt_image: 0,
        tokens_prompt_cached: 0,
        tokens_cache_creation: 0,
        pp1000t_cache_creation: 3750,
        tokens_completion_text: 0,
        tokens_completion_audio: 0,
        tokens_completion_reasoning: 0,
        pp1000t_completion_reasoning: 0,
      },
      ftm_created_ts: 1752579155.120208,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:4:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 4,
      ftm_prev_alt: 100,
      ftm_role: "assistant",
      ftm_content:
        "I'll help you add a kiss method to the Frog class in the specified file. First, let's explore the file to understand its structure.",
      ftm_tool_calls: [
        {
          id: "toolu_01RTbas6gyjXupdGsp2AURzp",
          type: "function",
          function: {
            name: "cat",
            arguments:
              '{"paths": "refact-agent/engine/tests/emergency_frog_situation/frog.py"}',
          },
        },
      ],
      ftm_call_id: "",
      ftm_usage: {
        coins: 12099,
        tokens_prompt: 3523,
        pp1000t_prompt: 3000,
        tokens_cache_read: 0,
        tokens_completion: 102,
        pp1000t_cache_read: 300,
        pp1000t_completion: 15000,
        tokens_prompt_text: 0,
        tokens_prompt_audio: 0,
        tokens_prompt_image: 0,
        tokens_prompt_cached: 0,
        tokens_cache_creation: 0,
        pp1000t_cache_creation: 3750,
        tokens_completion_text: 0,
        tokens_completion_audio: 0,
        tokens_completion_reasoning: 0,
        pp1000t_completion_reasoning: 0,
      },
      ftm_created_ts: 1752579155.120208,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:5:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 5,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579155.120208,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:6:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 6,
      ftm_prev_alt: 100,
      ftm_role: "tool",
      ftm_content:
        "Paths found:\n/Users/marc/Projects/refact/refact-agent/engine/tests/emergency_frog_situation/frog.py\n",
      ftm_tool_calls: {},
      ftm_call_id: "toolu_01RTbas6gyjXupdGsp2AURzp",
      ftm_usage: {},
      ftm_created_ts: 1752579155.787955,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:7:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 7,
      ftm_prev_alt: 100,
      ftm_role: "context_file",
      ftm_content:
        '[{"file_name":"refact-agent/engine/tests/emergency_frog_situation/frog.py","file_content":"   1 | import numpy as np\\n   2 | \\n   3 | DT = 0.01\\n   4 | \\n   5 | class Frog:\\n   6 |     def __init__(self, x, y, vx, vy):\\n   7 |         self.x = x\\n   8 |         self.y = y\\n   9 |         self.vx = vx\\n  10 |         self.vy = vy\\n  11 | \\n  12 |     def bounce_off_banks(self, pond_width, pond_height):\\n  13 |         if self.x < 0:\\n  14 |             self.vx = np.abs(self.vx)\\n  15 |         elif self.x > pond_width:\\n  16 |             self.vx = -np.abs(self.vx)\\n  17 |         if self.y < 0:\\n  18 |             self.vy = np.abs(self.vy)\\n  19 |         elif self.y > pond_height:\\n  20 |             self.vy = -np.abs(self.vy)\\n  21 | \\n  22 |     def jump(self, pond_width, pond_height):\\n  23 |         self.x += self.vx * DT\\n  24 |         self.y += self.vy * DT\\n  25 |         self.bounce_off_banks(pond_width, pond_height)\\n  26 |         self.x = np.clip(self.x, 0, pond_width)\\n  27 |         self.y = np.clip(self.y, 0, pond_height)\\n  28 | \\n  29 |     def croak(self, n_times):\\n  30 |         for n in range(n_times):\\n  31 |             print(\\"croak\\")\\n  32 |     \\n  33 |     def swim(self, pond_width, pond_height):\\n  34 |         print(\\"Swimming...\\")\\n  35 |         print(\\"Splash! The frog is moving through the water\\")\\n  36 |         self.x += self.vx * DT\\n  37 |         self.y += self.vy * DT\\n  38 |         print(\\"Ripple... ripple...\\")\\n  39 |         self.bounce_off_banks(pond_width, pond_height)\\n  40 |         self.x = np.clip(self.x, 0, pond_width)\\n  41 |         self.y = np.clip(self.y, 0, pond_height)\\n  42 |         print(\\"The frog swam to position ({:.2f}, {:.2f})\\".format(self.x, self.y))\\n  43 | \\n  44 | \\n  45 | class AlternativeFrog:\\n  46 |     def alternative_jump(self):\\n  47 |         pass\\n  48 | \\n  49 | \\n  50 | def standalone_jumping_function():\\n  51 |     print(\\"I\'m a frog! Jump! Jump! Really!\\")\\n","line1":1,"line2":50}]',
      ftm_tool_calls: {},
      ftm_call_id: "",
      ftm_usage: {},
      ftm_created_ts: 1752579155.787955,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:8:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 8,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579166.161127,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:9:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 9,
      ftm_prev_alt: 100,
      ftm_role: "assistant",
      ftm_content:
        "Now I understand the structure of the Frog class. I'll add a kiss method to it. Here's my plan:\n\n1. I'll add a new method called `kiss()` to the Frog class\n2. The method will print a simple message indicating a kiss action\n\nLet me implement this change:",
      ftm_tool_calls: [
        {
          id: "toolu_01MJjZ3QRtYVvs5bTBdTopVv",
          type: "function",
          function: {
            name: "update_textdoc",
            arguments:
              '{"path": "/Users/marc/Projects/refact/refact-agent/engine/tests/emergency_frog_situation/frog.py", "old_str": "    def swim(self, pond_width, pond_height):\\n        print(\\"Swimming...\\")\\n        print(\\"Splash! The frog is moving through the water\\")\\n        self.x += self.vx * DT\\n        self.y += self.vy * DT\\n        print(\\"Ripple... ripple...\\")\\n        self.bounce_off_banks(pond_width, pond_height)\\n        self.x = np.clip(self.x, 0, pond_width)\\n        self.y = np.clip(self.y, 0, pond_height)\\n        print(\\"The frog swam to position ({:.2f}, {:.2f})\\".format(self.x, self.y))\\n", "replacement": "    def swim(self, pond_width, pond_height):\\n        print(\\"Swimming...\\")\\n        print(\\"Splash! The frog is moving through the water\\")\\n        self.x += self.vx * DT\\n        self.y += self.vy * DT\\n        print(\\"Ripple... ripple...\\")\\n        self.bounce_off_banks(pond_width, pond_height)\\n        self.x = np.clip(self.x, 0, pond_width)\\n        self.y = np.clip(self.y, 0, pond_height)\\n        print(\\"The frog swam to position ({:.2f}, {:.2f})\\".format(self.x, self.y))\\n    \\n    def kiss(self):\\n        print(\\"The frog gives you a little kiss! *smooch*\\")\\n"}',
          },
        },
      ],
      ftm_call_id: "",
      ftm_usage: {
        coins: 21729,
        tokens_prompt: 4518,
        pp1000t_prompt: 3000,
        tokens_cache_read: 0,
        tokens_completion: 545,
        pp1000t_cache_read: 300,
        pp1000t_completion: 15000,
        tokens_prompt_text: 0,
        tokens_prompt_audio: 0,
        tokens_prompt_image: 0,
        tokens_prompt_cached: 0,
        tokens_cache_creation: 0,
        pp1000t_cache_creation: 3750,
        tokens_completion_text: 0,
        tokens_completion_audio: 0,
        tokens_completion_reasoning: 0,
        pp1000t_completion_reasoning: 0,
      },
      ftm_created_ts: 1752579166.161127,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:10:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 10,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579166.161127,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:11:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 11,
      ftm_prev_alt: 100,
      ftm_role: "diff",
      ftm_content: "The operation has succeeded.\n",
      ftm_tool_calls: {},
      ftm_call_id: "toolu_01MJjZ3QRtYVvs5bTBdTopVv",
      ftm_usage: {},
      ftm_created_ts: 1752579273.498838,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:12:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 12,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579278.328317,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:13:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 13,
      ftm_prev_alt: 100,
      ftm_role: "assistant",
      ftm_content:
        'I\'ve successfully added a `kiss()` method to the Frog class in the file `refact-agent/engine/tests/emergency_frog_situation/frog.py`.\n\nThe new method:\n```python\ndef kiss(self):\n    print("The frog gives you a little kiss! *smooch*")\n```\n\nThis method can be called on any Frog instance and will print a message indicating that the frog gives a kiss. The method has been added while maintaining the existing code structure and indentation style of the file.',
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: {
        coins: 17100,
        tokens_prompt: 5080,
        pp1000t_prompt: 3000,
        tokens_cache_read: 0,
        tokens_completion: 124,
        pp1000t_cache_read: 300,
        pp1000t_completion: 15000,
        tokens_prompt_text: 0,
        tokens_prompt_audio: 0,
        tokens_prompt_image: 0,
        tokens_prompt_cached: 0,
        tokens_cache_creation: 0,
        pp1000t_cache_creation: 3750,
        tokens_completion_text: 0,
        tokens_completion_audio: 0,
        tokens_completion_reasoning: 0,
        pp1000t_completion_reasoning: 0,
      },
      ftm_created_ts: 1752579278.328317,
      ftm_user_preferences: null,
    },
    "J7CJxOiP5F:100:14:100": {
      ft_app_specific: null,
      ftm_belongs_to_ft_id: "J7CJxOiP5F",
      ftm_alt: 100,
      ftm_num: 14,
      ftm_prev_alt: 100,
      ftm_role: "kernel",
      ftm_content: null,
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: 1752579278.328317,
      ftm_user_preferences: null,
    },
  },
  ft_id: "J7CJxOiP5F",
  endNumber: 14,
  endAlt: 100,
  endPrevAlt: 100,
  thread: {
    located_fgroup_id: "425mky3q5z",
    ft_id: "J7CJxOiP5F",
    ft_need_user: 100,
    ft_need_assistant: -1,
    ft_fexp_id: "id:agent:1",
    ft_confirmation_request: [
      {
        rule: "default",
        command: "update_textdoc",
        ftm_num: 11,
        tool_call_id: "toolu_01MJjZ3QRtYVvs5bTBdTopVv",
      },
    ],
    ft_confirmation_response: ["toolu_01MJjZ3QRtYVvs5bTBdTopVv"],
    ft_title: "Add Kiss Method to Frog Class in frog.py",
  },
};

const MockedStore: React.FC<{
  messages?: BaseMessage[];
  messageThread?: RootState["threadMessages"];
}> = ({ messages, messageThread }) => {
  const store = setUpStore({
    threadMessages: {
      waitingBranches: [],
      streamingBranches: [],
      ft_id: null,
      endNumber: 0,
      endAlt: 0,
      endPrevAlt: 0,
      thread: null,
      loading: false,
      messages: messages
        ? messages.reduce((acc, cur) => {
            return { ...acc, [cur.ftm_call_id]: cur };
          }, {})
        : {},
      ...(messageThread ? messageThread : {}),
    },
  });

  return (
    <Provider store={store}>
      <Theme>
        <ChatContent />
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "Chat Content",
  component: MockedStore,
  args: {
    messages: [],
  },
} satisfies Meta<typeof MockedStore>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const WithFunctions: Story = {
  args: {
    ...meta.args,
    messages: CHAT_FUNCTIONS_MESSAGES,
  },
};

export const Notes: Story = {
  args: {
    messages: FROG_CHAT,
  },
};

export const WithDiffs: Story = {
  args: {
    messages: CHAT_WITH_DIFFS,
  },
};

export const WithDiffActions: Story = {
  args: {
    messages: CHAT_WITH_DIFF_ACTIONS,
  },
};

export const LargeDiff: Story = {
  args: {
    messages: LARGE_DIFF,
  },
};

export const Empty: Story = {
  args: {
    ...meta.args,
  },
};

export const AssistantMarkdown: Story = {
  args: {
    ...meta.args,
    messages: [
      {
        ftm_role: "assistant",
        ftm_content: MarkdownMessage,
        ftm_belongs_to_ft_id: "",
        ftm_alt: 0,
        ftm_num: 1,
        ftm_prev_alt: 0,
        ftm_call_id: "",
        ftm_created_ts: 0,
      },
    ],
  },
};

export const ToolImages: Story = {
  args: {
    ...meta.args,
  },
};

export const MultiModal: Story = {
  args: {
    messages: CHAT_WITH_MULTI_MODAL,
  },
};

export const IntegrationChat: Story = {
  args: {
    messages: CHAT_CONFIG_THREAD,
  },
  parameters: {
    msw: {
      handlers: [
        http.post(`http://127.0.0.1:8001${CHAT_LINKS_URL}`, () => {
          return HttpResponse.json(STUB_LINKS_FOR_CHAT_RESPONSE);
        }),
      ],
    },
  },
};

export const TextDoc: Story = {
  args: {
    messages: CHAT_WITH_TEXTDOC,
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        noTools,

        noCompletions,
        noCommandPreview,
      ],
    },
  },
};

export const MarkdownIssue: Story = {
  args: {
    messages: MARKDOWN_ISSUE,
  },
  parameters: {
    msw: {
      handlers: [
        goodPing,

        goodUser,
        // noChatLinks,
        noTools,

        noCompletions,
        noCommandPreview,
      ],
    },
  },
};

export const ToolWaiting: Story = {
  args: {
    messages: [
      {
        ftm_role: "user",
        ftm_content: "call a tool and wait",
        ftm_belongs_to_ft_id: "",
        ftm_alt: 0,
        ftm_num: 1,
        ftm_prev_alt: 0,
        ftm_call_id: "",
        ftm_created_ts: 0,
      },
      {
        ftm_role: "assistant",
        ftm_content: "",
        ftm_tool_calls: [
          {
            id: "toolu_01JbWarAwzjMyV6azDkd5skX",
            function: {
              arguments: '{"use_ast": true}',
              name: "tree",
            },
            type: "function",
            index: 0,
          },
        ],
        ftm_belongs_to_ft_id: "",
        ftm_alt: 0,
        ftm_num: 2,
        ftm_prev_alt: 0,
        ftm_call_id: "",
        ftm_created_ts: 0,
      },
    ],
  },
  parameters: {
    msw: {
      handlers: [goodPing, goodUser, noTools, noCompletions, noCommandPreview],
    },
  },
};

export const TextDocUpdate: Story = {
  args: {
    messageThread: TEXT_DOC_UPDATE,
  },
  parameters: {
    msw: {
      handlers: [goodPing, goodUser, noTools, noCompletions, noCommandPreview],
    },
  },
};
