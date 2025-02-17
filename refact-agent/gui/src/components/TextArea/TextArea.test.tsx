import React from "react";
import { describe, test, expect } from "vitest";
import { render } from "../../utils/test-utils";
import { TextArea, TextAreaProps } from ".";

const App: React.FC<Partial<TextAreaProps>> = (props) => {
  const [value, setValue] = React.useState(props.value ?? "");
  const defaultProps: TextAreaProps = {
    onChange: (e) => setValue(e.target.value),
    value,
    ...props,
  };
  return <TextArea {...defaultProps} />;
};

describe("TextArea", () => {
  test("inserting text before previous text", async () => {
    const testId = "textarea";
    const { user, ...app } = render(<App data-testid={testId} />);
    const textarea = app.getByTestId(testId) as HTMLTextAreaElement;
    await user.type(textarea, "\nworld");
    await user.type(textarea, "hello", {
      initialSelectionStart: 0,
      initialSelectionEnd: 0,
    });
    expect(textarea.textContent).toEqual("hello\nworld");
  });

  test("undo / redo", async () => {
    const testId = "textarea";
    const { user, ...app } = render(<App data-testid={testId} />);
    const textarea = app.getByTestId(testId) as HTMLTextAreaElement;
    await user.type(textarea, "hello world");
    expect(textarea.textContent).toEqual("hello world");
    await user.keyboard("{Control>}{z}{/Control}");
    await user.keyboard("{Control>}{z}{/Control}");
    await user.keyboard("{Control>}{z}{/Control}");
    await user.keyboard("{Control>}{z}{/Control}");
    await user.keyboard("{Control>}{z}{/Control}");
    await user.keyboard("{Control>}{z}{/Control}");
    expect(textarea.textContent).toEqual("hello");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    await user.keyboard("{Shift>}{Control>}{z}{/Control}{/Shift}");
    expect(textarea.textContent).toEqual("hello world");
  });
});
