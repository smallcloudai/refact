import { describe, test, expect } from "vitest";
import {
  trimIndentFromMarkdown,
  trimIndent,
  filename,
  parseOrElse,
  takeFromEndWhile,
  scanFoDuplicatesWith,
  partition,
  fenceBackTicks,
} from ".";

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

describe("parseOrElse", () => {
  const tests: {
    args: Parameters<typeof parseOrElse>;
    expected: unknown;
  }[] = [
    {
      args: ['{"foo": "bar"}', {}, undefined],
      expected: { foo: "bar" },
    },
    {
      args: ["error", [], undefined],
      expected: [],
    },
    {
      args: ['["foo"]', [], Array.isArray],
      expected: ["foo"],
    },
    {
      args: ["error", [], Array.isArray],
      expected: [],
    },
  ];

  test.each(tests)(
    "when given %s it should return %s",
    ({ args, expected }) => {
      const result = parseOrElse(...args);
      expect(result).toEqual(expected);
    },
  );
});

describe("takeFromEndWhile", () => {
  const tests = [
    [
      ["a", "a", "b", "a", "b", "b"],
      ["b", "b"],
    ],
    [["a", "b", "c", "d"], []],
    [
      ["a", "b", "c", "b", "b"],
      ["b", "c", "b", "b"],
    ],
  ];

  test.each(tests)("when given %s it should return %s", (input, expected) => {
    const result = takeFromEndWhile(
      input,
      (char) => char === "b" || char === "c",
    );
    expect(result).toEqual(expected);
  });
});

describe("scanForDuplicates", () => {
  const tests: [string[], boolean][] = [
    [["a", "b", "c", "d", "b", "e"], true],
    [["a", "b", "c", "d", "e"], false],
  ];

  test.each(tests)("when given %s it should return %b", (input, expected) => {
    const result = scanFoDuplicatesWith(input, (a, b) => a === b);
    expect(result).toEqual(expected);
  });
});

describe("partition", () => {
  const tests: [string[], (a: string) => boolean, string[][]][] = [
    [
      ["a", "a", "b", "b", "a", "b", "c"],
      (a: string) => a === "a",
      [
        ["b", "b", "b", "c"],
        ["a", "a", "a"],
      ],
    ],
  ];

  test.each(tests)(
    "when given the array %j and function `%s` it should return %s",
    (input, condition, expected) => {
      const result = partition(input, condition);
      expect(result).toEqual(expected);
    },
  );
});

describe("fencedBackTicks", () => {
  test("it should wrap triple backticks with quadruple backticks", () => {
    const input = "```python\nprint('hello')\n```";
    const expected = "````\n```python\nprint('hello')\n````\n```";
    const result = fenceBackTicks(input);
    expect(result).toEqual(expected);
  });
});
