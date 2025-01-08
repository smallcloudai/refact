import { ChatLink } from "../services/refact";
import { toPascalCase } from "./toPascalCase";

export function extractLinkFromPuzzle(inputString: string): ChatLink | null {
  if (!inputString) {
    return null;
  }

  const puzzleLinkGoto = inputString.slice(2);
  if (!puzzleLinkGoto) {
    return null;
  }

  const [_linkType, linkPayload] = puzzleLinkGoto.split(":");

  if (!linkPayload) return null;

  const linkLabel = `🧩 Setup ${toPascalCase(linkPayload)}`;

  return {
    link_action: "goto",
    link_text: linkLabel,
    link_goto: puzzleLinkGoto,
    link_tooltip: linkLabel,
  };
}
