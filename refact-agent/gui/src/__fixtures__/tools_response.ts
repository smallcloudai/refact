import { ToolGroup } from "../services/refact";

export const STUB_TOOL_RESPONSE: ToolGroup[] = [
  {
    name: "ast",
    category: "builtin",
    description: "Use AST to find information",
    tools: [
      {
        enabled: true,
        spec: {
          name: "definition",
          display_name: "Definition",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "builtin",
            config_path: "~/.config/refact/builtin_tools.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition2",
          display_name: "Definition Two",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
    ],
  },
  {
    name: "mcp_fetch",
    category: "mcp",
    description: "Use Fetch MCP to execute HTTP requests ",
    tools: [
      {
        enabled: true,
        spec: {
          name: "mcp_fetch",
          display_name: "MCP Fetch",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path: "~/.config/refact/integration_tools.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition2",
          display_name: "Definition Two",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition3",
          display_name: "Definition Three",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition4",
          display_name: "Definition Four",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition5",
          display_name: "Definition Five",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
      {
        enabled: false,
        spec: {
          name: "definition6",
          display_name: "Definition Six",
          description: "Find definition of a symbol in the project using AST",

          parameters: [
            {
              name: "symbol",
              description:
                "The exact name of a function, method, class, type alias. No spaces allowed.",
              type: "string",
            },
          ],
          source: {
            source_type: "integration",
            config_path:
              "~/.config/refact/integrations.d/youShouldNotCare.yaml",
          },

          parameters_required: ["symbol"],
          agentic: false,
          experimental: false,
        },
      },
    ],
  },
];
