import { render } from "../../utils/test-utils";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { ChatForm, ChatFormProps } from "./ChatForm";
import React from "react";
import { SYSTEM_PROMPTS } from "../../__fixtures__";

import {
  server,
  goodCaps,
  goodPrompts,
  noTools,
  noCommandPreview,
  noCompletions,
  goodPing,
} from "../../utils/mockServer";

const handlers = [
  goodCaps,
  goodPrompts,
  noTools,
  noCommandPreview,
  noCompletions,
  goodPing,
];

server.use(...handlers);

const noop = () => ({});

const App: React.FC<Partial<ChatFormProps>> = ({ ...props }) => {
  const defaultProps: ChatFormProps = {
    chatId: "chatId",
    onSubmit: (_str: string) => ({}),
    isStreaming: false,
    onSetChatModel: noop,
    model: "gpt-3.5-turbo",
    caps: {
      fetching: false,
      default_cap: "foo",
      available_caps: {},
      error: "",
    },
    showControls: true,
    prompts: SYSTEM_PROMPTS,
    onSetSystemPrompt: noop,
    onToolConfirm: noop,
    selectedSystemPrompt: {},
    ...props,
  };

  return <ChatForm {...defaultProps} />;
};

describe("ChatForm", () => {
  beforeEach(() => {
    server.use(...handlers);
  });

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
    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />, {
      preloadedState: { active_file: activeFile },
    });

    const label = app.queryByText(/Lookup symbols/);
    expect(label).not.toBeNull();
    const btn = label?.querySelector("button");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(btn!);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    const expected = `@file ${activeFile.path}:${activeFile.line1}-${activeFile.line2}\n@symbols-at ${activeFile.path}:${activeFile.cursor}\nfoo\n`;

    expect(fakeOnSubmit).toHaveBeenCalledWith(expected);
  });

  test("checkbox snippet", async () => {
    const fakeOnSubmit = vi.fn();
    const snippet = {
      language: "python",
      code: "print(1)",
      path: "/Users/refact/projects/print1.py",
      basename: "print1.py",
    };
    const { user, ...app } = render(
      <App chatId="test-3" onSubmit={fakeOnSubmit} />,
      {
        preloadedState: {
          selected_snippet: snippet,
          active_file: {
            name: "foo.txt",
            cursor: 2,
            path: "foo.txt",
            line1: 1,
            line2: 3,
            can_paste: true,
          },
          config: { host: "vscode", themeProps: {}, lspPort: 8001 },
        },
      },
    );

    const label = app.queryByText(/Selected \d* lines/);
    expect(label).not.toBeNull();
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const textarea = app.container.querySelector("textarea")!;
    await user.type(textarea, "foo");
    await user.keyboard("{Enter}");
    const markdown = "```python\nprint(1)\n```\n";
    const cursor = app.store.getState().active_file.cursor;

    const expected = `@file foo.txt:${
      cursor ? cursor + 1 : 1
    }\n${markdown}\nfoo\n`;
    expect(fakeOnSubmit).toHaveBeenCalledWith(expected);
  });

  test.each([
    "{Shift>}{enter>}{/enter}{/Shift}", // hold shift, hold enter, release enter, release shift,
    "{Shift>}{enter>}{/Shift}{/enter}", // hold shift,  hold enter, release enter, release shift,
  ])("when pressing %s, it should not submit", async (a) => {
    const fakeOnSubmit = vi.fn();

    const { user, ...app } = render(<App onSubmit={fakeOnSubmit} />);
    const textarea = app.container.querySelector("textarea");
    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, a);
    }
    expect(fakeOnSubmit).not.toHaveBeenCalled();
  });
});
