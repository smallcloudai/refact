import { ChatContextFile } from "../../services/refact";
import { FileInfo } from "../../features/Chat/activeFile";
import type { Checkboxes } from "./useCheckBoxes";

export function addCheckboxValuesToInput(
  input: string,
  checkboxes: Checkboxes,
  _vecdb: boolean,
) {
  // prompts go to start
  let result = input;

  if (
    checkboxes.selected_lines.checked &&
    checkboxes.selected_lines.hide !== true
  ) {
    result = `${checkboxes.selected_lines.value ?? ""}\n` + result;
  }

  if (checkboxes.file_upload.checked && checkboxes.file_upload.hide !== true) {
    result = `@file ${checkboxes.file_upload.value ?? ""}\n` + result;
  }

  if (!result.endsWith("\n")) {
    result += "\n";
  }

  return result;
}

export function activeFileToContextFile(fileInfo: FileInfo): ChatContextFile {
  const content = fileInfo.content ?? "";
  return {
    file_name: fileInfo.path,
    file_content: content,
    line1: fileInfo.line1 ?? 1,
    line2: fileInfo.line2 ?? (content.split("\n").length || 1),
    usefulness: fileInfo.usefulness,
  };
}
