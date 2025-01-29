import { ChatLink } from "../services/refact";
import { formatPathName } from "./formatPathName";
import { isAbsolutePath } from "./isAbsolutePath";
import { toPascalCase } from "./toPascalCase";

export function extractLinkFromPuzzle(inputString: string): ChatLink | null {
  if (!inputString) {
    return null;
  }

  const puzzleLinkGoto = inputString.slice(2);
  if (!puzzleLinkGoto) {
    return null;
  }

  const colonIndex = puzzleLinkGoto.search(/(?<!^[A-Za-z]):/);
  if (colonIndex === -1) return null;

  const linkPayload = puzzleLinkGoto.slice(colonIndex + 1);
  if (!linkPayload) return null;

  const linkLabel = isAbsolutePath(linkPayload)
    ? `🧩 Open ${formatPathName(linkPayload)}`
    : `🧩 Setup ${toPascalCase(linkPayload)}`;

  return {
    link_action: "goto",
    link_text: linkLabel,
    link_goto: puzzleLinkGoto,
    link_tooltip: linkLabel,
  };
}
