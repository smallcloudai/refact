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

describe("Start a new chat", () => {
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
      pages: [{ name: "history" }],
      config: {
        apiKey: "test",
        lspPort: 8001,
        themeProps: {},
        host: "vscode",
        addressURL: "Refact",
      },
    },
  });

  // TODO: copy this for other tests done at a higher level
  test("open chat with New Chat Button", async () => {
    const btn = app.getByText("Start a new chat");
    await user.click(btn);

    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
  });
});
