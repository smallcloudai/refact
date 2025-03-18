export function scrollToBottom(elem: HTMLElement) {
  elem.scrollTop = elem.scrollHeight - elem.clientHeight;
}
