import { render } from "../utils/test-utils";
import { describe, expect, it } from "vitest";
import {
  server,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
} from "../utils/mockServer";
import { InnerApp } from "../features/App";

describe.skip("Delete a Chat form history", () => {
  server.use(goodUser, goodPing, chatLinks, telemetryChat, telemetryNetwork);
  it("can delete a chat", async () => {
    const { user, store, ...app } = render(<InnerApp />, {
      preloadedState: {
        teams: {
          group: { id: "123", name: "test" },
        },
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

    const itemTitleToDelete = "Test title";

    const restoreButtonText = await app.findByText(itemTitleToDelete);

    const deleteButton =
      restoreButtonText.parentElement?.parentElement?.parentElement?.querySelector(
        '[title="delete chat"]',
      );

    expect(deleteButton).not.toBeNull();

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(deleteButton!);

    expect(store.getState()).toEqual({});
  });
});
