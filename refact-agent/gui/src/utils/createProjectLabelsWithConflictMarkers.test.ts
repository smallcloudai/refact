import { describe, expect, it } from "vitest";
import { createProjectLabelsWithConflictMarkers } from "./createProjectLabelsWithConflictMarkers";

describe("createProjectLabelsWithConflictMarkers", () => {
  it("should return original labels when there are no conflicts", () => {
    const paths = [
      "/home/user/project1",
      "/home/user/project2",
      "/home/user/different-project",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths);

    expect(result).toHaveLength(3);
    expect(result[0].label).toBe("project1");
    expect(result[1].label).toBe("project2");
    expect(result[2].label).toBe("different-project");
    expect(result.every((item) => !item.hasConflict)).toBe(true);
  });

  it("should create unique labels when there are conflicts", () => {
    const paths = [
      "/workspace/projectA/frontend",
      "/workspace/projectB/frontend",
      "/workspace/projectC/backend",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths, 1); // Use indexOfLastFolder=1 to get last segment

    expect(result).toHaveLength(3);

    // Check that frontend projects have conflicts
    const frontendResults = result.filter((item) =>
      item.path.includes("frontend"),
    );
    expect(frontendResults).toHaveLength(2);
    expect(frontendResults.every((item) => item.hasConflict)).toBe(true);

    // Check that backend project has no conflict
    const backendResult = result.find((item) => item.path.includes("backend"));
    expect(backendResult?.hasConflict).toBe(false);
    expect(backendResult?.label).toBe("backend");
  });

  it("should handle deeply nested paths with conflicts", () => {
    const paths = [
      "/very/long/path/to/project/frontend",
      "/another/very/long/path/to/different/frontend",
      "/short/frontend",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths, 1); // Use indexOfLastFolder=1 to get last segment

    expect(result).toHaveLength(3);

    // All should have conflicts since they end with 'frontend'
    expect(result.every((item) => item.hasConflict)).toBe(true);
  });

  it("should preserve original order of paths", () => {
    const paths = ["/path/c/frontend", "/path/a/frontend", "/path/b/frontend"];

    const result = createProjectLabelsWithConflictMarkers(paths);

    expect(result[0].path).toBe("/path/c/frontend");
    expect(result[1].path).toBe("/path/a/frontend");
    expect(result[2].path).toBe("/path/b/frontend");
  });

  it("should handle empty array", () => {
    const result = createProjectLabelsWithConflictMarkers([]);
    expect(result).toHaveLength(0);
  });

  it("should handle single path", () => {
    const paths = ["/home/user/project"];
    const result = createProjectLabelsWithConflictMarkers(paths);

    expect(result).toHaveLength(1);
    expect(result[0].label).toBe("project");
    expect(result[0].hasConflict).toBe(false);
    expect(result[0].fullPath).toBe("/home/user/project");
  });

  it("should use custom indexOfLastFolder parameter", () => {
    const paths = [
      "/workspace/projectA/frontend",
      "/workspace/projectB/frontend",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths, 2);

    expect(result).toHaveLength(2);
    // With indexOfLastFolder=2, we get "projectA" vs "projectB" which are unique
    expect(result.every((item) => !item.hasConflict)).toBe(true);
    expect(result[0].label).not.toBe(result[1].label);
  });

  it("should handle Windows-style paths", () => {
    const paths = [
      "C:\\Users\\user\\projectA\\frontend",
      "C:\\Users\\user\\projectB\\frontend",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths, 1); // Use indexOfLastFolder=1 to get last segment

    expect(result).toHaveLength(2);
    expect(result.every((item) => item.hasConflict)).toBe(true);
  });

  it("should handle real-world scenario with Windows UNC paths", () => {
    const paths = [
      "\\\\?\\C:\\Users\\andre\\Desktop\\work\\refact.ai\\refact\\refact-agent\\engine",
      "\\\\?\\C:\\Users\\andre\\Desktop\\work\\frontend\\my-app\\engine",
    ];

    const result = createProjectLabelsWithConflictMarkers(paths, 1); // Get last folder

    expect(result).toHaveLength(2);
    // Both end with "engine", so should have conflicts
    expect(result.every((item) => item.hasConflict)).toBe(true);
  });
});
