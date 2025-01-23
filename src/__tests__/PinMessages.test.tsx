import { render } from "../utils/test-utils";
import { describe, expect, test } from "vitest";
import {
  server,
  goodPrompts,
  goodCaps,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
  goodPing,
  chatLinks,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";

describe("Pin messages", () => {
  server.use(
    goodPing,
    goodCaps,
    goodPrompts,
    noTools,
    noCommandPreview,
    noCompletions,
    goodUser,
    chatLinks,
  );

  test("it should replace 📍PARTIAL_EDIT 000 /Users/refact/code/refact-lsp/src/ast/ast_db.rs", () => {
    const app = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "chat" }],
        config: {
          apiKey: "test",
          lspPort: 8001,
          themeProps: {},
          host: "vscode",
          addressURL: "Refact",
        },
        chat: {
          prevent_send: false,
          streaming: false,
          error: null,
          waiting_for_response: false,
          max_new_tokens: 4096,
          cache: {},
          system_prompt: {},
          tool_use: "explore",
          send_immediately: false,
          thread: {
            id: "8ab48978-3ad3-4387-944c-fedd15bdbc0d",
            messages: [
              {
                role: "context_file",
                content: [
                  {
                    file_name: "refact-lsp/src/ast/ast_db.rs",
                    file_content: "big file\n",
                    line1: 1,
                    line2: 999,
                  },
                ],
              },
              {
                role: "user",
                content:
                  "\n```rust\n        doc_add(\n            ast_index.clone(),\n            &library_file_path.to_string(),\n            &library_text,\n            &mut errstats,\n        )\n\n```\n\nmake it one line\n",
              },
              {
                role: "assistant",
                content:
                  "To make the `doc_add` function call a single line, I will remove the line breaks and format it accordingly. Let's proceed with the change.",
                tool_calls: [
                  {
                    index: 0,
                    id: "call_5YBv9DPZETyjfpmsfwU0BbVm",
                    type: "function",
                    function: {
                      name: "search",
                      arguments: '{"query":"doc_add(","scope":"workspace"}',
                    },
                  },
                ],
              },
              {
                role: "tool",
                content: {
                  tool_call_id: "call_5YBv9DPZETyjfpmsfwU0BbVm",
                  content:
                    "Search produced no results, adjust the query or try a different scope.",
                },
              },
              {
                role: "assistant",
                content:
                  "It seems that the `doc_add` function is not defined within the current workspace. However, I can still proceed with making the requested change to the function call in the test module. Let's update the code accordingly.\n📍PARTIAL_EDIT 000 /Users/kot/code/refact-lsp/src/ast/ast_db.rs\n```rust\n        doc_add(ast_index.clone(), &library_file_path.to_string(), &library_text, &mut errstats)\n```",
              },
            ],
            title:
              "```rust\n        doc_add(\n            ast_index.clone(),\n            &library_file_path.to_string(),\n            &library_text,\n            &mut errstats,\n        )\n\n```\n\nmake it one line\n",
            model: "gpt-4o",
            tool_use: "explore",
            read: true,
            createdAt: "2024-10-20T05:03:15.550Z",
            updatedAt: "2024-10-20T05:03:15.550Z",
          },
        },
      },
    });

    expect(() => app.getAllByText(/📍/g)).throws();
  });
});
