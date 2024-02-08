import React from "react";
import { describe, test, vi, expect, afterEach } from "vitest";
import { render, cleanup } from "../../utils/test-utils";
import { ComboBox, ComboBoxProps } from "./ComboBox";
import { TextArea, type TextAreaProps } from "../TextArea";

const App = (props: Partial<ComboBoxProps>) => {
  const [value, setValue] = React.useState<string>(props.value ?? "");
  const [selectedCommand, setSelectedCommand] = React.useState<string>("");
  const [commandIsExecutable, setCommandIsExecutable] =
    React.useState<boolean>(false);

  React.useEffect(() => {
    if (
      selectedCommand === "@workspace " ||
      selectedCommand === "@file /bar" ||
      selectedCommand === "@file /foo"
    ) {
      setCommandIsExecutable(true);
    } else {
      setCommandIsExecutable(false);
    }
  }, [selectedCommand, value]);

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
    commandIsExecutable,
    executeCommand: () => ({}),
    selectedCommand,
    setSelectedCommand,
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
    await user.type(textarea, "@");
    await user.keyboard("{Tab}");
    await user.keyboard("{Tab}");
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
    await user.type(textarea, "@");
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
    await user.type(textarea, "foo{Shift>}{Enter}{/Shift}@");
    await user.keyboard("{Tab}");
    await user.keyboard("{Tab}");
    expect(app.getByRole("combobox").textContent).toEqual("foo\n@file /foo");
  });

  test("completes when pressing enter", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "foo{Shift>}{Enter}{/Shift}@");
    await user.keyboard("{Enter}");
    await user.keyboard("{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("foo\n@file /foo");
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
    await user.keyboard("{Enter}");
    await user.type(textarea, "@wo{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual(
      "@file /foo\n@workspace \n",
    );
  });

  test.todo(
    "typing command and pressing tab or enter twice, should complete the command and argument",
  );

  test("clicking on an executable command", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(<App executeCommand={executableSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@");

    const commandButton = app.getByText("@workspace");
    await user.click(commandButton);

    expect(executableSpy).toHaveBeenCalledWith("@workspace ");
  });

  test("execute command when pressing enter", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(<App executeCommand={executableSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@wo{Enter}");
    expect(executableSpy).toHaveBeenCalledWith("@workspace ");
  });

  test("execute command when pressing tab", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(<App executeCommand={executableSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@wo{Tab}");
    expect(executableSpy).toHaveBeenCalledWith("@workspace ");
  });

  test("typing executable command and pressing space", async () => {
    const executableSpy = vi.fn();
    const { user, ...app } = render(<App executeCommand={executableSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "@workspace{Space}");
    expect(executableSpy).toHaveBeenCalledWith("@workspace ");
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
});
