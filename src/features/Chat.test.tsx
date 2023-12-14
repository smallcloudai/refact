import { expect, vi, describe, it } from "vitest";
import { render, waitFor } from "../utils/test-utils";
import { Chat } from "./Chat";

Element.prototype.scrollTo = () => ({});

describe("Chat", () => {
  it("should send  and recive messages from the window", async () => {
    vi.mock("uuid", () => ({ v4: () => "foo" }));

    const { user, ...app } = render(<Chat />);

    const postMessageSpy = vi.spyOn(window, "postMessage");
    const windowSpy = vi.fn();
    window.addEventListener("message", windowSpy);

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");

    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{enter}");
    }

    expect(postMessageSpy).toHaveBeenLastCalledWith(
      {
        type: "chat_question",
        payload: {
          id: "foo",
          messages: [["user", "hello\n"]],
        },
      },
      "*",
    );

    window.postMessage(
      {
        type: "chat_response",
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
      },
      "*",
    );

    window.postMessage(
      {
        type: "chat_response",
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
      },
      "*",
    );

    await waitFor(() => {
      expect(app.getAllByText("hello there")).not.toBeNull();
    });
  });
});
