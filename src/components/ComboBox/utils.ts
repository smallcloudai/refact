import getCaretCoordinates from "textarea-caret";

export function getTriggerOffset(
  element: HTMLTextAreaElement,
  triggers: string[],
) {
  const { value, selectionStart } = element;
  for (let i = selectionStart; i >= 0; i--) {
    const char = value[i];
    if (char && triggers.includes(char)) {
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
  triggers: string[],
): AnchorRect {
  const offset = getTriggerOffset(element, triggers);
  const { left, top, height } = getCaretCoordinates(element, offset + 1);
  const { x, y } = element.getBoundingClientRect();
  return {
    x: left + x - element.scrollLeft,
    y: top + y - element.scrollTop,
    height,
  };
}

export function replaceRange(
  str: string,
  range: [number, number],
  replacement: string,
) {
  const sortedRange = [
    Math.min(range[0], range[1]),
    Math.max(range[0], range[1]),
  ];
  return str.slice(0, sortedRange[0]) + replacement + str.slice(sortedRange[1]);
}
