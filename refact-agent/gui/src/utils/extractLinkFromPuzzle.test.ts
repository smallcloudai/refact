import { describe, it, expect } from "vitest";
import { extractLinkFromPuzzle } from "./extractLinkFromPuzzle";

describe("extractLinkFromPuzzle", () => {
  it("should return null for empty input", () => {
    expect(extractLinkFromPuzzle("")).toBeNull();
    expect(extractLinkFromPuzzle(undefined as unknown as string)).toBeNull();
    expect(extractLinkFromPuzzle(null as unknown as string)).toBeNull();
  });

  it("should return null if input is too short after slicing first 2 characters", () => {
    expect(extractLinkFromPuzzle("ab")).toBeNull();
  });

  it("should return null if no valid colon is found", () => {
    expect(extractLinkFromPuzzle("abtest")).toBeNull();
  });

  it("should return null if no content after colon", () => {
    expect(extractLinkFromPuzzle("ab:")).toBeNull();
    expect(extractLinkFromPuzzle("abtest:")).toBeNull();
  });

  it("should handle absolute paths correctly (linux)", () => {
    const result = extractLinkFromPuzzle("ab EDITOR:/path/to/file");

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Open file",
      link_goto: "EDITOR:/path/to/file",
      link_tooltip: "🧩 Open file",
    });
  });

  it("should handle absolute paths correctly (windows)", () => {
    const result = extractLinkFromPuzzle(
      "ab EDITOR:\\\\?\\c:\\Users\\Desktop\\example-project\\file",
    );

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Open file",
      link_goto: "EDITOR:\\\\?\\c:\\Users\\Desktop\\example-project\\file",
      link_tooltip: "🧩 Open file",
    });
  });

  it("should handle integrations names correctly", () => {
    const result = extractLinkFromPuzzle("ab SETTINGS:test_command");

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Setup Test Command",
      link_goto: "SETTINGS:test_command",
      link_tooltip: "🧩 Setup Test Command",
    });
  });

  it("should ignore colons that are part of drive letters", () => {
    const result = extractLinkFromPuzzle("ab EDITOR:C:\\path\\file.txt");

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Open file.txt",
      link_goto: "EDITOR:C:\\path\\file.txt",
      link_tooltip: "🧩 Open file.txt",
    });
  });

  it("should handle complex paths with spaces and special characters (linux)", () => {
    const result = extractLinkFromPuzzle(
      "ab EDITOR:/path/with spaces/file.txt",
    );

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Open file.txt",
      link_goto: "EDITOR:/path/with spaces/file.txt",
      link_tooltip: "🧩 Open file.txt",
    });
  });
  it("should handle complex paths with spaces and special characters (windows)", () => {
    const result = extractLinkFromPuzzle(
      "ab EDITOR:\\\\?\\C:\\path\\with spaces\\file.txt",
    );

    expect(result).toEqual({
      link_action: "goto",
      link_text: "🧩 Open file.txt",
      link_goto: "EDITOR:\\\\?\\C:\\path\\with spaces\\file.txt",
      link_tooltip: "🧩 Open file.txt",
    });
  });
});
