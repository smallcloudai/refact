import getCaretCoordinates from "textarea-caret";

export function getTriggerOffset(
  element: HTMLTextAreaElement,
  trigger: string,
) {
  const { value, selectionStart } = element;
  for (let i = selectionStart; i >= 0; i--) {
    const char = value[i];
    if (char && trigger === char) {
      return i;
    }
  }
  return -1;
}

export type AnchorRect = {
  x: number;
  y: number;
  height: number;
};

export function getAnchorRect(
  element: HTMLTextAreaElement,
  trigger: string,
): AnchorRect {
  const offset = getTriggerOffset(element, trigger);
  const { left, top, height } = getCaretCoordinates(element, offset + 1);
  const { x, y } = element.getBoundingClientRect();
  return {
    x: left + x - element.scrollLeft,
    y: top + y - element.scrollTop,
    height,
  };
}

export function replaceValue(
  element: HTMLTextAreaElement,
  trigger: string,
  command: string,
  startAt: number | null,
): { value: string; endPosition: number } {
  const maybeExistingCommand = detectCommand(element);
  const maybeEndOfCommand = maybeExistingCommand
    ? maybeExistingCommand.startPosition + maybeExistingCommand.command.length
    : null;
  const startPosition =
    maybeExistingCommand?.startPosition ?? startAt ?? element.selectionStart;

  const endPosition =
    maybeEndOfCommand ?? element.selectionStart + trigger.length;

  const start = element.value.substring(0, startPosition);
  const end = element.value.substring(endPosition);
  const result = `${start}${command}${end}`;

  const finalEndPosition = result.length - end.length;

  return {
    value: result,
    endPosition: finalEndPosition,
  };
}

export function detectCommand(element: HTMLTextAreaElement): {
  command: string;
  startPosition: number;
} | null {
  const start = element.value.substring(0, element.selectionStart);

  if (start.length === 0) return null;
  const maybeNewLine = Math.max(start.lastIndexOf("\n"), 0);
  const currentLine = start.substring(maybeNewLine > 0 ? maybeNewLine + 1 : 0);
  const maybeCommandIndex = currentLine.lastIndexOf("@");
  if (maybeCommandIndex < 0) return null;
  const maybeCommand = start.substring(maybeCommandIndex);
  return {
    command: maybeCommand,
    startPosition: maybeCommandIndex,
  };
}
