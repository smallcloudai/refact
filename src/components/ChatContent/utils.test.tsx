import { describe, test, expect } from "vitest";
import { trimIndent } from "./utils";

describe("trim indent", () => {
  const spaces = "    ";
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
        spaces,

      "```spaces\nfunction foo() {\n" + spaces + "return 'bar'\n}\n",
    ],
  ];
  test.each(tests)("when given %s it should return %s", (input, expected) => {
    const result = trimIndent(input);
    expect(result).toBe(expected);
  });
});
