import { expect, vi, describe, it } from "vitest";
import { render, stubResizeObserver } from "../utils/test-utils";
import { HistorySideBar } from "./HistorySideBar";
import { EVENT_NAMES_TO_CHAT } from "../events";
import { ChatHistoryItem } from "../hooks/useChatHistory";

describe("HistorySideBar", () => {
  stubResizeObserver();

  it("start new chat", async () => {
    const { user, ...app } = render(<HistorySideBar />);

    const postMessageSpy = vi.spyOn(window, "postMessage");

    const startNewChatButton = app.getByText("Start a new chat");

    await user.click(startNewChatButton);

    expect(postMessageSpy).toHaveBeenLastCalledWith(
      {
        type: EVENT_NAMES_TO_CHAT.NEW_CHAT,
        payload: {
          id: "",
        },
      },
      "*",
    );
  });

  it("restore chat", async () => {
    vi.mock("uuid", () => ({ v4: () => "bar" }));
    const postMessageSpy = vi.spyOn(window, "postMessage");

    const historyItem: ChatHistoryItem = {
      id: "foo",
      createdAt: "",
      lastUpdated: "",
      messages: [
        ["user", "hello"],
        ["assistant", "hello there"],
      ],
      title: "Title for the item",
      model: "chat-gpt-3.5-turbo",
    };

    window.localStorage.setItem("chatHistory", JSON.stringify([historyItem]));

    const { user, ...app } = render(<HistorySideBar />);

    const restoreButton = await app.findByText("Title for the item");
    // expect(restoreButton).toBeInTheDocument(); //TODO: setup matchers
    await user.click(restoreButton);

    expect(postMessageSpy).toHaveBeenLastCalledWith(
      {
        type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
        payload: {
          id: "",
          chat: historyItem,
        },
      },
      "*",
    );
  });

  it("can remove an item", async () => {
    const historyItems: ChatHistoryItem[] = [
      {
        id: "foo",
        createdAt: "",
        lastUpdated: "",
        messages: [
          ["user", "hello"],
          ["assistant", "hello there"],
        ],
        title: "Title for the item",
        model: "chat-gpt-3.5-turbo",
      },
      {
        id: "bar",
        createdAt: "",
        lastUpdated: "",
        messages: [
          ["user", "hello"],
          ["assistant", "hello there"],
        ],
        title: "Delete this item",
        model: "chat-gpt-3.5-turbo",
      },
    ];

    window.localStorage.setItem("chatHistory", JSON.stringify(historyItems));

    const { user, ...app } = render(<HistorySideBar />);

    const itemTitleToDelete = "Delete this item";

    const restoreButtonText = await app.findByText(itemTitleToDelete);

    const deleteButton =
      restoreButtonText.parentElement?.parentElement?.querySelector(
        '[title="delete chat"]',
      );
    expect(deleteButton).not.toBeNull();

    if (deleteButton) {
      await user.click(deleteButton);
    }

    const history = localStorage.getItem("chatHistory") as unknown as string;

    const json = JSON.parse(history) as ChatHistoryItem[];

    expect(json.length).toEqual(1);

    const maybeItem = json.find((item) => item.title === itemTitleToDelete);

    expect(maybeItem).toBeUndefined();
  });
});
