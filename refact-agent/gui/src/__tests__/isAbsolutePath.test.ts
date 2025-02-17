import { describe, it, expect } from "vitest";
import { isAbsolutePath } from "../utils/isAbsolutePath";

describe("isAbsolutePath", () => {
  it("should return true for Windows absolute paths", () => {
    expect(
      isAbsolutePath(
        "\\\\?\\C:\\Users\\andre\\.config\\integrations.d\\cmdline_example.yaml",
      ),
    ).toBe(true);
    expect(isAbsolutePath("D:\\Folder\\Subfolder")).toBe(true);
  });

  it("should return true for Unix absolute paths", () => {
    expect(isAbsolutePath("/usr/local/bin")).toBe(true);
    expect(
      isAbsolutePath(
        "/Users/andre/.config/integrations.d/cmdline_example.yaml",
      ),
    ).toBe(true);
  });

  it("should return true for UNC paths", () => {
    expect(isAbsolutePath("\\\\Server\\Share")).toBe(true);
    expect(isAbsolutePath("//Server/Share")).toBe(true);
  });

  it("should return false for relative paths", () => {
    expect(isAbsolutePath("relative/path")).toBe(false);
    expect(isAbsolutePath("folder\\subfolder")).toBe(false);
    expect(isAbsolutePath("./relative")).toBe(false);
    expect(isAbsolutePath("../relative")).toBe(false);
  });

  it("should return false for empty string", () => {
    expect(isAbsolutePath("")).toBe(false);
  });
});
