import { render } from "../utils/test-utils";
import { describe, expect, test, beforeEach, afterEach } from "vitest";
import {
  server,
  goodPrompts,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";
import { stubResizeObserver } from "../utils/test-utils";

describe("Start a new chat", () => {
  // TODO: this shouldn't need to be called here.

  beforeEach(() => {
    stubResizeObserver();

    server.use(
      goodPing,
      goodPrompts,
      noTools,
      noCommandPreview,
      noCompletions,
      goodUser,
      chatLinks,
      telemetryChat,
      telemetryNetwork,
    );
  });

  afterEach(() => {
    server.resetHandlers();
  });

  // TODO: copy this for other tests done at a higher level
  test("open chat with New Chat Button", async () => {
    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "history" }],
        teams: {
          group: { id: "123", name: "test" },
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
    const btn = app.getByText("New chat");
    await user.click(btn);

    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
  });

  test("open chat with New Chat Button when knowledge feature is available", async () => {
    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "history" }],
        teams: {
          group: { id: "123", name: "test" },
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
    const btn = app.getByText("New chat");
    await user.click(btn);

    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
  });
  test("open chat with New Chat Button when knowledge feature is NOT available", async () => {
    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "history" }],
        teams: {
          group: null,
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
    const btn = app.getByText("New chat");
    await user.click(btn);

    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
  });
});
