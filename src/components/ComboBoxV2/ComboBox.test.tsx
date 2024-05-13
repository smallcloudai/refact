import React from "react";
import { describe, test, vi, expect, afterEach } from "vitest";
import { render, cleanup } from "../../utils/test-utils";
import { ComboBox, ComboBoxProps } from "./ComboBox";
import { TextArea, type TextAreaProps } from "../TextArea";
import type { Commands } from "./ComboBox";

const defaultCommands = ["@file", "@workspace"];
const defaultArgs = ["/foo", "/bar"];

// replace fakeRequestCommands with this to run with the lsp
// async function getCommands(query: string, cursor: number) {
//   return fetch("http://127.0.0.1:8001/v1/at-command-completion", {
//     method: "POST",
//     body: JSON.stringify({ query, cursor, top_n: 5 }),
//   })
//     .then((res) => res.json())
//     .then((json) => json as Commands)
//     .catch((err) => {
//       // eslint-disable-next-line no-console
//       console.error(err);
//     });
// }

const App = (props: Partial<ComboBoxProps>) => {
  const [value, setValue] = React.useState<string>(props.value ?? "");
  const [commands, setCommands] = React.useState<Commands>({
    completions: [],
    replace: [0, 0],
    is_cmd_executable: false,
  });

  const fakeRequestCommands = React.useCallback(
    (query: string, cursor: number) => {
      if (query === "@" && cursor === 1) {
        setCommands({
          completions: defaultCommands,
          replace: [0, cursor],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@file " && cursor === 6) {
        setCommands({
          completions: ["/foo", "/bar"],
          replace: [6, 6],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@\nhello" && cursor === 1) {
        setCommands({
          completions: defaultCommands,
          replace: [0, 1],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@file\nhello" && cursor === 5) {
        setCommands({
          completions: [],
          replace: [-1, -1],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@file \nhello" && cursor === 6) {
        setCommands({
          completions: defaultArgs,
          replace: [6, 6],
          is_cmd_executable: false,
        });
        return;
      }

      // TODO how does an exicutable comand respond?

      if (
        (query === "@f" && cursor === 2) ||
        (query === "@fil" && cursor === 4) ||
        (query === "@fi" && cursor === 3)
      ) {
        setCommands({
          completions: ["@file"],
          replace: [0, query.length],
          is_cmd_executable: true,
        });
        return;
      }

      if (query === "@file f" && cursor === 7) {
        setCommands({
          completions: defaultArgs,
          replace: [6, 7],
          is_cmd_executable: true,
        });

        return;
      }

      if (query === "@file" && cursor === 5) {
        setCommands({
          completions: [],
          replace: [-1, -1],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@file /foo" && cursor === 10) {
        setCommands({
          completions: [],
          replace: [-1, -1],
          is_cmd_executable: false,
        });
        return;
      }

      if (
        (query === "@file /fo" && cursor === 9) ||
        (query === "@file /f" && cursor === 8) ||
        (query === "@file /" && cursor === 7)
      ) {
        setCommands({
          completions: defaultArgs,
          replace: [-1, -1],
          is_cmd_executable: false,
        });
        return;
      }

      if (query === "@file /foo\n@" && cursor === 12) {
        setCommands({
          completions: defaultCommands,
          replace: [11, cursor],
          is_cmd_executable: false,
        });
        return;
      }

      // Use to run with the lsp
      // console.log({ query, cursor });
      // void getCommands(query, cursor).then((commands) => {
      //   if (commands) {
      //     console.log({ commands });
      //     setCommands(commands);
      //   }
      // });
      // return

      setCommands({
        completions: [],
        replace: [-1, -1],
        is_cmd_executable: false,
      });
    },
    [],
  );

  const defaultProps: ComboBoxProps = {
    commands,
    requestCommandsCompletion: fakeRequestCommands,
    onSubmit: () => ({}),
    value: value,
    onChange: setValue,
    placeholder: "Type @ for commands",
    render: (props: TextAreaProps) => <TextArea {...props} />,
    ...props,
  };

  return <ComboBox {...defaultProps} />;
};

describe("ComboBox", () => {
  afterEach(cleanup);
  test("type @ and select command and arguments by clicking", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    const commandButton = app.getByText("@file");

    await user.click(commandButton);

    await user.type(textarea, " ");

    const argumentsButton = app.getByText("/bar");
    await user.click(argumentsButton);
    const result = app.getByRole("combobox");
    expect(result.textContent).toBe("@file /bar");
  });

  test("deleting while typing a command", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Tab} f{Tab}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.keyboard("{Backspace}");
    expect(app.queryByText("/foo")).not.toBeNull();
    await user.keyboard("{Backspace}");
    await user.keyboard("{Backspace}");
    expect(app.queryByText("/bar")).not.toBeNull();
  });

  test("delete part of a command and press tab", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await user.keyboard("{Tab}");
    expect(textarea.textContent).toEqual("@file");
    await user.keyboard("{Backspace}{BackSpace}");
    await user.keyboard("{Tab}");
    expect(textarea.textContent).toEqual("@file");
  });

  test("completes when pressing tab", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Tab} f{Tab}");
    expect(app.getByRole("combobox").textContent).toEqual("@file /foo");
  });

  test("completes when pressing enter", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await user.keyboard("{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file /foo");
  });

  test("type part of the command, then press ender", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@fi{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file");
  });

  test("multiple commands", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.type(textarea, "{Shift>}{Enter}{/Shift}");
    await user.type(textarea, "@");

    await user.keyboard("{ArrowDown}{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual(
      "@file /foo\n@workspace",
    );
  });

  test("typing @ and tab space then tab it should complete the command and argument", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Tab} f{Tab}");
    const result = app.getByRole("combobox").textContent;
    const expected = "@file /foo";
    expect(result).toEqual(expected);
  });

  test("typing @ and enter space then enter, should complete the command and argument", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("f{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file /foo");
  });

  test("submit when pressing enter", async () => {
    const onSubmitSpy = vi.fn();
    const { user, ...app } = render(<App onSubmit={onSubmitSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "hello");
    await user.keyboard("{Enter}");
    expect(onSubmitSpy).toHaveBeenCalled();
  });

  test("select and submit command", async () => {
    const onSubmitSpy = vi.fn();
    const { user, ...app } = render(<App onSubmit={onSubmitSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.type(textarea, "{Shift>}{Enter}{/Shift}");
    await user.type(textarea, "what's this?");
    await user.keyboard("{Enter}");
    if (onSubmitSpy.mock.calls.length === 0) {
      app.debug();
    }
    expect(onSubmitSpy).toHaveBeenCalled();
  });

  test("select command, type space and then delete the command", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@fi{Enter}");
    expect(textarea.textContent).toEqual("@file");
    await user.type(textarea, " ");
    expect(app.queryByText("/foo")).not.toBeNull();
    expect(app.queryByText("/bar")).not.toBeNull();
    await user.keyboard("{Backspace}");
    await user.keyboard("{Backspace}");
    await user.keyboard("{Backspace}");
    await user.keyboard("{Backspace}");
    await user.keyboard("{Backspace}");
    expect(app.queryByText("/foo")).toBeNull();
    expect(app.queryByText("/bar")).toBeNull();
    expect(app.queryByText("@workspace")).not.toBeNull();
  });

  test("change a command after typing", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file");
    await user.keyboard("{Backspace>4}");
    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@workspace");

    // TODO: deleting between the lines
    // await user.keyboard("{Shift>}{Enter}{/Shift}");
    // await user.keyboard("hello");
    // expect(textarea.textContent).toEqual("@file /bar\nhello");
    // await user.type(textarea, "{ArrowLeft>6}{Backspace>3}");
    // await user.keyboard("{Enter}");
    // expect(textarea.textContent).toEqual("@file /foo\nhello");
  });

  test("undo/redo mac command key", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.keyboard("{Meta>}{z}");
    expect(textarea.textContent).toEqual("@file ");
    await user.keyboard("{z}");
    await user.keyboard("{z}");
    expect(textarea.textContent).toEqual("@");
    await user.keyboard("{z}{/Meta}");
    expect(textarea.textContent).toEqual("");

    await user.keyboard("{Shift>}{Meta>}{z}");
    expect(textarea.textContent).toEqual("@");

    await user.keyboard("{z}");
    await user.keyboard("{z}");
    expect(textarea.textContent).toEqual("@file ");

    await user.keyboard("{z}{/Meta}{/Shift}");
    expect(textarea.textContent).toEqual("@file /foo");
  });

  test("undo/redo windows ctrl key", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.keyboard("{Control>}{z}");
    expect(textarea.textContent).toEqual("@file ");
    await user.keyboard("{z}");
    await user.keyboard("{z}");
    expect(textarea.textContent).toEqual("@");
    await user.keyboard("{z}{/Control}");
    expect(textarea.textContent).toEqual("");

    await user.keyboard("{Shift>}{Control>}{z}");
    expect(textarea.textContent).toEqual("@");
    await user.keyboard("{z}");
    await user.keyboard("{z}");
    expect(textarea.textContent).toEqual("@file ");

    await user.keyboard("{z}{/Control}{/Shift}");
    expect(textarea.textContent).toEqual("@file /foo");
  });

  test("@, enter, space, enter, ctrl+z, enter", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    await user.keyboard("{Enter}");
    await user.type(textarea, " ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.keyboard("{Control>}{z}{/Control}");
    expect(textarea.textContent).toEqual("@file ");
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
  });

  test("insert command before text", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "\nhello");
    await user.type(textarea, "@", {
      initialSelectionEnd: 0,
      initialSelectionStart: 0,
    });
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file\nhello");

    await user.type(textarea, " ", {
      initialSelectionStart: 5,
      initialSelectionEnd: 5,
    });
    await user.keyboard("{Enter}");
    expect(textarea.textContent).toEqual("@file /foo\nhello");
  });

  test("it should close when pressing escape", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@");
    expect(app.queryByText("@file")).not.toBeNull();
    await user.keyboard("{Escape}");
    expect(app.queryByText("@file")).toBeNull();
  });

  // test("textarea should be empty after submit", async () => {
  //   const submitSpy = vi.fn();
  //   const { user, ...app } = render(<App onSubmit={submitSpy} />);
  //   const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
  //   await user.type(textarea, "hello");
  //   await user.keyboard("{Enter}");
  //   expect(submitSpy).toHaveBeenCalled();
  //   expect(textarea.textContent).toEqual("");
  // });
});
