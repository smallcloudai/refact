import { ToolCommand } from "../services/refact";

export const STUB_TOOL_RESPONSE: ToolCommand[] = [
  {
    type: "function",
    function: {
      name: "search",
      agentic: false,
      description: "Find similar pieces of code or text using vector database",
      parameters: {
        type: "object",
        properties: {
          query: {
            type: "string",
            description:
              "Single line, paragraph or code sample to search for similar content.",
          },
          scope: {
            type: "string",
            description:
              "'workspace' to search all files in workspace, 'dir/subdir/' to search in files within a directory, 'dir/file.ext' to search in a single file.",
          },
        },
        required: ["query", "scope"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "definition",
      agentic: false,
      description: "Read definition of a symbol in the project using AST",
      parameters: {
        type: "object",
        properties: {
          symbol: {
            type: "string",
            description:
              "The exact name of a function, method, class, type alias. No spaces allowed.",
          },
          skeleton: {
            type: "boolean",
            description:
              "Skeletonize ouput. Set true to explore, set false when as much context as possible is needed.",
          },
        },
        required: ["symbol"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "references",
      agentic: false,
      description: "Find usages of a symbol within a project using AST",
      parameters: {
        type: "object",
        properties: {
          symbol: {
            type: "string",
            description:
              "The exact name of a function, method, class, type alias. No spaces allowed.",
          },
          skeleton: {
            type: "boolean",
            description:
              "Skeletonize ouput. Set true to explore, set false when as much context as possible is needed.",
          },
        },
        required: ["symbol"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "tree",
      agentic: false,
      description:
        "Get a files tree with symbols for the project. Use it to get familiar with the project, file names and symbols",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description:
              "An absolute path to get files tree for. Do not pass it if you need a full project tree.",
          },
          use_ast: {
            type: "boolean",
            description:
              "If true, for each file an array of AST symbols will appear as well as its filename",
          },
        },
        required: [],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "web",
      agentic: false,
      description: "Fetch a web page and convert to readable plain text.",
      parameters: {
        type: "object",
        properties: {
          url: {
            type: "string",
            description: "URL of the web page to fetch.",
          },
        },
        required: ["url"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "cat",
      agentic: false,
      description:
        "Like cat in console, but better: it can read multiple files and skeletonize them. Give it AST symbols important for the goal (classes, functions, variables, etc) to see them in full. It can also read images just fine.",
      parameters: {
        type: "object",
        properties: {
          paths: {
            type: "string",
            description:
              "Comma separated file names or directories: dir1/file1.ext, dir2/file2.ext, dir3/dir4",
          },
          symbols: {
            type: "string",
            description:
              "Comma separated AST symbols: MyClass, MyClass::method, my_function",
          },
          skeleton: {
            type: "boolean",
            description:
              "if true, files will be skeletonized - mostly only AST symbols will be visible",
          },
        },
        required: ["paths"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "locate",
      agentic: true,
      description:
        "Get a list of files that are relevant to solve a particular task.",
      parameters: {
        type: "object",
        properties: {
          problem_statement: {
            type: "string",
            description:
              "Copy word-for-word the problem statement as provided by the user, if available. Otherwise, tell what you need to do in your own words.",
          },
        },
        required: ["problem_statement"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "patch",
      agentic: true,
      description:
        "Collect context first, then write the necessary changes using the 📍-notation before code blocks, then call this function to apply the changes.\nTo make this call correctly, you only need the tickets.\nIf you wrote changes for multiple files, call this tool in parallel for each file.\nIf you have several attempts to change a single thing, for example following a correction from the user, pass only the ticket for the latest one.\nMultiple tickets is allowed only for PARTIAL_EDIT, otherwise only one ticket must be provided.\n",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description: "Path to the file to change.",
          },
          tickets: {
            type: "string",
            description:
              "Use 3-digit tickets comma separated to refer to the changes within ONE file. No need to copy anything else. Additionaly, you can put DELETE here to delete the file.",
          },
        },
        required: ["tickets", "path"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "postgres",
      agentic: true,
      description: "PostgreSQL integration, can run a single query per call.",
      parameters: {
        type: "object",
        properties: {
          query: {
            type: "string",
            description:
              "Don't forget semicolon at the end, examples:\nSELECT * FROM table_name;\nCREATE INDEX my_index_users_email ON my_users (email);\n",
          },
        },
        required: ["query"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "docker",
      agentic: true,
      description:
        "Access to docker cli, in a non-interactive way, don't open a shell.",
      parameters: {
        type: "object",
        properties: {
          command: {
            type: "string",
            description: "Examples: docker images",
          },
        },
        required: ["project_dir", "command"],
      },
    },
  },
  {
    type: "function",
    function: {
      name: "knowledge",
      agentic: true,
      description:
        "Fetches successful trajectories to help you accomplish your task. Call each time you have a new task to increase your chances of success.",
      parameters: {
        type: "object",
        properties: {
          im_going_to_use_tools: {
            type: "string",
            description:
              "Which tools are you about to use? Comma-separated list, examples: hg, git, gitlab, rust debugger, patch",
          },
          im_going_to_apply_to: {
            type: "string",
            description:
              "What your actions will be applied to? List all you can identify, starting with the project name. Comma-separated list, examples: project1, file1.cpp, MyClass, PRs, issues",
          },
          goal: {
            type: "string",
            description: "What is your goal here?",
          },
          language_slash_framework: {
            type: "string",
            description:
              "What programming language and framework is the current project using? Use lowercase, dashes and dots. Examples: python/django, typescript/node.js, rust/tokio, ruby/rails, php/laravel, c++/boost-asio",
          },
        },
        required: [
          "im_going_to_use_tools",
          "im_going_to_apply_to",
          "goal",
          "language_slash_framework",
        ],
      },
    },
  },
];
