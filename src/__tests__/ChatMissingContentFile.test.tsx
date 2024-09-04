import { render } from "../utils/test-utils";
import { describe, test } from "vitest";
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

describe("Start a new chat", () => {
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
      // console.log("chat called");
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
            { role: "assistant", content: "ok" },
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

  // TODO: copy this for other tests done at a higher level
  test("response from tool request", async () => {
    // const btn = app.getByText("New chat");
    // await user.click(btn);

    // const textarea = app.container.querySelector("textarea");
    // expect(textarea).not.toBeNull();

    await new Promise((resolve) => setTimeout(resolve, 1000));
    app.debug(app.container, Infinity);
  });
});
