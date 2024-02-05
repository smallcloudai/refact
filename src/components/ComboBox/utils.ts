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
  const start = element.value.substring(
    0,
    element.selectionStart - trigger.length,
  );
  const end = element.value.substring(element.selectionStart);
  const maybeNewLineAfterStart =
    start.length && !start.endsWith("\n") ? "\n" : "";
  return `${start}${maybeNewLineAfterStart}${command}${end}`;
}
