import { render } from "../../utils/test-utils";
import { describe, expect, test, vi } from "vitest";
import { ChatForm, ChatFormProps } from "./ChatForm";
import React from "react";
import { SYSTEM_PROMPTS } from "../../__fixtures__";
import { useDebounceCallback } from "usehooks-ts";

const noop = () => ({});

const App: React.FC<Partial<ChatFormProps>> = ({ ...props }) => {
  const defaultProps: ChatFormProps = {
    // removePreviewFileByName: noop,
    chatId: "chatId",
    selectedSnippet: { code: "", language: "", path: "", basename: "" },
    onSubmit: (_str: string) => ({}),
    isStreaming: false,
    onStopStreaming: noop,
    onSetChatModel: noop,
    model: "gpt-3.5-turbo",
    caps: {
      fetching: false,
      default_cap: "foo",
      available_caps: {},
      error: "",
    },
    // error: "",
    // clearError: noop,
    showControls: true,
    // hasContextFile: false,
    commands: {
      completions: [],
      replace: [-1, -1],
      is_cmd_executable: false,
    },
    requestCommandsCompletion: useDebounceCallback(noop, 0),
    requestPreviewFiles: noop,
    attachFile: {
      name: "",
      line1: null,
      line2: null,
      can_paste: false,
      attach: false,
      path: "",
      cursor: null,
    },
    // setSelectedCommand: noop,
    filesInPreview: [],
    onTextAreaHeightChange: noop,
    // requestCaps: noop,
    prompts: SYSTEM_PROMPTS,
    onSetSystemPrompt: noop,
    selectedSystemPrompt: {},
    canUseTools: false,
    setUseTools: noop,
    useTools: false,
    ...props,
  };

  // return (

  //   <ConfigProvider
  //     config={{
  //       host: host ?? "web",
  //       features: {
  //         vecdb: true,
  //         ast: true,
  //       },
  //     }}
  //   >
  //     <ChatForm {...defaultProps} />
  //   </ConfigProvider>
  // );
  // TODO: use store provider
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
    const expected = "@workspace\nfoo\n";
    expect(fakeOnSubmit).toHaveBeenCalledWith(expected);
  });

  test.skip("checkbox lookup symbols", async () => {
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
    const epexted = `@file ${activeFile.path}:${activeFile.line1}-${activeFile.line2}\n@symbols-at ${activeFile.path}:${activeFile.cursor}\nfoo\n`;

    expect(fakeOnSubmit).toHaveBeenCalledWith(epexted);
  });

  // TODO: fix this test because the host is not set in redux
  test.skip("checkbox snippet", async () => {
    // skipped because if the snippet is there on the first render it's automatically appened
    const fakeOnSubmit = vi.fn();
    const snippet = {
      language: "python",
      code: "print(1)",
      path: "/Users/refact/projects/print1.py",
      basename: "print1.py",
    };
    const { user, ...app } = render(
      <App
        onSubmit={fakeOnSubmit}
        // host="ide"
      />,
    );

    app.rerender(
      <App
        onSubmit={fakeOnSubmit}
        selectedSnippet={snippet}
        // host="ide"
      />,
    );

    const label = app.queryByText(/Selected \d* lines/);
    app.debug();
    expect(label).not.toBeNull();
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    const markdown = "```python\nprint(1)\n```\n";
    const expected = `${markdown}\nfoo\n`;
    expect(fakeOnSubmit).toHaveBeenCalledWith(expected);
  });
});
