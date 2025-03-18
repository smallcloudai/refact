export function scrollToBottom(elem: HTMLElement) {
  elem.scrollTop = elem.scrollHeight - elem.clientHeight;
}

export function overflowing(element: HTMLDivElement | null) {
  if (element === null) return false;
  const { scrollHeight, clientHeight } = element;
  return scrollHeight > clientHeight;
}

export function atBottom(element: HTMLDivElement | null) {
  if (element === null) return true;
  const { scrollHeight, scrollTop, clientHeight } = element;
  return Math.abs(scrollHeight - (scrollTop + clientHeight)) <= 1;
}
