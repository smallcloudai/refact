import { describe, test, expect } from "vitest";
import { replaceRange } from "./utils";

describe("replaceRange", () => {
  test("when given a range, and a replacement it should replcace the range", () => {
    const input = "@work";
    const expected = "@workspace ";
    const result = replaceRange(input, [0, 5], "@workspace ");
    expect(result).toEqual(expected);
  });

  test("some times the range is incorrect", () => {
    const input = "@work";
    const expected = "@workspace ";
    const result = replaceRange(input, [0, 4], "@workspace ");
    expect(result).toEqual(expected);
  });
});
