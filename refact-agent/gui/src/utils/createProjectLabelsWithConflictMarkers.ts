import { formatProjectName } from "./formatProjectName";

export type ProjectLabelInfo = {
  path: string;
  label: string;
  fullPath: string;
  hasConflict: boolean;
};

/**
 * Creates project labels and marks conflicting ones for tooltip display.
 * @param projectPaths - Array of project paths
 * @param indexOfLastFolder - Number of folders to show from the end (default: 1)
 * @returns Array of ProjectLabelInfo objects with conflict markers
 */
export const createProjectLabelsWithConflictMarkers = (
  projectPaths: string[],
  indexOfLastFolder = 1,
): ProjectLabelInfo[] => {
  if (projectPaths.length === 0) {
    return [];
  }

  // First, get the initial formatted names
  const initialLabels = projectPaths.map((path) => ({
    path,
    fullPath: path,
    label: formatProjectName({
      projectPath: path,
      isMarkdown: false,
      indexOfLastFolder,
    }),
    hasConflict: false,
  }));

  // Find duplicates
  const labelCounts = new Map<string, ProjectLabelInfo[]>();
  initialLabels.forEach((item) => {
    if (!labelCounts.has(item.label)) {
      labelCounts.set(item.label, []);
    }
    const items = labelCounts.get(item.label);
    if (items) {
      items.push(item);
    }
  });

  // Process duplicates to mark conflicting labels
  const result: ProjectLabelInfo[] = [];

  for (const [, items] of labelCounts) {
    if (items.length === 1) {
      // No conflict, use original label
      result.push(items[0]);
    } else {
      // Handle conflicts by showing more parent directories
      const markedConflictingItems = markConflictingLabels(items);
      result.push(...markedConflictingItems);
    }
  }

  // Sort result to maintain original order
  return result.sort(
    (a, b) => projectPaths.indexOf(a.path) - projectPaths.indexOf(b.path),
  );
};

/**
 * Handles conflicts by simply marking them as having conflicts.
 */
function markConflictingLabels(
  conflictingItems: ProjectLabelInfo[],
): ProjectLabelInfo[] {
  return conflictingItems.map((item) => ({
    ...item,
    hasConflict: true,
  }));
}
