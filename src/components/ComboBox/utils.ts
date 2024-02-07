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
) {
  console.log("\n### replaceValue ###\n");

  const start = element.value.substring(
    0,
    element.selectionStart - trigger.length,
  );
  const end = element.value.substring(element.selectionStart);
  const maybeNewLineAfterStart =
    start.length && !start.endsWith("\n") ? "\n" : "";
  const result = `${start}${maybeNewLineAfterStart}${command}${end}`;
  console.log({
    element,
    command,
    elementValue: element.value,
    selectionStart: element.selectionStart,
    trigger,
    triggerLength: trigger.length,
    start,
    maybeNewLineAfterStart,
    end,
    result,
  });
  return result;
}

export function detectCommand(element: HTMLTextAreaElement): string {
  const start = element.value.substring(0, element.selectionStart);
  if (start.length === 0) return "";
  const maybeCommandIndex = start.lastIndexOf("@");
  if (maybeCommandIndex < 0) return "";
  const maybeCommand = start.substring(maybeCommandIndex);
  return maybeCommand;
}
