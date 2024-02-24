import { render } from "../../utils/test-utils";
import { describe, expect, test, vi } from "vitest";
import { ChatForm, ChatFormProps } from "./ChatForm";
import React from "react";

const noop = () => ({});

const App: React.FC<Partial<ChatFormProps>> = (props) => {
  const defaultProps: ChatFormProps = {
    removePreviewFileByName: noop,
    selectedSnippet: { code: "", language: "" },
    onSubmit: noop,
    isStreaming: false,
    onStopStreaming: noop,
    onSetChatModel: noop,
    model: "gpt-3.5-turbo",
    caps: { fetching: false, default_cap: "foo", available_caps: [] },
    error: "",
    clearError: noop,
    canChangeModel: false,
    hasContextFile: false,
    commands: {
      available_commands: [],
      selected_command: "",
      arguments: [],
      is_cmd_executable: false,
    },
    requestCommandsCompletion: noop,
    attachFile: {
      name: "",
      can_paste: false,
      attach: false,
    },
    setSelectedCommand: noop,
    filesInPreview: [],
    onTextAreaHeightChange: noop,
    ...props,
  };

  return <ChatForm {...defaultProps} />;
};

describe("ChatForm", () => {
  test("when I push enter it should call onSubmit", async () => {
    const fakeOnSubmit = vi.fn();

    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />);

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{Enter}");
    }

    expect(fakeOnSubmit).toHaveBeenCalled();
  });

  test("when I hole shift and push enter it should not call onSubmit", async () => {
    const fakeOnSubmit = vi.fn();

    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />);
    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{Shift>}{enter}{/Shift}");
    }
    expect(fakeOnSubmit).not.toHaveBeenCalled();
  });
});
