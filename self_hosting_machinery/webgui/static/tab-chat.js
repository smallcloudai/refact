export async function init() {
  const elem = document.querySelector("#chat");
  const iframe = document.createElement("iframe");
  iframe.src = "tab-chat.html";
  iframe.style.width = "100%";
  iframe.style.height = "100%";
  iframe.style.minHeight = "480px";

  elem.appendChild(iframe);
  // const req = await fetch("tab-chat.html")
  // const text = await req.text();
}

export function tab_switched_here() {
  // const elem = document.querySelector("#chat");
  // RefactChat(elem, "lsp");
}

export function tab_switched_away() {
  // const elem = document.querySelector("#chat")
  // elem.children =  []
}

export function tab_update_each_couple_of_seconds() {}
