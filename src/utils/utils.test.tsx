import { describe, test, expect } from "vitest";
import { trimIndentFromMarkdown, trimIndent, filename } from ".";

const spaces = "    ";
describe("trim indent from markdown", () => {
  const tests = [
    ["```\n\thello\n\t\tworld\n```", "```\nhello\n\tworld\n```"],
    [
      "```spaces\n" +
        spaces +
        "function foo() {\n" +
        spaces +
        spaces +
        "return 'bar'\n" +
        spaces +
        "}\n" +
        spaces +
        "```",

      "```spaces\nfunction foo() {\n" + spaces + "return 'bar'\n}\n```",
    ],
  ];
  test.each(tests)("when given %s it should return %s", (input, expected) => {
    const result = trimIndentFromMarkdown(input);
    expect(result).toBe(expected);
  });
});

describe("trim indent", () => {
  const tests = [
    ["", ""],
    [`${spaces}hello\n${spaces}${spaces}world`, `hello\n${spaces}world`],
  ];

  test.each(tests)("when given %s it should return %s", (input, expected) => {
    const result = trimIndent(input);
    expect(result).toBe(expected);
  });
});

describe("filename", () => {
  const tests = [
    ["/user/foo.txt", "foo.txt"],
    ["C:\\user\\bar.py", "bar.py"],
  ];

  test.each(tests)("when given %s is should return %s", (input, expected) =>
    expect(filename(input)).toEqual(expected),
  );
});
