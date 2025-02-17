import { describe, test, expect } from "vitest";
import { formatCellNumber } from "./formatTableCell";

describe("formatCellValue", () => {
  test.each<[number | string, string]>([
    ["0", "0"],
    ["10", "10"],
    ["100", "100"],
    ["1000", "1k"],
    ["10000", "10k"],
    ["564731", "564.73k"],
    ["100000", "100k"],
    ["1000000", "1M"],
    [0, "0"],
    [10, "10"],
    [100, "100"],
    [1000, "1k"],
    [10000, "10k"],
    [564731, "564.73k"],
    [100000, "100k"],
    [1000000, "1M"],
  ])("returns %s for %s", (cellValue, expected) => {
    expect(formatCellNumber(cellValue)).toEqual(expected);
  });
});
