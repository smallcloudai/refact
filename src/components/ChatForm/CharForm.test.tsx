import { render } from "../../utils/test-utils";
import { describe, expect, test, vi } from "vitest";
import { ChatForm } from "./ChatForm";

describe("ChatForm", () => {
  test("when I push enter it should call onSubmit", async () => {
    const fakeOnSubmit = vi.fn();

    const { user, ...app } = render(
      <ChatForm
        onSubmit={fakeOnSubmit}
        isStreaming={false}
        onStopStreaming={vi.fn}
        onSetChatModel={vi.fn}
        model="gpt-3.5-turbo"
        caps={{ fetching: false, default_cap: "foo", available_caps: [] }}
        error=""
        clearError={vi.fn}
        canChangeModel={false}
        handleContextFile={vi.fn}
        hasContextFile={false}
      />,
    );

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{enter}");
    }

    expect(fakeOnSubmit).toHaveBeenCalled();
  });

  test("when I hole shift and push enter it should not call onSubmit", async () => {
    const fakeOnSubmit = vi.fn();

    const { user, ...app } = render(
      <ChatForm
        onSubmit={fakeOnSubmit}
        isStreaming={false}
        onStopStreaming={vi.fn}
        onSetChatModel={vi.fn}
        model="gpt-3.5-turbo"
        caps={{ fetching: false, default_cap: "foo", available_caps: [] }}
        error=""
        clearError={vi.fn}
        canChangeModel={false}
        handleContextFile={vi.fn}
        hasContextFile={false}
      />,
    );
    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{Shift>}{enter}{/Shift}");
    }
    expect(fakeOnSubmit).not.toHaveBeenCalled();
  });
});
