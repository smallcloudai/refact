import { render } from "../../utils/test-utils";
import { describe, expect, test, vi } from "vitest";
import { ChatForm, ChatFormProps } from "./ChatForm";
import React from "react";

const noop = () => ({});

const App: React.FC<Partial<ChatFormProps>> = (props) => {
  const defaultProps: ChatFormProps = {
    removePreviewFileByName: noop,
    selectedSnippet: { code: "", language: "", path: "", basename: "" },
    onSubmit: noop,
    isStreaming: false,
    onStopStreaming: noop,
    onSetChatModel: noop,
    model: "gpt-3.5-turbo",
    caps: {
      fetching: false,
      default_cap: "foo",
      available_caps: [],
      error: "",
    },
    error: "",
    clearError: noop,
    showControls: true,
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
      line1: null,
      line2: null,
      can_paste: false,
      attach: false,
      path: "",
      cursor: null,
    },
    setSelectedCommand: noop,
    filesInPreview: [],
    onTextAreaHeightChange: noop,
    requestCaps: noop,
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

  test("checkbox workspace", async () => {
    const fakeOnSubmit = vi.fn();
    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />);

    const label = app.queryByText("Search workspace");
    expect(label).not.toBeNull();
    const btn = label?.querySelector("button");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(btn!);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    expect(fakeOnSubmit).toHaveBeenCalledWith("foo\n@workspace\n");
  });

  test("checkbox lookup symbols", async () => {
    const fakeOnSubmit = vi.fn();
    const activeFile = {
      name: "foo.txt",
      line1: 1,
      line2: 2,
      can_paste: false,
      attach: false,
      path: "path/to/foo.txt",
      cursor: 2,
    };
    const { user, ...app } = render(
      <App onSubmit={fakeOnSubmit} attachFile={activeFile} />,
    );

    const label = app.queryByText(/Lookup symbols/);
    expect(label).not.toBeNull();
    const btn = label?.querySelector("button");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(btn!);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    expect(fakeOnSubmit).toHaveBeenCalledWith(
      `foo\n@symbols-at ${activeFile.path}:${activeFile.cursor}\n`,
    );
  });

  test("checkbox snippet", async () => {
    // skipped because if the snippet is there on the first render it's automatically appened
    const fakeOnSubmit = vi.fn();
    const snippet = {
      language: "python",
      code: "print(1)",
      path: "/Users/refact/projects/print1.py",
      basename: "print1.py",
    };
    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />);

    app.rerender(<App onSubmit={fakeOnSubmit} selectedSnippet={snippet} />);

    const label = app.queryByText(/Selected lines/);
    expect(label).not.toBeNull();
    const btn = label?.querySelector("button");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(btn!);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    const markdown = "```python\nprint(1)\n```\n";
    expect(fakeOnSubmit).toHaveBeenCalledWith(`foo\n${markdown}\n`);
  });
});
