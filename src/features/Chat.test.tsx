import { expect, vi, describe, it, afterEach, beforeEach, test } from "vitest";
import {
  render,
  waitFor,
  postMessage,
  setUpCapsForChat,
  stubResizeObserver,
  setUpSystemPromptsForChat,
  cleanup,
  // screen,
} from "../utils/test-utils";
import { Chat } from "./Chat";
import {
  EVENT_NAMES_TO_CHAT,
  EVENT_NAMES_FROM_CHAT,
  RestoreChat,
  CreateNewChatThread,
  ChatErrorStreaming,
  ChatReceiveCapsError,
  ResponseToChat,
  ToolCall,
  ToolResult,
} from "../events";
import {
  MARS_ROVER_CHAT,
  STUB_CAPS_RESPONSE,
  SYSTEM_PROMPTS,
} from "../__fixtures__";

describe("Chat", () => {
  beforeEach(() => {
    stubResizeObserver();
    vi.spyOn(window, "postMessage").mockImplementation(postMessage);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("should send and receive messages from the window", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    const postMessageSpy = vi.spyOn(window, "postMessage");
    const windowSpy = vi.fn();
    window.addEventListener("message", windowSpy);

    const { user, ...app } = render(<Chat />);

    expect(postMessageSpy).toHaveBeenCalledWith(
      { type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS, payload: { id: "foo" } },
      "*",
    );

    setUpCapsForChat("foo");
    setUpSystemPromptsForChat("foo");

    const select = await app.findByTitle("chat model");

    expect(select.textContent).toContain("gpt-3.5-turbo");

    const textarea = app.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    await user.type(textarea, "hello");

    await user.keyboard("{Enter}");

    expect(postMessageSpy).toHaveBeenCalledWith(
      {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: {
          id: "foo",
          messages: [["user", "@local-notes-to-self\nhello\n"]],
          model: "", // not added because it's default
          title: "",
          attach_file: false,
        },
      },
      "*",
    );

    postMessage({
      type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
      payload: {
        id: "foo",
        choices: [
          {
            delta: {
              content: "",
              role: "assistant",
            },
            finish_reason: null,
            index: 0,
          },
        ],
        created: 1702552152.03,
        model: "gpt-3.5-turbo",
      },
    });

    postMessage({
      type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
      payload: {
        id: "foo",
        choices: [
          {
            delta: {
              content: "hello there",
              role: "assistant",
            },
            finish_reason: null,
            index: 0,
          },
        ],
        created: 1702552152.03,
        model: "gpt-3.5-turbo",
      },
    });

    postMessage({
      type: EVENT_NAMES_TO_CHAT.DONE_STREAMING,
      payload: { id: "foo" },
    });

    await waitFor(() => {
      expect(app.getAllByText("hello there")).not.toBeNull();
    });
  });

  it("can restore a chat", async () => {
    const app = render(<Chat />);
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    const restoreChatAction: RestoreChat = {
      type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
      payload: {
        id: "foo",
        chat: MARS_ROVER_CHAT,
      },
    };

    postMessage(restoreChatAction);

    const firstMessage = MARS_ROVER_CHAT.messages[0][1] as string;

    postMessage(restoreChatAction);

    await waitFor(() => expect(app.queryByText(firstMessage)).not.toBeNull());

    await waitFor(() => expect(app.queryByText(/Certainly!/)).not.toBeNull());
  });

  it("when creating a new chat I can select which model to use", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    // Missing props in jsdom
    // window.PointerEvent = class PointerEvent extends Event {};
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    window.HTMLElement.prototype.hasPointerCapture = vi.fn();
    window.HTMLElement.prototype.releasePointerCapture = vi.fn();

    const postMessageSpy = vi.spyOn(window, "postMessage");

    const { user, ...app } = render(<Chat />);

    const restoreChatAction: RestoreChat = {
      type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
      payload: {
        id: "foo",
        chat: {
          id: "bar",
          messages: [
            ["user", "hello"],
            ["assistant", "hello there"],
          ],
          title: "hello",
          model: "gpt-3.5-turbo",
        },
      },
    };

    postMessage(restoreChatAction);

    const userInput = await app.findByText("hello");
    expect(userInput.textContent).toContain("hello");

    expect(app.queryByTitle("chat model")).toBeNull();

    const createNewChatAction: CreateNewChatThread = {
      type: EVENT_NAMES_TO_CHAT.NEW_CHAT,
      payload: { id: "bar" },
    };

    postMessage(createNewChatAction);

    setUpCapsForChat("foo");

    await waitFor(() => expect(app.queryByTitle("chat model")).not.toBeNull(), {
      timeout: 1000,
    });
    await waitFor(() =>
      expect(
        app.queryByText(STUB_CAPS_RESPONSE.code_chat_default_model),
      ).not.toBeNull(),
    );

    await user.click(app.getByTitle("chat model"));

    await user.click(app.getByRole("option", { name: /test-model/i }));

    await waitFor(() => expect(app.queryByText("test-model")).not.toBeNull());

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");

    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{enter}");
    }

    expect(postMessageSpy).toHaveBeenCalledWith(
      {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: {
          id: "foo",
          messages: [["user", "@local-notes-to-self\nhello\n"]],
          model: "test-model",
          title: "",
          attach_file: false,
        },
      },
      "*",
    );
  });

  it("retry chat", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));
    const postMessageSpy = vi.spyOn(window, "postMessage");

    const { user, ...app } = render(<Chat />);

    const restoreChatAction: RestoreChat = {
      type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
      payload: {
        id: "foo",
        chat: {
          id: "bar",
          messages: [
            ["user", "hello"],
            ["assistant", "hello there"],
            ["user", "how are you?"],
            ["assistant", "fine"],
          ],
          title: "hello",
          model: "gpt-3.5-turbo",
        },
      },
    };

    postMessage(restoreChatAction);

    await waitFor(() => expect(app.queryByText("hello")).not.toBeNull());

    const retryButtons = app.queryAllByText("Retry");

    expect(retryButtons.length).toBe(2);

    await user.click(retryButtons[0]);

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");

    expect(textarea).not.toBeNull();
    if (textarea) {
      textarea.setSelectionRange(0, textarea.value.length);
      await user.type(textarea, "{Enter}");
    }

    expect(postMessageSpy).toHaveBeenLastCalledWith(
      {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: {
          id: "bar",
          messages: [["user", "hello"]],
          title: "hello",
          model: "gpt-3.5-turbo",
          attach_file: false,
        },
      },
      "*",
    );
  });

  it("chat error streaming", async () => {
    const chatId = "foo";
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    const app = render(<Chat />);

    const chatError: ChatErrorStreaming = {
      type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING,
      payload: {
        id: chatId,
        message: "whoops",
      },
    };

    postMessage(chatError);

    await waitFor(() => expect(app.queryByText(/whoops/)).not.toBeNull());
  });

  it("char error getting caps", async () => {
    const chatId = "foo";
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    const app = render(<Chat />);

    const chatError: ChatReceiveCapsError = {
      type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS_ERROR,
      payload: {
        id: chatId,
        message: "whoops error getting caps",
      },
    };

    postMessage(chatError);

    await waitFor(() => expect(app.queryByText(/whoops/)).not.toBeNull());
  });

  test("chat with different system prompt", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    // Missing props in jsdom
    // window.PointerEvent = class PointerEvent extends Event {};
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    window.HTMLElement.prototype.hasPointerCapture = vi.fn();
    window.HTMLElement.prototype.releasePointerCapture = vi.fn();

    const postMessageSpy = vi.spyOn(window, "postMessage");
    const windowSpy = vi.fn();
    window.addEventListener("message", windowSpy);

    const { user, ...app } = render(<Chat />);

    expect(postMessageSpy).toHaveBeenCalledWith(
      { type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS, payload: { id: "foo" } },
      "*",
    );

    setUpCapsForChat("foo");
    setUpSystemPromptsForChat("foo");

    const btn = await waitFor(
      () => app.getByTitle(SYSTEM_PROMPTS.default.text),
      { timeout: 1000 },
    );

    await user.click(btn);

    await user.click(app.getByText(/insert_jokes/i));

    const textarea = app.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    await user.type(textarea, "hello");

    await user.keyboard("{Enter}");

    expect(postMessageSpy).toHaveBeenCalledWith(
      {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: {
          id: "foo",
          title: "",
          model: "",
          attach_file: false,
          messages: [
            ["system", SYSTEM_PROMPTS.insert_jokes.text],
            ["user", "@local-notes-to-self\nhello\n"],
          ],
        },
      },
      "*",
    );
  });

  test("restore and receive response with use question", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));
    const { user: _user, ...app } = render(<Chat />);

    const restoreChatAction: RestoreChat = {
      type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
      payload: {
        id: "foo",
        chat: {
          id: "bar",
          messages: [
            ["user", "/shorter"],
            ["assistant", "hello there"],
            ["user", "even shorter still"],
          ],
          title: "hello",
          model: "gpt-3.5-turbo",
        },
      },
    };

    postMessage(restoreChatAction);

    await waitFor(() => expect(app.queryByText("hello there")).not.toBeNull());

    const file: ResponseToChat = {
      type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
      payload: {
        id: "bar",
        content:
          '[{"file_name":"/refact-chat-js/src/services/refact.ts","file_content":"hello","line1":121,"line2":451,"usefulness":100.0}]',
        role: "context_file",
      },
    };

    postMessage(file);

    const assistant: ResponseToChat = {
      type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
      payload: {
        id: "bar",
        role: "user",
        content: "even shorter still",
      },
    };

    postMessage(assistant);

    postMessage({
      type: EVENT_NAMES_TO_CHAT.DONE_STREAMING,
      payload: { id: "bar" },
    });

    await new Promise((r) => setTimeout(r, 500));

    const messages = app.getAllByText("even shorter still");
    expect(messages.length).toBe(1);

    expect(() => app.queryByText("hello there")).not.toBeNull();
  });

  test("Chat with functions", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));
    const postMessageSpy = vi.spyOn(window, "postMessage");

    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    window.HTMLElement.prototype.hasPointerCapture = vi.fn();
    window.HTMLElement.prototype.releasePointerCapture = vi.fn();

    const { user, ...app } = render(<Chat />);

    const toolCalls: ToolCall[] = [
      {
        id: "a",
        function: {
          name: "cat",
          arguments: JSON.stringify({ file: "meow.txt" }),
        },
        type: "function",
        index: 0,
      },
    ];

    const toolResult: ToolResult = {
      tool_call_id: "a",
      finish_reason: "call_worked",
      content: "meow\nmeow\n🐈\n",
    };

    const restoreChatAction: RestoreChat = {
      type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
      payload: {
        id: "foo",
        chat: {
          id: "bar",
          messages: [
            ["user", "hello"],
            ["assistant", "hello there", toolCalls],
            ["tool", toolResult],
          ],
          title: "hello",
          model: "gpt-3.5-turbo",
        },
      },
    };

    postMessage(restoreChatAction);

    const textarea = app.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    await user.type(textarea, "hello");

    await user.keyboard("{Enter}");

    expect(postMessageSpy).toHaveBeenCalledWith(
      {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: {
          id: "bar",
          title: "hello",
          model: "gpt-3.5-turbo",
          attach_file: false,
          messages: [
            ["user", "hello"],
            ["assistant", "hello there", toolCalls],
            ["tool", toolResult],
            ["user", "hello"],
          ],
        },
      },
      "*",
    );
  });
});
