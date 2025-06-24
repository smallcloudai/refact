import { render } from "../utils/test-utils";
import { describe, expect, test } from "vitest";
import {
  server,
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

describe.skip("Restore Chat from history", () => {
  test("Restore chat from history", async () => {
    server.use(
      goodPing,

      noTools,
      noCommandPreview,
      noCompletions,
      goodUser,
      chatLinks,
      telemetryChat,
      telemetryNetwork,
    );

    const { user, ...app } = render(<InnerApp />, {
      preloadedState: {
        pages: [{ name: "login page" }, { name: "history" }],
        teams: {
          group: { id: "123", name: "test" },
          workspace: { ws_id: "123", root_group_name: "test" },
          skipped: false,
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
