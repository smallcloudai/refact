import React from "react";
import { describe, test, vi, expect, afterEach } from "vitest";
import { render, cleanup } from "../../utils/test-utils";
import { ComboBox, ComboBoxProps } from "./ComboBox";
import { TextArea, type TextAreaProps } from "../TextArea";

const App = (props: Partial<ComboBoxProps>) => {
  const [value, setValue] = React.useState<string>(props.value ?? "");
  const requestCompletionSpy = vi.fn();
  const onSubmitSpy = vi.fn();
  const defaultProps = {
    commands: ["@file"],
    requestCommandsCompletion: requestCompletionSpy,
    commandArguments: ["/foo", "/bar"],
    value: value,
    onChange: setValue,
    onSubmit: onSubmitSpy,
    placeholder: "Type @ for commands",
    render: (props: TextAreaProps) => <TextArea {...props} />,
    ...props,
  };

  return <ComboBox {...defaultProps} />;
};

describe("ComboBox", () => {
  afterEach(cleanup);
  test("type @ and select command and arguments by clicking", async () => {
    const submitSpy = vi.fn();
    const { user, ...app } = render(<App onSubmit={submitSpy} />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "foo{Shift>}{Enter}{/Shift}@");
    const commandButton = app.getByText("@file");

    await user.click(commandButton);
    const argumentsButton = app.getByText("/bar");
    await user.click(argumentsButton);
    const result = app.getByRole("combobox");
    expect(result.textContent).toBe("foo\n@file /bar");
    // await user.type(result, "{Enter}");
    // // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    // expect(submitSpy.mock.lastCall[0].target.value).toBe("foo\n@file /bar\n");
  });

  test.skip("deleting over arguments of a command", async () => {
    const { user, ...app } = render(<App />);
    const textarea = app.getByRole("combobox");
    await user.type(textarea, "foo {Shift>}{Enter}{/Shift}@");
    const commandButton = app.getByText("@file");

    await user.click(commandButton);
    const argumentsButton = app.getByText("/bar");
    await user.click(argumentsButton);
    expect(app.queryByText("/bar")).toBeNull();
    await user.keyboard("{Backspace}");
    expect(app.queryByText("/bar")).not.toBeNull();
    await user.keyboard(
      "{Backspace}{Backspace}{Backspace}{Backspace}{Backspace}",
    );
    expect(app.queryByText("@file")).not.toBeNull();
    await user.keyboard(
      "{Backspace}{Backspace}{Backspace}{Backspace}{Backspace}",
    );
    expect(app.queryByText("@file")).toBeNull();
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
    await user.type(textarea, "@fi{Enter}");
    expect(app.getByRole("combobox").textContent).toEqual("@file ");
    await user.keyboard("{Enter}");
    await user.type(textarea, " hello @");
    await user.keyboard("{Enter}");
    await user.keyboard("{Enter}");
    app.debug();
    expect(app.getByRole("combobox").textContent).toEqual(
      "@file /foo hello @file /foo",
    );
  });

  test.todo("executable commands");
  test.todo("typing executable command and pressing space");
  test.todo("submit");
});
