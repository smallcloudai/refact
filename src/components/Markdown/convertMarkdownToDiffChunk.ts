import { DiffChunk } from "../../services/refact/types";

export function convertMarkdownToDiffChunk(markdown: string): DiffChunk {
  const lines = markdown.split("\n");
  let fileName = "";
  let fileAction: "edit" | "add" | "rename" | "remove" = "remove"; // Default to remove
  let line1 = 0;
  let line2 = 0;
  let linesRemove = "";
  let linesAdd = "";

  let originalFileName = "";
  let newFileName = "";

  lines.forEach((line) => {
    if (line.startsWith("--- ")) {
      originalFileName = line.substring(4).trim(); // Extract original file name
      fileName = originalFileName; // Set fileName to original initially
      fileAction = "remove"; // Action for the original file
    } else if (line.startsWith("+++ ")) {
      newFileName = line.substring(4).trim(); // Extract new file name
      fileName = newFileName; // Update fileName to new
      fileAction = originalFileName === newFileName ? "edit" : "add"; // Determine action
    } else if (line.startsWith("@@ ")) {
      const parts = line.match(/@@ -(\d+),\d+ \+(\d+),\d+ @@/);
      if (parts) {
        line1 = parseInt(parts[1], 10); // Starting line number for the original file
        line2 = parseInt(parts[2], 10); // Starting line number for the new file
      }
    } else if (line.startsWith("-")) {
      linesRemove += line.substring(1).trim() + "\n"; // Lines removed
    } else if (line.startsWith("+")) {
      linesAdd += line.substring(1).trim() + "\n"; // Lines added
    }
  });

  // If the original and new file names are different, it could be a rename
  if (originalFileName && newFileName && originalFileName !== newFileName) {
    fileAction = "rename";
  }

  return {
    file_name: fileName,
    file_action: fileAction,
    line1: line1,
    line2: line2,
    lines_remove: linesRemove.trim(),
    lines_add: linesAdd.trim(),
  };
}
