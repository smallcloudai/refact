import React, { HTMLProps } from "react";
import { describe, test, expect } from "vitest";
import { render } from "../../utils/test-utils";
import { detectCommand, replaceValue } from "./utils";

const TextArea: React.FC<HTMLProps<HTMLTextAreaElement>> = (props) => {
  const [value, setValue] = React.useState("");
  return (
    <textarea
      {...props}
      data-testid="textarea"
      value={value}
      onChange={(event) => setValue(event.target.value)}
    />
  );
};

type DetectResult = ReturnType<typeof detectCommand>;
describe("detectCommand", () => {
  test("it should return null if there is no command", async () => {
    const { user, ...app } = render(<TextArea />);

    const textarea = app.getByTestId("textarea") as HTMLTextAreaElement;
    await user.type(textarea, "foo bar\baz");
    const expected = null;
    const result = detectCommand(
      app.getByTestId("textarea") as HTMLTextAreaElement,
    );
    expect(result).toBe(expected);
  });

  test("when a user types some text and then a command it should return the command and the text before it", async () => {
    const { user, ...app } = render(<TextArea />);
    const textarea = app.getByTestId("textarea") as HTMLTextAreaElement;
    await user.type(textarea, "foo bar @file");

    const expected: DetectResult = {
      command: "@file",
      beforeCommand: "foo bar ",
      startPosition: 8,
    };
    const result = detectCommand(textarea);
    expect(result).toEqual(expected);
  });

  test("it should detect a command between lines", async () => {
    const { user, ...app } = render(<TextArea />);
    const textarea = app.getByTestId("textarea") as HTMLTextAreaElement;
    await user.type(textarea, "f\n@b\nc");
    await user.keyboard("{ArrowLeft}{ArrowLeft}");
    const expected: DetectResult = {
      command: "@b",
      beforeCommand: "f\n",
      startPosition: 2,
    };
    const result = detectCommand(textarea);
    expect(result).toEqual(expected);
  });
});

type ReplaceValueResult = ReturnType<typeof replaceValue>;

describe("replaceValue", () => {
  test("it should return the textarea value with the completed command ", async () => {
    const { user, ...app } = render(<TextArea />);
    const textarea = app.getByTestId("textarea") as HTMLTextAreaElement;
    await user.type(textarea, "foo bar @f");
    const result = replaceValue(textarea, "@f", "@file", null);
    const expected: ReplaceValueResult = {
      value: "foo bar \n@file",
      endPosition: 14,
    };
    expect(result).toEqual(expected);
  });

  test("it should set the end poisition to the end  of the command", async () => {
    const { user, ...app } = render(<TextArea />);
    const textarea = app.getByTestId("textarea") as HTMLTextAreaElement;
    await user.type(
      textarea,
      "foo\n\nbar{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}@f",
    );
    const result = replaceValue(textarea, "@f", "@file", null);
    const expected: ReplaceValueResult = {
      value: "foo\n@file\nbar",
      endPosition: 9,
    };
    expect(result).toEqual(expected);
  });
});
