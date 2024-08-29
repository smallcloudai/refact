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
} from "../utils/mockServer";
import { InnerApp } from "../features/App";

describe("Restore Chat from history", () => {
  test("Restore chat from history", async () => {
    server.use(
      goodCaps,
      goodPrompts,
      noTools,
      noCommandPreview,
      noCompletions,
      goodUser,
    );

    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "initial setup" }, { name: "history" }],
        history: {
          id: {
            title: "test title",
            id: "id",
            createdAt: "0",
            updatedAt: "0",
            model: "test",
            messages: [
              { role: "user", content: "test user message" },
              { role: "assistant", content: "👋" },
            ],
          },
        },
        config: {
          apiKey: "test",
          lspPort: 8001,
          themeProps: {},
          host: "vscode",
          addressURL: "Refact",
        },
      },
    });

    const btn = app.getByText("test title");
    await user.click(btn);

    expect(app.queryByText("test user message")).not.toBeNull();

    expect(app.queryByText("👋")).not.toBeNull();
  });
});
