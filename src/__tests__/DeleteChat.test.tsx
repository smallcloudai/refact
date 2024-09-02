import { render } from "../utils/test-utils";
import { describe, expect, it } from "vitest";
import { server, goodUser } from "../utils/mockServer";
import { InnerApp } from "../features/App";
import { HistoryState } from "../features/History/historySlice";

describe("Delete a Chat form history", () => {
  server.use(goodUser);
  it("can delete a chat", async () => {
    const now = new Date().toISOString();
    const history: HistoryState = {
      abc123: {
        title: "Test title",
        messages: [],
        id: "abc123",
        model: "foo",
        createdAt: now,
        updatedAt: now,
        read: true,
      },
    };
    const { user, store, ...app } = render(<InnerApp />, {
      preloadedState: {
        history,
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

    expect(store.getState().history).toEqual({});
  });
});
