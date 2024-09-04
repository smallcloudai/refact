import { render, waitFor } from "../utils/test-utils";
import { describe, test, expect } from "vitest";
import {
  server,
  goodPrompts,
  goodCaps,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";
import { http, HttpResponse } from "msw";
import { responseStream } from "../__fixtures__/context_response";

describe("Content file issue", () => {
  test(
    "response from lsp",
    async () => {
      server.use(
        goodCaps,
        goodPrompts,
        noTools,
        noCommandPreview,
        noCompletions,
        goodUser,
      );

      server.use(
        http.post("http://127.0.0.1:8001/v1/chat", () => {
          return new HttpResponse(responseStream(), {
            headers: {
              "Content-Type": "application/json",
              "Transfer-Encoding": "chunked",
            },
          });
        }),
      );

      const { user: _user, ...app } = render(<InnerApp />, {
        preloadedState: {
          pages: [{ name: "history" }, { name: "chat" }],
          config: {
            apiKey: "test",
            lspPort: 8001,
            themeProps: {},
            host: "vscode",
            addressURL: "Refact",
          },
          chat: {
            thread: {
              id: "12345",
              title: "foo",
              messages: [
                { role: "user", content: "search" },
                {
                  role: "assistant",
                  content: "ok",
                  tool_calls: [
                    {
                      function: { name: "search", arguments: "foo" },
                      index: 0,
                    },
                  ],
                },
              ],
              model: "test",
            },
            prevent_send: false,
            streaming: false,
            cache: {},
            waiting_for_response: false,
            system_prompt: {},
            tool_use: "agent",
            send_immediately: true,
            error: null,
          },
        },
      });

      // await new Promise((r) => setTimeout(r, 1000));
      // app.debug(app.container, Infinity);
      // screen.logTestingPlaygroundURL();
      await waitFor(
        () => {
          // seems duplicated :/
          const elem = app.getAllByText(/📎 main\.rs\.json/i);
          expect(elem.length).toBeGreaterThan(0);
        },
        { timeout: 10000 },
      );
    },
    { timeout: 10000 },
  );
});
