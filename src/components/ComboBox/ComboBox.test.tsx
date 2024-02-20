import React from "react";
import { describe, test, vi, expect, afterEach } from "vitest";
import { render, cleanup, waitFor } from "../../utils/test-utils";
import { ComboBox, ComboBoxProps } from "./ComboBox";
import { TextArea, type TextAreaProps } from "../TextArea";

const App = (props: Partial<ComboBoxProps>) => {
  const [value, setValue] = React.useState<string>(props.value ?? "");
  const [selectedCommand, setSelectedCommand] = React.useState<string>("");

  const requestCompletionSpy = vi.fn();
  const onSubmitSpy = vi.fn();
  const defaultProps: ComboBoxProps = {
    commands: ["@file", "@workspace"],
    requestCommandsCompletion: requestCompletionSpy,
    commandArguments: ["/foo", "/bar"],
    value: value,
    onChange: setValue,
    onSubmit: onSubmitSpy,
    placeholder: "Type @ for commands",
    render: (props: TextAreaProps) => <TextArea {...props} />,
    selectedCommand,
    setSelectedCommand,
    removePreviewFileByName: () => ({}),
    ...props,
  };

  return <ComboBox {...defaultProps} />;
};

describe("ComboBox", () => {
  afterEach(cleanup);
  test("type @ and select command and arguments by clicking", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "foo{Shift>}{Enter}{/Shift}@");
    const commandButton = app.getByText("@file");

    await user.click(commandButton);
    const argumentsButton = app.getByText("/bar");
    await user.click(argumentsButton);
    const result = app.getByRole("combobox");
    expect(result.textContent).toBe("foo\n@file /bar");
  });

  test("deleting while typing a command", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Tab}f{Tab}");
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
    expect(textarea.textContent).toEqual("@file ");
    await user.type(textarea, "{Backspace}{BackSpace}");
    expect(textarea.textContent).toEqual("@fil");
    await user.keyboard("{Tab}");
    expect(textarea.textContent).toEqual("@file ");
  });

  test("completes when pressing tab", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "foo{Shift>}{Enter}{/Shift}@f{Tab}f{Tab}");
    expect(app.getByRole("combobox").textContent).toEqual("foo\n@file /foo");
  });

  test("completes when pressing enter", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await waitFor(() => app.getByText("@file"));
    await user.keyboard("{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file ");
    await waitFor(() => app.getByText("/foo"));
    await user.type(textarea, "/f{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file /foo");
  });

  test("type part of the command, then press ender", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@fi{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file ");
  });

  test("multiple commands", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Enter}");
    await user.keyboard("/{Enter}");
    expect(textarea.textContent).toEqual("@file /foo");
    await user.type(textarea, "{Shift>}{Enter}{/Shift}");
    await user.type(textarea, "@wo{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual(
      "@file /foo\n@workspace ",
    );
  });

  test("typing @ and tab twice, should complete the command and argument", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f{Tab}f{Tab}");
    const result = app.getByRole("combobox").textContent;
    const expected = "@file /foo";
    expect(result).toEqual(expected);
  });

  test("typing @ and enter twice, should complete the command and argument", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@f");
    await user.keyboard("{Enter}");
    await user.keyboard("f{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file /foo");
  });

  test("clicking on an executable command", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(
      <App requestCommandsCompletion={executableSpy} />,
    );
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@");

    const commandButton = app.getByText("@workspace");
    await user.click(commandButton);

    expect(executableSpy).toHaveBeenLastCalledWith(
      "@workspace ",
      11,
      "@workspace ",
    );
  });

  test("execute command when pressing enter", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(
      <App requestCommandsCompletion={executableSpy} />,
    );
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@wo{Enter}");
    expect(executableSpy).toHaveBeenLastCalledWith(
      "@workspace ",
      11,
      "@workspace ",
    );
  });

  test("execute command when pressing tab", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(
      <App requestCommandsCompletion={executableSpy} />,
    );
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@wo{Tab}");
    expect(executableSpy).toHaveBeenLastCalledWith(
      "@workspace ",
      11,
      "@workspace ",
    );
  });

  test("typing executable command and pressing space", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(
      <App requestCommandsCompletion={executableSpy} />,
    );
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@workspace{Space}");
    expect(executableSpy).toHaveBeenLastCalledWith(
      "@workspace ",
      11,
      "@workspace ",
    );
  });

  test("submit when pressing enter", async () => {
    const onSubmitSpy = vi.fn();
    const { user, ...app } = render(<App onSubmit={onSubmitSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "hello{Enter}");

    expect(onSubmitSpy).toHaveBeenCalled();

    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const event = onSubmitSpy.mock.lastCall[0];
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    expect(event.target.value).toEqual("hello\n");
  });

  test("select command, type / and then delete", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@fi{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file ");
    await user.type(textarea, "/");
    expect(app.queryByText("/foo")).not.toBeNull();
    expect(app.queryByText("/bar")).not.toBeNull();
    await user.type(textarea, "{Backspace}");
    expect(app.queryByText("/foo")).not.toBeNull();
    expect(app.queryByText("/bar")).not.toBeNull();
  });

  test("change a command after typing", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox") as HTMLTextAreaElement;
    await user.type(textarea, "@file /bar");

    await user.type(textarea, "{Shift>}{Enter}{/Shift}hello");
    expect(textarea.textContent).toEqual("@file /bar\nhello");
    await user.keyboard(
      "{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}",
    );
    await user.keyboard("{Backspace}{Backspace}{Backspace}");
    await user.keyboard("f{Enter}");
    expect(textarea.textContent).toEqual("@file /foo\nhello");
  });
});
