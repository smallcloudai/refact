/**
 * Formats the project path to display only the last folder.
 * @param projectPath - The full path of the project.
 * @param isMarkdown (optional) - Rather project name should be formatted to be inserted in markdown.
 * @param indexOfLastFolder (optional) - Indicates which folder to extract from the path. (from right to left)
 * @returns The formatted project name.
 */
export const formatProjectName = ({
  projectPath,
  isMarkdown = true,
  indexOfLastFolder = 1,
}: {
  projectPath: string;
  isMarkdown?: boolean;
  indexOfLastFolder?: number;
}): string => {
  const shortenedProjectPath =
    projectPath.split(/[/\\]/)[
      projectPath.split(/[/\\]/).length - indexOfLastFolder
    ];
  if (isMarkdown) {
    return "```.../" + shortenedProjectPath + "/```";
  } else {
    return shortenedProjectPath;
  }
};
