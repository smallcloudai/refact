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

function countMatchingLetters(str1: string, str2: string) {
  if (!str1 || !str2) return 0;
  let i = 0;
  while (str1[i] && str2[i] && str1[i] === str2[i]) {
    i++;
  }
  return i;
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

  const tail = str.slice(sortedRange[0]);
  const count = countMatchingLetters(tail, replacement);
  const maybeLargerEnd = sortedRange[0] + count;
  const endIndex = Math.max(sortedRange[1], maybeLargerEnd);

  return str.slice(0, sortedRange[0]) + replacement + str.slice(endIndex);
}
